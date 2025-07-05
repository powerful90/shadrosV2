// src/bof/integration.rs - Enhanced BOF integration (Fixed)
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

// Simplified BOF parser for command parsing
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

    /// Generate comprehensive help text for BOF
    pub fn generate_help_text(metadata: &crate::bof::BofMetadata) -> String {
        let mut help = format!("🎯 BOF: {}\n", metadata.name);
        help.push_str(&format!("📝 Description: {}\n", metadata.description));
        help.push_str(&format!("👤 Author: {} (v{})\n", metadata.author, metadata.version));
        help.push_str(&format!("🚨 OPSEC Level: {}\n", metadata.opsec_level));
        help.push_str(&format!("⏱️ Est. Execution Time: {}ms\n", metadata.execution_time_estimate));
        
        if !metadata.usage_examples.is_empty() {
            help.push_str("\n💡 Examples:\n");
            for example in &metadata.usage_examples {
                help.push_str(&format!("  • {}\n", example));
            }
        }
        
        if !metadata.tactics.is_empty() {
            help.push_str(&format!("\n🎯 MITRE ATT&CK Tactics: {}\n", metadata.tactics.join(", ")));
        }
        
        if !metadata.techniques.is_empty() {
            help.push_str(&format!("🔍 MITRE ATT&CK Techniques: {}\n", metadata.techniques.join(", ")));
        }
        
        help
    }
}

// Simplified BOF execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofExecutionResult {
    pub success: bool,
    pub output: String,
    pub error: String,
    pub execution_time_ms: u64,
    pub exit_code: i32,
}

// BOF collections for organized access
pub struct BofCollections;

impl BofCollections {
    pub fn red_team_bofs() -> Vec<&'static str> {
        vec![
            "mimikatz",
            "rubeus", 
            "sharphound",
            "seatbelt",
            "inlineExecute-Assembly"
        ]
    }

    pub fn reconnaissance_bofs() -> Vec<&'static str> {
        vec![
            "ps",
            "ls", 
            "whoami",
            "hostname",
            "ipconfig",
            "seatbelt",
            "sharphound"
        ]
    }

    pub fn post_exploitation_bofs() -> Vec<&'static str> {
        vec![
            "mimikatz",
            "rubeus",
            "inlineExecute-Assembly",
            "service_persist",
            "registry_persist"
        ]
    }

    pub fn stealth_bofs() -> Vec<&'static str> {
        vec![
            "ps",
            "ls",
            "whoami", 
            "hostname"
        ]
    }
}

// Re-export for compatibility
pub use BofParser as EnhancedBofParser;
pub use BofCollections as EnhancedBofCollections;