// src/models/agent.rs
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub hostname: String,
    pub username: String,
    pub os_version: String,
    pub architecture: String,
    pub ip_address: String,
    pub first_seen: u64,
    pub last_seen: u64,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub command: String,
    pub status: TaskStatus,
    pub result: Option<String>,
    pub created_at: u64,
    pub completed_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl Agent {
    pub fn new(id: String, hostname: String, username: String, os_version: String, 
               architecture: String, ip_address: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Agent {
            id,
            hostname,
            username,
            os_version,
            architecture,
            ip_address,
            first_seen: now,
            last_seen: now,
            tasks: Vec::new(),
        }
    }
    
    pub fn add_task(&mut self, command: String) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let task_id = format!("task-{}-{}", self.id, now);
        
        let task = Task {
            id: task_id.clone(),
            command,
            status: TaskStatus::Pending,
            result: None,
            created_at: now,
            completed_at: None,
        };
        
        self.tasks.push(task);
        task_id
    }
    
    pub fn update_task(&mut self, task_id: &str, status: TaskStatus, result: Option<String>) -> bool {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            // Clone status to avoid move issues
            let status_clone = status.clone();
            task.status = status;
            task.result = result;
            
            if status_clone == TaskStatus::Completed || status_clone == TaskStatus::Failed {
                task.completed_at = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                );
            }
            
            true
        } else {
            false
        }
    }
}