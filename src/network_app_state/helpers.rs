// src/network_app_state/helpers.rs - Helper types and functions
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct CommandEntry {
    pub timestamp: String,
    pub agent_id: String,
    pub command: String,
    pub output: Option<String>,
    pub success: bool,
    pub task_id: String,
}

#[derive(Clone, Debug)]
pub struct BeaconSession {
    pub agent_id: String,
    pub hostname: String,
    pub username: String,
    pub command_history: Vec<CommandEntry>,
    pub current_directory: String,
}

pub fn format_time_ago(seconds: u64) -> String {
    match seconds {
        0..=59 => format!("{}s ago", seconds),
        60..=3599 => format!("{}m ago", seconds / 60),
        3600..=86399 => format!("{}h ago", seconds / 3600),
        _ => format!("{}d ago", seconds / 86400),
    }
}

pub fn format_timestamp(time: SystemTime) -> String {
    let duration = time.duration_since(UNIX_EPOCH).unwrap();
    let secs = duration.as_secs();
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}