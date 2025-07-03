// src/models/messages.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Checkin,
    TaskAssignment,
    TaskResult,
    FileUpload,
    FileDownload,
    Screenshot,
    ProcessList,
    CommandExecution,
    BofExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub message_type: MessageType,
    pub agent_id: String,
    pub data: Vec<u8>,
    pub timestamp: u64,
}