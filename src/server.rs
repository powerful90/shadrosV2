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

use listener::{Listener, ListenerConfig};
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum ServerMessage {
    AuthResult { success: bool, message: String },
    ListenersUpdate { listeners: Vec<ListenerInfo> },
    AgentsUpdate { agents: Vec<Agent> },
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
    agents: Vec<Agent>,
    agent_generator: AgentGenerator,
    bof_executor: BofExecutor,
    clients: HashMap<String, mpsc::Sender<ServerMessage>>,
    password: String,
}

impl ServerState {
    fn new(password: String) -> Self {
        ServerState {
            listeners: Vec::new(),
            agents: Vec::new(),
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

    // Broadcast updates to all connected clients
    async fn broadcast_updates(&self) {
        let listeners_update = ServerMessage::ListenersUpdate { 
            listeners: self.get_listener_info() 
        };
        
        let agents_update = ServerMessage::AgentsUpdate { 
            agents: self.agents.clone() 
        };

        for (_, tx) in &self.clients {
            let _ = tx.send(listeners_update.clone()).await;
            let _ = tx.send(agents_update.clone()).await;
        }
    }
}

// Handle an individual client connection
async fn handle_client(
    mut stream: TcpStream, 
    addr: SocketAddr,
    state: Arc<Mutex<ServerState>>,
) {
    info!("New client connected: {}", addr);
    
    // Create channels for communication
    let (client_tx, mut client_rx) = mpsc::channel::<ServerMessage>(100);
    
    // We'll use a separate buffer for reading data
    let mut buffer = [0u8; 4096];
    let mut authenticated = false;
    
    // Add a temporary entry for this client
    {
        let mut state = state.lock().unwrap();
        state.clients.insert(addr.to_string(), client_tx.clone());
    }
    
    // Set up the client receiver task
    let client_stream = stream.clone();
    let mut client_writer = tokio::io::BufWriter::new(client_stream);
    
    let receiver_task = tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            let data = bincode::serialize(&msg).unwrap();
            let len = data.len() as u32;
            
            // Send message length first, then the message
            if client_writer.write_all(&len.to_be_bytes()).await.is_err() {
                break;
            }
            if client_writer.write_all(&data).await.is_err() {
                break;
            }
            if client_writer.flush().await.is_err() {
                break;
            }
        }
    });
    
    // Main client handling loop
    loop {
        // Read message length (4 bytes)
        let mut len_bytes = [0u8; 4];
        match stream.read_exact(&mut len_bytes).await {
            Ok(_) => {},
            Err(_) => break,
        }
        
        let len = u32::from_be_bytes(len_bytes) as usize;
        if len > buffer.len() {
            error!("Message too large from client {}", addr);
            break;
        }
        
        // Read the message
        match stream.read_exact(&mut buffer[0..len]).await {
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
                let success;
                let message;
                
                {
                    let state = state.lock().unwrap();
                    success = password == state.password;
                    message = if success { 
                        "Authentication successful".to_string() 
                    } else { 
                        "Invalid password".to_string() 
                    };
                }
                
                authenticated = success;
                let auth_result = ServerMessage::AuthResult { success, message };
                if let Err(e) = client_tx.send(auth_result).await {
                    error!("Failed to send auth result to {}: {}", addr, e);
                    break;
                }
                
                // If authenticated, send initial state
                if authenticated {
                    let state = state.lock().unwrap();
                    let listeners_update = ServerMessage::ListenersUpdate { 
                        listeners: state.get_listener_info() 
                    };
                    let agents_update = ServerMessage::AgentsUpdate { 
                        agents: state.agents.clone() 
                    };
                    
                    if let Err(e) = client_tx.send(listeners_update).await {
                        error!("Failed to send listeners update to {}: {}", addr, e);
                        break;
                    }
                    
                    if let Err(e) = client_tx.send(agents_update).await {
                        error!("Failed to send agents update to {}: {}", addr, e);
                        break;
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
                let result = {
                    let mut state = state.lock().unwrap();
                    state.add_listener(config)
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
                let state = state.lock().unwrap();
                state.broadcast_updates().await;
            },
            
            ClientMessage::StartListener { id } => {
                let result = {
                    let mut state = state.lock().unwrap();
                    state.start_listener(id)
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
                let state = state.lock().unwrap();
                state.broadcast_updates().await;
            },
            
            ClientMessage::StopListener { id } => {
                let result = {
                    let mut state = state.lock().unwrap();
                    state.stop_listener(id)
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
                let state = state.lock().unwrap();
                state.broadcast_updates().await;
            },
            
            ClientMessage::GetListeners => {
                let state = state.lock().unwrap();
                let listeners_update = ServerMessage::ListenersUpdate { 
                    listeners: state.get_listener_info() 
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
                let state = state.lock().unwrap();
                let agents_update = ServerMessage::AgentsUpdate { 
                    agents: state.agents.clone() 
                };
                
                if let Err(e) = client_tx.send(agents_update).await {
                    error!("Failed to send agents to {}: {}", addr, e);
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
    let mut state = state.lock().unwrap();
    state.clients.remove(&addr.to_string());
    
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