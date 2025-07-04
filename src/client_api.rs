// src/client_api.rs
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};

use crate::listener::{ListenerConfig};
use crate::agent::{AgentConfig};
use crate::models::agent::Agent;

// Message types for communication between client and server
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    AuthResult { success: bool, message: String },
    ListenersUpdate { listeners: Vec<ListenerInfo> },
    AgentsUpdate { agents: Vec<Agent> },
    Error { message: String },
    Success { message: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListenerInfo {
    pub id: usize,
    pub config: ListenerConfig,
    pub running: bool,
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
            let mut buffer = [0u8; 4096];
            
            loop {
                // Read message length
                let mut len_bytes = [0u8; 4];
                if read_half.read_exact(&mut len_bytes).await.is_err() {
                    break;
                }
                
                let len = u32::from_be_bytes(len_bytes) as usize;
                if len > buffer.len() {
                    eprintln!("Message too large from server");
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