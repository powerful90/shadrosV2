// src/client_api.rs - Complete BOF Integration
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::listener::{ListenerConfig};
use crate::agent::{AgentConfig};
use crate::models::agent::Agent;
use crate::bof::integration::{BofTask, BofExecutionResult, BofMetadata, BofExecutionStatus};

use std::collections::HashMap;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommandResult {
    pub agent_id: String,
    pub task_id: String,
    pub command: String,
    pub output: String,
    pub success: bool,
    pub timestamp: u64,
    pub is_bof_result: bool,
    pub bof_metadata: Option<BofExecutionMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BofExecutionMetadata {
    pub bof_name: String,
    pub execution_time_ms: u64,
    pub output_size: usize,
    pub error_details: Option<String>,
    pub exit_code: i32,
}

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
    
    // ADD these new BOF messages:
    ExecuteBofByName { bof_name: String, args: String, target: String },
    GetBofLibrary,
    GetBofHelp { bof_name: String },
    SearchBofs { query: String },
    GetBofStats,
}

// ADD these new variants to your existing ServerMessage enum:
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

// ADD these new structs for BOF data:
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BofInfo {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub opsec_level: String,
    pub tactics: Vec<String>,
    pub techniques: Vec<String>,
    pub execution_time_estimate: u64,
    pub usage_examples: Vec<String>,
}

// ADD these new methods to your existing ClientApi impl block:
impl ClientApi {
    // ... existing methods ...

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

    // ... rest of your existing methods ...
}

// Enhanced ClientApi with comprehensive BOF support
pub struct ClientApi {
    server_addr: String,
    rx: Option<mpsc::Receiver<ServerMessage>>,
    tx: Option<mpsc::Sender<ClientMessage>>,
    connected: bool,
    authenticated: bool,
    
    // BOF-specific state
    bof_session_manager: BofSessionManager,
    last_bof_update: std::time::Instant,
}

