// src/listener/mod.rs
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use serde::{Serialize, Deserialize};

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
        // If already running, do nothing
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        // Create a channel for communication
        let (tx, rx) = channel::<ListenerMessage>();
        
        // Store the sender
        {
            let mut tx_guard = self.tx.lock().unwrap();
            *tx_guard = Some(tx);
        }
        
        // Set running flag
        self.running.store(true, Ordering::SeqCst);
        
        let config = self.config.clone();
        let running = self.running.clone();
        
        // Spawn thread for the listener
        thread::spawn(move || {
            println!("Starting {:?} listener on {}:{}", config.listener_type, config.host, config.port);
            
            // Spawn another thread to actually handle the server
            // This ensures we can receive stop messages while the server is running
            let server_running = running.clone();
            let server_thread = thread::spawn(move || {
                // Simulate server work
                while server_running.load(Ordering::SeqCst) {
                    // In a real implementation, this would be your actual server logic
                    // For now, just sleep to avoid high CPU usage
                    thread::sleep(std::time::Duration::from_millis(100));
                }
            });
            
            // Wait for a stop message or for the running flag to be cleared
            loop {
                // Try to receive a message with a short timeout
                match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(ListenerMessage::Stop) => {
                        // Received stop message, exit loop
                        running.store(false, Ordering::SeqCst);
                        break;
                    },
                    Err(_) => {
                        // Timeout, check running flag
                        if !running.load(Ordering::SeqCst) {
                            break;
                        }
                    }
                }
            }
            
            // Wait for server thread to finish (with timeout)
            let _ = server_thread.join();
            
            println!("{:?} listener stopped on {}:{}", config.listener_type, config.host, config.port);
        });
        
        Ok(())
    }
    
    pub fn stop(&self) -> io::Result<()> {
        // If already stopped, do nothing
        if !self.running.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        // Set running flag to false
        self.running.store(false, Ordering::SeqCst);
        
        // Send stop message if possible
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
        // Make sure to stop the listener when it's dropped
        let _ = self.stop();
    }
}