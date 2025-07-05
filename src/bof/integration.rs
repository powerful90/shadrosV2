// src/bof/integration.rs - Integration layer for BOF system with C2 framework
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

use crate::bof::{BofExecutor, BofContext, BofArgs};
use crate::listener::add_task_for_agent;
use crate::models::agent::Agent;

// Enhanced BOF task structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofTask {
    pub id: String,
    pub bof_name: String,
    pub bof_path: String,
    pub arguments: String,
    pub target_agent: String,
    pub created_at: u64,
    pub execution_status: BofExecutionStatus,
    pub result: Option<BofExecutionResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BofExecutionStatus {
    Queued,
    Sent,
    Executing,
    Completed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofExecutionResult {
    pub output: String,
    pub error: String,
    pub exit_code: i32,
    pub execution_time_ms: u64,
    pub completed_at: u64,
}

// BOF Management System
pub struct BofManager {
    executor: Arc<Mutex<BofExecutor>>,
    active_tasks: Arc<Mutex<HashMap<String, BofTask>>>,
    bof_library: Arc<Mutex<HashMap<String, BofMetadata>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofMetadata {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub architecture: String,
    pub file_path: String,
    pub file_size: u64,
    pub help_text: String,
    pub usage_examples: Vec<String>,
    pub parameters: Vec<BofParameter>,
    pub tactics: Vec<String>, // MITRE ATT&CK tactics
    pub techniques: Vec<String>, // MITRE ATT&CK techniques
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
}

impl BofManager {
    pub fn new() -> Self {
        BofManager {
            executor: Arc::new(Mutex::new(BofExecutor::new())),
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            bof_library: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Initialize with common BOFs
    pub fn initialize_default_bofs(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔧 Initializing default BOF library...");

        // Load built-in BOFs
        self.register_builtin_bofs()?;
        
        // Scan for BOF files in bofs/ directory
        self.scan_bof_directory("bofs/")?;

        println!("✅ BOF library initialized");
        Ok(())
    }

    // Register built-in BOFs
    fn register_builtin_bofs(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // InlineExecute-Assembly BOF
        let inline_execute_metadata = BofMetadata {
            name: "inlineExecute-Assembly".to_string(),
            description: "Execute .NET assemblies in-process without fork and run".to_string(),
            author: "anthemtotheego".to_string(),
            version: "1.0".to_string(),
            architecture: "x64".to_string(),
            file_path: "bofs/inlineExecute-Assembly.o".to_string(),
            file_size: 0,
            help_text: "Load CLR and execute .NET assembly in current process".to_string(),
            usage_examples: vec![
                "inlineExecute-Assembly /path/to/Seatbelt.exe".to_string(),
                "inlineExecute-Assembly /path/to/SharpHound.exe -c All".to_string(),
            ],
            parameters: vec![
                BofParameter {
                    name: "assembly".to_string(),
                    param_type: "file".to_string(),
                    description: "Path to .NET assembly".to_string(),
                    required: true,
                    default_value: None,
                },
                BofParameter {
                    name: "args".to_string(),
                    param_type: "string".to_string(),
                    description: "Assembly arguments".to_string(),
                    required: false,
                    default_value: None,
                },
                BofParameter {
                    name: "amsi".to_string(),
                    param_type: "bool".to_string(),
                    description: "Bypass AMSI".to_string(),
                    required: false,
                    default_value: Some("false".to_string()),
                },
            ],
            tactics: vec!["Execution".to_string(), "Defense Evasion".to_string()],
            techniques: vec!["T1055".to_string(), "T1218".to_string()],
        };

        // Process listing BOF
        let ps_metadata = BofMetadata {
            name: "ps".to_string(),
            description: "List running processes".to_string(),
            author: "C2 Framework".to_string(),
            version: "1.0".to_string(),
            architecture: "x64".to_string(),
            file_path: "bofs/ps.o".to_string(),
            file_size: 0,
            help_text: "Enumerate running processes with detailed information".to_string(),
            usage_examples: vec![
                "ps".to_string(),
                "ps -v".to_string(),
            ],
            parameters: vec![
                BofParameter {
                    name: "verbose".to_string(),
                    param_type: "bool".to_string(),
                    description: "Show detailed process information".to_string(),
                    required: false,
                    default_value: Some("false".to_string()),
                },
            ],
            tactics: vec!["Discovery".to_string()],
            techniques: vec!["T1057".to_string()],
        };

        // Directory listing BOF
        let ls_metadata = BofMetadata {
            name: "ls".to_string(),
            description: "List directory contents".to_string(),
            author: "C2 Framework".to_string(),
            version: "1.0".to_string(),
            architecture: "x64".to_string(),
            file_path: "bofs/ls.o".to_string(),
            file_size: 0,
            help_text: "List files and directories with detailed attributes".to_string(),
            usage_examples: vec![
                "ls".to_string(),
                "ls C:\\Windows\\System32".to_string(),
                "ls -la /etc".to_string(),
            ],
            parameters: vec![
                BofParameter {
                    name: "path".to_string(),
                    param_type: "string".to_string(),
                    description: "Directory path to list".to_string(),
                    required: false,
                    default_value: Some(".".to_string()),
                },
                BofParameter {
                    name: "recursive".to_string(),
                    param_type: "bool".to_string(),
                    description: "Recursive listing".to_string(),
                    required: false,
                    default_value: Some("false".to_string()),
                },
            ],
            tactics: vec!["Discovery".to_string()],
            techniques: vec!["T1083".to_string()],
        };

        let mut library = self.bof_library.lock().unwrap();
        library.insert("inlineExecute-Assembly".to_string(), inline_execute_metadata);
        library.insert("ps".to_string(), ps_metadata);
        library.insert("ls".to_string(), ls_metadata);

        println!("📚 Registered {} built-in BOFs", library.len());
        Ok(())
    }

    // Scan directory for BOF files
    fn scan_bof_directory(&mut self, dir_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        use std::fs;
        use std::path::Path;

        if !Path::new(dir_path).exists() {
            fs::create_dir_all(dir_path)?;
            println!("📁 Created BOF directory: {}", dir_path);
            return Ok(());
        }

        let mut found_bofs = 0;
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("o") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    let metadata = fs::metadata(&path)?;
                    
                    // Create basic metadata for discovered BOF
                    let bof_metadata = BofMetadata {
                        name: file_name.to_string(),
                        description: format!("BOF discovered in {}", dir_path),
                        author: "Unknown".to_string(),
                        version: "1.0".to_string(),
                        architecture: "x64".to_string(), // Assume x64 for now
                        file_path: path.to_string_lossy().to_string(),
                        file_size: metadata.len(),
                        help_text: "No help available".to_string(),
                        usage_examples: vec![],
                        parameters: vec![],
                        tactics: vec![],
                        techniques: vec![],
                    };

                    let mut library = self.bof_library.lock().unwrap();
                    library.insert(file_name.to_string(), bof_metadata);
                    found_bofs += 1;
                }
            }
        }

        println!("🔍 Discovered {} BOF files in {}", found_bofs, dir_path);
        Ok(())
    }

    // Execute BOF on target agent
    pub fn execute_bof_on_agent(&self, bof_name: &str, args: &str, agent_id: &str) -> Result<String, String> {
        // Check if BOF exists
        let bof_metadata = {
            let library = self.bof_library.lock().unwrap();
            library.get(bof_name).cloned()
        };

        let metadata = bof_metadata.ok_or_else(|| format!("BOF '{}' not found", bof_name))?;

        // Create BOF task
        let task_id = format!("bof-{}-{}", agent_id, 
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());

        let bof_task = BofTask {
            id: task_id.clone(),
            bof_name: bof_name.to_string(),
            bof_path: metadata.file_path.clone(),
            arguments: args.to_string(),
            target_agent: agent_id.to_string(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            execution_status: BofExecutionStatus::Queued,
            result: None,
        };

        // Store task
        {
            let mut tasks = self.active_tasks.lock().unwrap();
            tasks.insert(task_id.clone(), bof_task);
        }

        // Create command for agent
        let bof_command = format!("bof {} {}", bof_name, args);

        // Queue task for agent
        add_task_for_agent(agent_id, bof_command);

        println!("📋 BOF task '{}' queued for agent '{}'", task_id, agent_id);
        Ok(task_id)
    }

    // Execute BOF locally (for testing)
    pub fn execute_bof_local(&self, bof_name: &str, args: &str) -> Result<BofExecutionResult, String> {
        let bof_metadata = {
            let library = self.bof_library.lock().unwrap();
            library.get(bof_name).cloned()
        };

        let metadata = bof_metadata.ok_or_else(|| format!("BOF '{}' not found", bof_name))?;

        println!("🚀 Executing BOF '{}' locally", bof_name);

        let context = {
            let executor = self.executor.lock().unwrap();
            executor.execute(&metadata.file_path, args, "local")?
        };

        let result = BofExecutionResult {
            output: String::from_utf8_lossy(&context.beacon_output).to_string(),
            error: String::from_utf8_lossy(&context.beacon_error).to_string(),
            exit_code: context.exit_code,
            execution_time_ms: context.execution_time_ms,
            completed_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        };

        println!("✅ BOF '{}' completed in {}ms", bof_name, result.execution_time_ms);
        Ok(result)
    }

    // Handle BOF execution result from agent
    pub fn handle_bof_result(&self, task_id: &str, output: &str, success: bool) {
        let mut tasks = self.active_tasks.lock().unwrap();
        
        if let Some(task) = tasks.get_mut(task_id) {
            let result = BofExecutionResult {
                output: output.to_string(),
                error: if success { String::new() } else { "BOF execution failed".to_string() },
                exit_code: if success { 0 } else { 1 },
                execution_time_ms: 0, // Would be provided by agent
                completed_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            };

            task.result = Some(result);
            task.execution_status = if success { 
                BofExecutionStatus::Completed 
            } else { 
                BofExecutionStatus::Failed 
            };

            println!("📊 BOF task '{}' completed: {}", task_id, if success { "SUCCESS" } else { "FAILED" });
        }
    }

    // Get available BOFs
    pub fn list_available_bofs(&self) -> Vec<BofMetadata> {
        let library = self.bof_library.lock().unwrap();
        library.values().cloned().collect()
    }

    // Get BOF by name
    pub fn get_bof_metadata(&self, name: &str) -> Option<BofMetadata> {
        let library = self.bof_library.lock().unwrap();
        library.get(name).cloned()
    }

    // Get active tasks
    pub fn get_active_tasks(&self) -> Vec<BofTask> {
        let tasks = self.active_tasks.lock().unwrap();
        tasks.values().cloned().collect()
    }

    // Clear completed tasks
    pub fn cleanup_completed_tasks(&self) {
        let mut tasks = self.active_tasks.lock().unwrap();
        tasks.retain(|_, task| {
            !matches!(task.execution_status, BofExecutionStatus::Completed | BofExecutionStatus::Failed)
        });
    }

    // Create specialized BOF arguments for common use cases
    pub fn create_seatbelt_args(&self, checks: &[&str]) -> String {
        if checks.is_empty() {
            "All".to_string()
        } else {
            checks.join(" ")
        }
    }

    pub fn create_sharphound_args(&self, collection_method: &str, domain: Option<&str>) -> String {
        let mut args = vec!["-c", collection_method];
        
        if let Some(d) = domain {
            args.extend_from_slice(&["-d", d]);
        }
        
        args.join(" ")
    }

    pub fn create_rubeus_args(&self, action: &str, additional_args: &[&str]) -> String {
        let mut args = vec![action];
        args.extend_from_slice(additional_args);
        args.join(" ")
    }
}

// BOF Command Parser for agent integration
pub struct BofCommandParser;

impl BofCommandParser {
    // Parse BOF command from agent
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

    // Generate BOF help text
    pub fn generate_help_text(metadata: &BofMetadata) -> String {
        let mut help = format!("📋 BOF: {}\n", metadata.name);
        help.push_str(&format!("📝 Description: {}\n", metadata.description));
        help.push_str(&format!("👤 Author: {}\n", metadata.author));
        help.push_str(&format!("🏗️ Architecture: {}\n", metadata.architecture));
        
        if !metadata.parameters.is_empty() {
            help.push_str("\n📋 Parameters:\n");
            for param in &metadata.parameters {
                let required = if param.required { "[REQUIRED]" } else { "[OPTIONAL]" };
                help.push_str(&format!("  • {} ({}) {}: {}\n", 
                    param.name, param.param_type, required, param.description));
                
                if let Some(default) = &param.default_value {
                    help.push_str(&format!("    Default: {}\n", default));
                }
            }
        }
        
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

// Predefined BOF collections for different scenarios
pub struct BofCollections;

impl BofCollections {
    // Red Team BOF collection
    pub fn red_team_bofs() -> Vec<&'static str> {
        vec![
            "inlineExecute-Assembly",
            "ps",
            "ls",
            "whoami",
            "ipconfig",
            "netstat",
            "tasklist",
            "systeminfo",
            "reg_query",
            "wmi_query",
            "powershell",
            "mimikatz",
            "kerberoast",
            "bloodhound",
            "lateral_movement",
            "persistence",
            "privilege_escalation",
        ]
    }

    // Recon BOF collection
    pub fn reconnaissance_bofs() -> Vec<&'static str> {
        vec![
            "seatbelt",
            "sharphound",
            "adrecon",
            "powerup",
            "sherlock",
            "winpeas",
            "network_discovery",
            "domain_enum",
            "host_enum",
            "service_enum",
        ]
    }

    // Post-exploitation BOF collection
    pub fn post_exploitation_bofs() -> Vec<&'static str> {
        vec![
            "mimikatz",
            "rubeus",
            "sharpdpapi",
            "sharpchrome",
            "sharpcloud",
            "file_download",
            "file_upload",
            "screenshot",
            "keylogger",
            "clipboard_monitor",
        ]
    }

    // Stealth BOF collection (minimal detection risk)
    pub fn stealth_bofs() -> Vec<&'static str> {
        vec![
            "ps",
            "ls",
            "whoami",
            "pwd",
            "env",
            "netstat",
            "arp",
            "route",
            "hostname",
            "uptime",
        ]
    }
}