impl ClientApi {
    pub fn new(server_addr: String) -> Self {
        ClientApi {
            server_addr,
            rx: None,
            tx: None,
            connected: false,
            authenticated: false,
            bof_session_manager: BofSessionManager::new(),
            last_bof_update: std::time::Instant::now(),
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
            let mut buffer = [0u8; 8192];
            
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

    // Existing methods (add_listener, start_listener, etc.)
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

    // Enhanced BOF methods
    pub async fn list_bofs(&self) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }

        println!("📚 CLIENT: Requesting BOF list from server...");

        if let Some(tx) = &self.tx {
            let msg = ClientMessage::ListBofs;
            tx.send(msg).await
                .map_err(|e| format!("Failed to send list BOFs message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    pub async fn get_bof_info(&self, bof_name: &str) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }

        println!("ℹ️ CLIENT: Requesting BOF info for '{}'...", bof_name);

        if let Some(tx) = &self.tx {
            let msg = ClientMessage::GetBofInfo {
                bof_name: bof_name.to_string(),
            };
            tx.send(msg).await
                .map_err(|e| format!("Failed to send get BOF info message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    pub async fn execute_bof_on_agent(&self, bof_name: &str, args: &str, agent_id: &str) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }

        println!("🚀 CLIENT: Executing BOF '{}' on agent '{}' with args: '{}'", 
            bof_name, agent_id, args);

        // Start BOF session tracking
        let session_id = format!("bof-{}-{}", agent_id, 
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs());
        
        self.bof_session_manager.start_session(
            session_id.clone(), 
            agent_id.to_string(), 
            bof_name.to_string()
        );

        if let Some(tx) = &self.tx {
            let msg = ClientMessage::ExecuteBofOnAgent {
                bof_name: bof_name.to_string(),
                args: args.to_string(),
                agent_id: agent_id.to_string(),
            };
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send execute BOF message: {}", e))?;
            
            println!("✅ CLIENT: BOF execution request sent (session: {})", session_id);
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    pub async fn execute_bof_local(&self, bof_name: &str, args: &str) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }

        println!("🧪 CLIENT: Executing BOF '{}' locally with args: '{}'", bof_name, args);

        if let Some(tx) = &self.tx {
            let msg = ClientMessage::ExecuteBofLocal {
                bof_name: bof_name.to_string(),
                args: args.to_string(),
            };
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send local BOF execution message: {}", e))?;
            
            println!("✅ CLIENT: Local BOF execution request sent");
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    pub async fn get_bof_tasks(&self) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }

        if let Some(tx) = &self.tx {
            let msg = ClientMessage::GetBofTasks;
            tx.send(msg).await
                .map_err(|e| format!("Failed to send get BOF tasks message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    pub async fn cleanup_bof_tasks(&self) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }

        println!("🧹 CLIENT: Requesting BOF task cleanup...");

        if let Some(tx) = &self.tx {
            let msg = ClientMessage::CleanupBofTasks;
            tx.send(msg).await
                .map_err(|e| format!("Failed to send cleanup BOF tasks message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    pub async fn load_bof_file(&self, file_path: &str) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }

        println!("📦 CLIENT: Requesting BOF file load: {}", file_path);

        if let Some(tx) = &self.tx {
            let msg = ClientMessage::LoadBofFile {
                file_path: file_path.to_string(),
            };
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send load BOF file message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    pub async fn get_bof_collections(&self) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }

        if let Some(tx) = &self.tx {
            let msg = ClientMessage::GetBofCollections;
            tx.send(msg).await
                .map_err(|e| format!("Failed to send get BOF collections message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    pub async fn reload_bof_library(&self) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }

        println!("🔄 CLIENT: Requesting BOF library reload...");

        if let Some(tx) = &self.tx {
            let msg = ClientMessage::ReloadBofLibrary;
            tx.send(msg).await
                .map_err(|e| format!("Failed to send reload BOF library message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    pub async fn cancel_bof_task(&self, task_id: &str) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }

        println!("🚫 CLIENT: Requesting BOF task cancellation: {}", task_id);

        if let Some(tx) = &self.tx {
            let msg = ClientMessage::CancelBofTask {
                task_id: task_id.to_string(),
            };
            
            tx.send(msg).await
                .map_err(|e| format!("Failed to send cancel BOF task message: {}", e))?;
            Ok(())
        } else {
            Err("Internal client error".into())
        }
    }

    // Enhanced command execution with BOF detection
    pub async fn execute_command_enhanced(&self, agent_id: &str, command: &str) -> Result<(), String> {
        if !self.authenticated {
            return Err("Not authenticated".into());
        }

        // Check if this is a BOF command
        if let Some((bof_name, args)) = parse_bof_command(command) {
            println!("🔥 CLIENT: Detected BOF command: {} with args: {}", bof_name, args);
            return self.execute_bof_on_agent(&bof_name, &args, agent_id).await;
        }

        // Fall back to regular command execution
        self.execute_command(agent_id, command).await
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

    // Enhanced BOF message handling
    pub fn handle_bof_server_message(&mut self, msg: &ServerMessage) -> Option<String> {
        match msg {
            ServerMessage::BofsListed { bofs } => {
                println!("📚 CLIENT: Received {} available BOFs", bofs.len());
                for bof in bofs {
                    println!("  • {} - {} ({})", bof.name, bof.description, bof.architecture);
                }
                Some(format!("Received {} BOFs", bofs.len()))
            },

            ServerMessage::BofInfo { metadata, help_text } => {
                println!("ℹ️ CLIENT: BOF Info for '{}':", metadata.name);
                println!("{}", help_text);
                Some(format!("BOF info for {}", metadata.name))
            },

            ServerMessage::BofTaskQueued { task_id, agent_id } => {
                println!("📋 CLIENT: BOF task queued: {} on agent {}", task_id, agent_id);
                
                // Update session tracking
                self.bof_session_manager.update_session_status(task_id, BofSessionStatus::Queued);
                
                Some(format!("BOF task {} queued on {}", task_id, agent_id))
            },

            ServerMessage::BofExecutionResult { task_id, result } => {
                println!("📊 CLIENT: BOF execution completed:");
                println!("  Task ID: {}", task_id);
                println!("  Exit Code: {}", result.exit_code);
                println!("  Execution Time: {}ms", result.execution_time_ms);
                println!("  Output ({} chars):", result.output.len());
                
                // Show output preview
                if result.output.len() > 500 {
                    println!("{}...", &result.output[..500]);
                    println!("  [Output truncated - {} total characters]", result.output.len());
                } else {
                    println!("{}", result.output);
                }
                
                if !result.error.is_empty() {
                    println!("  Errors:\n{}", result.error);
                }
                
                // Update session tracking
                self.bof_session_manager.complete_session(task_id, result.clone());
                
                Some(format!("BOF {} completed in {}ms", task_id, result.execution_time_ms))
            },

            ServerMessage::BofTasks { tasks } => {
                println!("📋 CLIENT: Received {} BOF tasks", tasks.len());
                for task in tasks {
                    println!("  • {} - {} on {} ({:?})", 
                        task.id, task.bof_name, task.target_agent, task.execution_status);
                }
                Some(format!("Received {} BOF tasks", tasks.len()))
            },

            ServerMessage::BofTasksCleanedUp { count } => {
                println!("🧹 CLIENT: Cleaned up {} BOF tasks", count);
                Some(format!("Cleaned up {} tasks", count))
            },

            ServerMessage::BofFileLoaded { bof_name } => {
                println!("📦 CLIENT: BOF file loaded: {}", bof_name);
                Some(format!("Loaded BOF: {}", bof_name))
            },

            ServerMessage::BofCollections { collections } => {
                println!("📦 CLIENT: Received {} BOF collections", collections.len());
                for (collection_name, bofs) in collections {
                    println!("  • {}: {} BOFs", collection_name, bofs.len());
                    for bof_name in bofs.iter().take(3) {
                        println!("    - {}", bof_name);
                    }
                    if bofs.len() > 3 {
                        println!("    ... and {} more", bofs.len() - 3);
                    }
                }
                Some(format!("Received {} collections", collections.len()))
            },

            ServerMessage::BofLibraryReloaded { count } => {
                println!("🔄 CLIENT: BOF library reloaded with {} BOFs", count);
                Some(format!("BOF library reloaded: {} BOFs", count))
            },

            ServerMessage::BofTaskCancelled { task_id } => {
                println!("🚫 CLIENT: BOF task cancelled: {}", task_id);
                
                // Update session tracking
                self.bof_session_manager.update_session_status(task_id, BofSessionStatus::Cancelled);
                
                Some(format!("BOF task {} cancelled", task_id))
            },

            ServerMessage::BofError { message } => {
                println!("❌ CLIENT: BOF error: {}", message);
                Some(format!("BOF error: {}", message))
            },

            _ => None,
        }
    }

    // Enhanced message handling for better integration
    pub fn handle_enhanced_server_message(&mut self, msg: &ServerMessage) -> Option<String> {
        // Handle BOF-specific messages first
        if let Some(bof_result) = self.handle_bof_server_message(msg) {
            return Some(bof_result);
        }

        // Enhanced handling for existing messages with BOF context
        match msg {
            ServerMessage::CommandResult { agent_id, task_id, command, output, success } => {
                // Check if this is a BOF command result
                if let Some((bof_name, args)) = parse_bof_command(command) {
                    println!("📊 CLIENT: BOF command result for '{}' on agent {}", bof_name, agent_id);
                    println!("  Success: {}", success);
                    println!("  Args: {}", args);
                    
                    let output_preview = if output.len() > 200 { 
                        format!("{}... ({} total chars)", &output[..200], output.len())
                    } else { 
                        output.clone() 
                    };
                    println!("  Output: {}", output_preview);
                    
                    // Parse results based on BOF type
                    match bof_name.as_str() {
                        "inlineExecute-Assembly" => {
                            if command.contains("Seatbelt.exe")