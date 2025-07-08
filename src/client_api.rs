// Enhanced client_api.rs with BOF support - COMPLETE FIX
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::listener::{ListenerConfig};
use crate::agent::{AgentConfig};
use crate::models::agent::Agent;

// ADDED: BOF-related structures
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BofMetadata {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub opsec_level: String,
    pub execution_time_estimate: u64,
    pub usage_examples: Vec<String>,
    pub tactics: Vec<String>,
    pub techniques: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BofExecutionResult {
    pub bof_name: String,
    pub agent_id: String,
    pub success: bool,
    pub output: String,
    pub error: String,
    pub execution_time_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BofFileInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub hash: String,
    pub uploaded_at: u64,
}

// Enhanced message types for communication between client and server
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    Authenticate { password: String },
    AddListener { config: ListenerConfig },
    StartListener { id: usize },
    StopListener { id: usize },
    GetListeners,
    GenerateAgent { config: AgentConfig },
    GetAgents,
    ExecuteBof { bof_path: String, args: String, target: String },
    ExecuteCommand { agent_id: String, command: String },
    // ADDED: BOF-related messages
    GetBofLibrary,
    SearchBofs { query: String },
    GetBofHelp { bof_name: String },
    ExecuteBofByName { bof_name: String, args: String, target: String },
    GetBofStats,
    ImportBof { file_path: String },
    ListBofFiles,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    AuthResult { success: bool, message: String },
    ListenersUpdate { listeners: Vec<ListenerInfo> },
    AgentsUpdate { agents: Vec<Agent> },
    CommandResult { agent_id: String, task_id: String, command: String, output: String, success: bool },
    Error { message: String },
    Success { message: String },
    // ADDED: BOF-related messages
    BofLibrary { bofs: Vec<BofMetadata> },
    BofStats { stats: HashMap<String, u64> },
    BofHelp { bof_name: String, help_text: String },
    BofSearchResults { results: Vec<BofMetadata> },
    BofExecutionComplete { result: BofExecutionResult },
    BofFilesList { files: Vec<BofFileInfo> },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListenerInfo {
    pub id: usize,
    pub config: ListenerConfig,
    pub running: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandResult {
    pub agent_id: String,
    pub task_id: String,
    pub command: String,
    pub output: String,
    pub success: bool,
    pub timestamp: u64,
}

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
    
    pub async fn connect(&mut self) -> Result<(), String> {
        let addr = self.server_addr.parse::<SocketAddr>()
            .map_err(|e| format!("Invalid server address: {}", e))?;
        
        let stream = TcpStream::connect(&addr).await
            .map_err(|e| format!("Failed to connect to server: {}", e))?;
        
        self.connected = true;
        
        // Set up channels
        let (tx, mut client_rx) = mpsc::channel::<ClientMessage>(100);
        let (server_tx, server_rx) = mpsc::channel::<ServerMessage>(100);
        
        self.rx = Some(server_rx);
        self.tx = Some(tx);
        
        // Split the stream into read and write halves
        let (mut read_half, mut write_half) = stream.into_split();
        
        // Spawn a task to receive messages from the server
        tokio::spawn(async move {
            let mut buffer = [0u8; 8192]; // Increased buffer size for command outputs
            
            loop {
                // Read message length
                let mut len_bytes = [0u8; 4];
                if read_half.read_exact(&mut len_bytes).await.is_err() {
                    break;
                }
                
                let len = u32::from_be_bytes(len_bytes) as usize;
                if len > buffer.len() {
                    eprintln!("Message too large from server: {} bytes", len);
                    break;
                }
                
                // Read message
                if read_half.read_exact(&mut buffer[0..len]).await.is_err() {
                    break;
                }
                
                // Deserialize message
                match bincode::deserialize::<ServerMessage>(&buffer[0..len]) {
                    Ok(msg) => {
                        if server_tx.send(msg).await.is_err() {
                            break;
                        }
                    },
                    Err(e) => {
                        eprintln!("Failed to deserialize server message: {}", e);
                        continue;
                    }
                }
            }
        });
        
        // Spawn a task to send messages to the server
        tokio::spawn(async move {
            while let Some(msg) = client_rx.recv().await {
                let data = bincode::serialize(&msg).unwrap();
                let len = data.len() as u32;
                
                // Send length followed by the message
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
                        return Ok(success);
                    },
                    Some(_) => return Err("Unexpected response from server".into()),
                    None => return Err("No response from server".into()),
                }
            }
        }
        
        Err("Internal client error".into())
    }
    
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
            
            Ok(Vec::new()) // Simplified for now
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
            
            Ok(Vec::new()) // Simplified for now
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
    
    pub async fn execute_bof(&self, bof_path: &str, args: &str, target: &str) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }
        
        if let Some(tx) = &self.tx {
            let msg = ClientMessage::ExecuteBof { 
                bof_path: bof_path.to_string(), 
                args: args.to_string(), 
                target: target.to_string() 
            };
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send execute BOF message: {}", e))?;
            
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }
    
    // ADDED: BOF-related methods
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
    
    pub async fn close(&mut self) {
        self.connected = false;
        self.authenticated = false;
        self.rx = None;
        self.tx = None;
    }
    
    pub fn is_connected(&self) -> bool {
        self.connected
    }
    
    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }
}