// BOF argument builders for complex BOFs
pub struct BofArgumentBuilder;

impl BofArgumentBuilder {
    // Build arguments for InlineExecute-Assembly
    pub fn inline_execute_assembly(
        assembly_path: &str,
        assembly_args: Option<&str>,
        amsi_bypass: bool,
        etw_bypass: bool,
        app_domain: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        let mut args = BofArgs::new();
        
        // App domain name
        args.add_string(app_domain.unwrap_or("DefaultDomain"));
        
        // Flags
        args.add_int(if amsi_bypass { 1 } else { 0 });
        args.add_int(if etw_bypass { 1 } else { 0 });
        args.add_int(0); // revert ETW
        args.add_int(0); // mailslot
        args.add_int(1); // entry point (Main with args)
        
        // Pipe/mailslot names
        args.add_string("DefaultSlot");
        args.add_string("DefaultPipe");
        
        // Assembly arguments
        args.add_string(assembly_args.unwrap_or(""));
        
        // Assembly binary data
        let assembly_data = std::fs::read(assembly_path)
            .map_err(|e| format!("Failed to read assembly: {}", e))?;
        
        args.add_int(assembly_data.len() as i32);
        args.add_binary(&assembly_data);
        
        Ok(args.finalize())
    }

    // Build arguments for Mimikatz
    pub fn mimikatz_command(command: &str) -> Vec<u8> {
        let mut args = BofArgs::new();
        args.add_string(command);
        args.finalize()
    }

