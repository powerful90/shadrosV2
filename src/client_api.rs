// src/client_api.rs - Fixed with proper BOF structures (no more serde_json::Value)
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::listener::{ListenerConfig};
use crate::agent::{AgentConfig};
use crate::models::agent::Agent;

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

// Complete message types with BOF support (FIXED)
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
    
    // Enhanced BOF support (FIXED)
    ExecuteBofByName { bof_name: String, args: String, target: String },
    GetBofLibrary,
    GetBofHelp { bof_name: String },
    SearchBofs { query: String },
    GetBofStats,
    ImportBof { file_path: String },
    ListBofFiles,
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

// Enhanced ClientApi with comprehensive BOF support
pub struct ClientApi {
    server_addr: String,
    rx: Option<mpsc::Receiver<ServerMessage>>,
    tx: Option<mpsc::Sender<ClientMessage>>,
    connected: bool,
    authenticated: bool,
}

impl ClientApi {
    pub fn new(server_addr: String) -> Self {
        ClientApi {
            server_addr,
            rx: None,
            tx: None,
            connected: false,
            authenticated: false,
        }
    }
    
    // Existing connection methods
    pub async fn connect(&mut self) -> Result<(), String> {
        let addr = self.server_addr.parse::<SocketAddr>()
            .map_err(|e| format!("Invalid server address: {}", e))?;
        
        let stream = TcpStream::connect(&addr).await
            .map_err(|e| format!("Failed to connect to server: {}", e))?;
        
        self.connected = true;
        
        let (tx, mut client_rx) = mpsc::channel::<ClientMessage>(100);
        let (server_tx, server_rx) = mpsc::channel::<ServerMessage>(100);
        
        self.rx = Some(server_rx);
        self.tx = Some(tx);
        
        let (mut read_half, mut write_half) = stream.into_split();
        
        // Spawn receiver task
        tokio::spawn(async move {
            let mut buffer = [0u8; 16384]; // Increased buffer size
            
            loop {
                let mut len_bytes = [0u8; 4];
                if read_half.read_exact(&mut len_bytes).await.is_err() {
                    break;
                }
                
                let len = u32::from_be_bytes(len_bytes) as usize;
                if len > buffer.len() {
                    eprintln!("❌ Message too large from server: {} bytes", len);
                    break;
                }
                
                if read_half.read_exact(&mut buffer[0..len]).await.is_err() {
                    break;
                }
                
                match bincode::deserialize::<ServerMessage>(&buffer[0..len]) {
                    Ok(msg) => {
                        if server_tx.send(msg).await.is_err() {
                            break;
                        }
                    },
                    Err(e) => {
                        eprintln!("❌ Failed to deserialize server message: {}", e);
                        continue;
                    }
                }
            }
        });
        
        // Spawn sender task
        tokio::spawn(async move {
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
        });
        
        Ok(())
    }
    
    pub async fn authenticate(&mut self, password: &str) -> Result<bool, String> {
        if !self.connected {
            return Err("Not connected to server".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::Authenticate {
                password: password.to_string(),
            };
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send authentication message: {}", e))?;
            
            if let Some(rx) = &mut self.rx {
                match rx.recv().await {
                    Some(ServerMessage::AuthResult { success, message }) => {
                        self.authenticated = success;
                        if !success {
                            return Err(message);
                        }
                        println!("✅ CLIENT: Authentication successful");
                        return Ok(success);
                    },
                    Some(_) => return Err("Unexpected response from server".into()),
                    None => return Err("No response from server".into()),
                }
            }
        }
        
        Err("Internal client error".into())
    }

    // Existing methods
    pub async fn add_listener(&self, config: ListenerConfig) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::AddListener { config };
            tx.send(msg).await
                .map_err(|e| format!("Failed to send add listener message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }
    
    pub async fn start_listener(&self, id: usize) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::StartListener { id };
            tx.send(msg).await
                .map_err(|e| format!("Failed to send start listener message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }
    
    pub async fn stop_listener(&self, id: usize) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::StopListener { id };
            tx.send(msg).await
                .map_err(|e| format!("Failed to send stop listener message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }
    
    pub async fn get_listeners(&self) -> Result<Vec<ListenerInfo>, String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::GetListeners;
            tx.send(msg).await
                .map_err(|e| format!("Failed to send get listeners message: {}", e))?;
            Ok(Vec::new())
        } else {
            Err("Internal client error".into())
        }
    }
    
    pub async fn generate_agent(&self, config: AgentConfig) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::GenerateAgent { config };
            tx.send(msg).await
                .map_err(|e| format!("Failed to send generate agent message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }
    
    pub async fn get_agents(&self) -> Result<Vec<Agent>, String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::GetAgents;
            tx.send(msg).await
                .map_err(|e| format!("Failed to send get agents message: {}", e))?;
            Ok(Vec::new())
        } else {
            Err("Internal client error".into())
        }
    }
    
    pub async fn execute_command(&self, agent_id: &str, command: &str) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::ExecuteCommand { 
                agent_id: agent_id.to_string(), 
                command: command.to_string() 
            };
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send execute command message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    /// Execute BOF by name on target agent
    pub async fn execute_bof_by_name(&self, bof_name: &str, args: &str, target: &str) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::ExecuteBofByName { 
                bof_name: bof_name.to_string(),
                args: args.to_string(),
                target: target.to_string()
            };
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send execute BOF by name message: {}", e))?;
            
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    /// Get available BOFs from the server
    pub async fn get_bof_library(&self) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::GetBofLibrary;
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send get BOF library message: {}", e))?;
            
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    /// Get BOF execution statistics
    pub async fn get_bof_stats(&self) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::GetBofStats;
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send get BOF stats message: {}", e))?;
            
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    /// Get help for a specific BOF
    pub async fn get_bof_help(&self, bof_name: &str) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::GetBofHelp { 
                bof_name: bof_name.to_string() 
            };
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send get BOF help message: {}", e))?;
            
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    /// Search BOFs with query
    pub async fn search_bofs(&self, query: &str) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::SearchBofs { 
                query: query.to_string()
            };
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send search BOFs message: {}", e))?;
            
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    /// Import a BOF file
    pub async fn import_bof(&self, file_path: &str) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::ImportBof { 
                file_path: file_path.to_string()
            };
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send import BOF message: {}", e))?;
            
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    /// List available BOF files
    pub async fn list_bof_files(&self) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::ListBofFiles;
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send list BOF files message: {}", e))?;
            
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    // Message handling
    pub async fn receive_message(&mut self) -> Option<ServerMessage> {
        if let Some(rx) = &mut self.rx {
            rx.recv().await
        } else {
            None
        }
    }
    
    pub fn try_receive_message(&mut self) -> Option<ServerMessage> {
        if let Some(rx) = &mut self.rx {
            rx.try_recv().ok()
        } else {
            None
        }
    }
}

// Helper function to parse BOF commands
pub fn parse_bof_command(command: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    
    if parts.len() >= 2 && parts[0] == "bof" {
        let bof_name = parts[1].to_string();
        let args = if parts.len() > 2 {
            parts[2..].join(" ")
        } else {
            String::new()
        };
        
        Some((bof_name, args))
    } else {
        None
    }
}