    // Build arguments for file operations
    pub fn file_operation(source: &str, destination: Option<&str>) -> Vec<u8> {
        let mut args = BofArgs::new();
        args.add_string(source);
        
        if let Some(dest) = destination {
            args.add_string(dest);
        }
        
        args.finalize()
    }

    // Build arguments for network operations
    pub fn network_scan(target: &str, ports: &[u16]) -> Vec<u8> {
        let mut args = BofArgs::new();
        args.add_string(target);
        args.add_int(ports.len() as i32);
        
        for &port in ports {
            args.add_short(port as i16);
        }
        
        args.finalize()
    }
}

// Integration with existing agent task system
pub fn integrate_bof_with_agent_tasks() {
    // This would be called from your agent task processing system
    // When an agent receives a BOF task, it would:
    // 1. Parse the BOF command
    // 2. Load the BOF file
    // 3. Execute using the COFF loader
    // 4. Return results through the normal task result mechanism
}

// Example integration function for your server
pub fn handle_bof_execution_request(
    bof_manager: &BofManager,
    agent_id: &str,
    bof_name: &str,
    args: &str,
) -> Result<String, String> {
    println!("🎯 BOF execution request: {} on agent {}", bof_name, agent_id);
    
    // Validate BOF exists
    if bof_manager.get_bof_metadata(bof_name).is_none() {
        return Err(format!("BOF '{}' not found", bof_name));
    }
    
    // Queue BOF task
    let task_id = bof_manager.execute_bof_on_agent(bof_name, args, agent_id)?;
    
    println!("✅ BOF task queued: {}", task_id);
    Ok(task_id)
}

// Export main components
pub use BofManager as BofSystem;
pub use BofCommandParser as BofParser;