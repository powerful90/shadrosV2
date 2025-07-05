// src/bof/integration.rs - Enhanced BOF integration with complete C2 framework support
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH, Duration, Instant};
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::fs;

use crate::bof::{BofExecutor, BofContext, BofExecutionResult};
use crate::listener::add_task_for_agent;
use crate::models::agent::Agent;

// Enhanced BOF task structure with more comprehensive metadata
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
    pub priority: BofPriority,
    pub timeout_ms: u64,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BofExecutionStatus {
    Queued,
    Sent,
    Executing,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BofPriority {
    Low,
    Normal,
    High,
    Critical,
}

// Enhanced BOF metadata with operational details
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
    pub opsec_level: OpsecLevel,
    pub execution_time_estimate: u64,
    pub requirements: Vec<String>,
    pub detection_signatures: Vec<String>,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
    pub default_value: Option<String>,
    pub validation_regex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpsecLevel {
    Stealth,    // Minimal detection risk
    Careful,    // Low detection risk
    Standard,   // Normal detection risk  
    Aggressive, // High detection risk
    Loud,       // Very high detection risk
}

// Enhanced BOF Management System
pub struct BofManager {
    executor: Arc<Mutex<BofExecutor>>,
    active_tasks: Arc<Mutex<HashMap<String, BofTask>>>,
    bof_library: Arc<Mutex<HashMap<String, BofMetadata>>>,
    execution_history: Arc<Mutex<Vec<BofExecutionHistory>>>,
    bof_directories: Vec<String>,
    auto_discovery: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofExecutionHistory {
    pub task_id: String,
    pub bof_name: String,
    pub agent_id: String,
    pub executed_at: u64,
    pub duration_ms: u64,
    pub success: bool,
    pub output_size: usize,
    pub detection_events: Vec<String>,
}

impl BofManager {
    pub fn new() -> Self {
        BofManager {
            executor: Arc::new(Mutex::new(BofExecutor::new())),
            active_tasks: Arc::new(Mutex::new(HashMap::new())),
            bof_library: Arc::new(Mutex::new(HashMap::new())),
            execution_history: Arc::new(Mutex::new(Vec::new())),
            bof_directories: vec![
                "bofs/".to_string(),
                "bofs/recon/".to_string(),
                "bofs/persistence/".to_string(),
                "bofs/evasion/".to_string(),
                "bofs/collection/".to_string(),
                "bofs/exfiltration/".to_string(),
            ],
            auto_discovery: true,
        }
    }

    /// Initialize BOF system with comprehensive library setup
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Initializing BOF Management System...");

        // Create BOF directories if they don't exist
        self.create_bof_directories()?;
        
        // Load built-in BOFs
        self.register_builtin_bofs()?;
        
        // Scan for BOF files in all directories
        for directory in &self.bof_directories.clone() {
            if let Err(e) = self.scan_bof_directory(directory) {
                eprintln!("⚠️ Failed to scan BOF directory {}: {}", directory, e);
            }
        }

        // Load BOF manifests (metadata files)
        self.load_bof_manifests()?;

        // Validate BOF library
        self.validate_bof_library()?;

        let library_count = {
            let library = self.bof_library.lock().unwrap();
            library.len()
        };

        println!("✅ BOF Management System initialized with {} BOFs", library_count);
        Ok(())
    }

    /// Create BOF directory structure
    fn create_bof_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        for directory in &self.bof_directories {
            if !Path::new(directory).exists() {
                fs::create_dir_all(directory)?;
                println!("📁 Created BOF directory: {}", directory);
            }
        }
        Ok(())
    }

    /// Register comprehensive built-in BOF library
    fn register_builtin_bofs(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let bofs = vec![
            // Reconnaissance BOFs
            ("seatbelt", "Execute Seatbelt security enumeration", "GhostPack", 
             vec!["Discovery"], vec!["T1082", "T1016", "T1033"], OpsecLevel::Careful),
            ("sharphound", "BloodHound data collection", "BloodHound Team", 
             vec!["Discovery"], vec!["T1087", "T1069"], OpsecLevel::Standard),
            ("adrecon", "Active Directory reconnaissance", "Security Community", 
             vec!["Discovery"], vec!["T1087", "T1482"], OpsecLevel::Careful),
            
            // Process and System BOFs
            ("ps", "List running processes", "C2 Framework", 
             vec!["Discovery"], vec!["T1057"], OpsecLevel::Stealth),
            ("ls", "List directory contents", "C2 Framework", 
             vec!["Discovery"], vec!["T1083"], OpsecLevel::Stealth),
            ("whoami", "Get current user identity", "C2 Framework", 
             vec!["Discovery"], vec!["T1033"], OpsecLevel::Stealth),
            ("hostname", "Get system hostname", "C2 Framework", 
             vec!["Discovery"], vec!["T1082"], OpsecLevel::Stealth),
            ("ipconfig", "Network configuration", "C2 Framework", 
             vec!["Discovery"], vec!["T1016"], OpsecLevel::Stealth),
            
            // Credential Access BOFs
            ("mimikatz", "Credential extraction and manipulation", "Benjamin Delpy", 
             vec!["Credential Access"], vec!["T1003", "T1558"], OpsecLevel::Loud),
            ("rubeus", "Kerberos abuse toolkit", "GhostPack", 
             vec!["Credential Access"], vec!["T1558", "T1550"], OpsecLevel::Aggressive),
            ("sharpdpapi", "DPAPI credential extraction", "GhostPack", 
             vec!["Credential Access"], vec!["T1555"], OpsecLevel::Standard),
            
            // Execution BOFs
            ("inlineExecute-Assembly", "In-process .NET assembly execution", "anthemtotheego", 
             vec!["Execution", "Defense Evasion"], vec!["T1055", "T1218"], OpsecLevel::Standard),
            ("powershell", "PowerShell command execution", "C2 Framework", 
             vec!["Execution"], vec!["T1059.001"], OpsecLevel::Standard),
            ("cmd", "Command prompt execution", "C2 Framework", 
             vec!["Execution"], vec!["T1059.003"], OpsecLevel::Standard),
            
            // Persistence BOFs
            ("service_persist", "Service-based persistence", "C2 Framework", 
             vec!["Persistence"], vec!["T1543.003"], OpsecLevel::Aggressive),
            ("registry_persist", "Registry-based persistence", "C2 Framework", 
             vec!["Persistence"], vec!["T1547.001"], OpsecLevel::Standard),
            ("scheduled_task", "Scheduled task persistence", "C2 Framework", 
             vec!["Persistence"], vec!["T1053.005"], OpsecLevel::Standard),
            
            // Defense Evasion BOFs
            ("disable_defender", "Windows Defender manipulation", "C2 Framework", 
             vec!["Defense Evasion"], vec!["T1562.001"], OpsecLevel::Loud),
            ("amsi_bypass", "AMSI bypass techniques", "C2 Framework", 
             vec!["Defense Evasion"], vec!["T1562.001"], OpsecLevel::Aggressive),
            ("etw_bypass", "ETW bypass techniques", "C2 Framework", 
             vec!["Defense Evasion"], vec!["T1562.001"], OpsecLevel::Aggressive),
            
            // Collection BOFs
            ("screenshot", "Desktop screenshot capture", "C2 Framework", 
             vec!["Collection"], vec!["T1113"], OpsecLevel::Standard),
            ("clipboard", "Clipboard data collection", "C2 Framework", 
             vec!["Collection"], vec!["T1115"], OpsecLevel::Careful),
            ("keylogger", "Keystroke logging", "C2 Framework", 
             vec!["Collection"], vec!["T1056.001"], OpsecLevel::Aggressive),
            ("browser_data", "Browser data extraction", "C2 Framework", 
             vec!["Collection"], vec!["T1555.003"], OpsecLevel::Standard),
        ];

        let mut library = self.bof_library.lock().unwrap();
        
        for (name, description, author, tactics, techniques, opsec_level) in bofs {
            let metadata = BofMetadata {
                name: name.to_string(),
                description: description.to_string(),
                author: author.to_string(),
                version: "1.0".to_string(),
                architecture: "x64".to_string(),
                file_path: format!("bofs/{}.o", name),
                file_size: 0,
                help_text: format!("Execute {} BOF", name),
                usage_examples: vec![
                    format!("{}", name),
                    format!("{} --help", name),
                ],
                parameters: self.generate_default_parameters(name),
                tactics: tactics.into_iter().map(|s| s.to_string()).collect(),
                techniques: techniques.into_iter().map(|s| s.to_string()).collect(),
                opsec_level,
                execution_time_estimate: self.estimate_execution_time(name),
                requirements: self.generate_requirements(name),
                detection_signatures: self.generate_detection_signatures(name),
                last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            };
            
            library.insert(name.to_string(), metadata);
        }

        println!("📚 Registered {} built-in BOFs", library.len());
        Ok(())
    }

    /// Generate default parameters for common BOFs
    fn generate_default_parameters(&self, bof_name: &str) -> Vec<BofParameter> {
        match bof_name {
            "seatbelt" => vec![
                BofParameter {
                    name: "checks".to_string(),
                    param_type: "string".to_string(),
                    description: "Comma-separated list of checks to run".to_string(),
                    required: false,
                    default_value: Some("All".to_string()),
                    validation_regex: None,
                },
            ],
            "sharphound" => vec![
                BofParameter {
                    name: "collection".to_string(),
                    param_type: "string".to_string(),
                    description: "Collection method (All, Default, DCOnly, etc.)".to_string(),
                    required: false,
                    default_value: Some("All".to_string()),
                    validation_regex: None,
                },
                BofParameter {
                    name: "domain".to_string(),
                    param_type: "string".to_string(),
                    description: "Target domain".to_string(),
                    required: false,
                    default_value: None,
                    validation_regex: None,
                },
            ],
            "inlineExecute-Assembly" => vec![
                BofParameter {
                    name: "assembly".to_string(),
                    param_type: "file".to_string(),
                    description: "Path to .NET assembly".to_string(),
                    required: true,
                    default_value: None,
                    validation_regex: Some(r".*\.(exe|dll)$".to_string()),
                },
                BofParameter {
                    name: "args".to_string(),
                    param_type: "string".to_string(),
                    description: "Assembly arguments".to_string(),
                    required: false,
                    default_value: None,
                    validation_regex: None,
                },
                BofParameter {
                    name: "amsi".to_string(),
                    param_type: "bool".to_string(),
                    description: "Bypass AMSI".to_string(),
                    required: false,
                    default_value: Some("false".to_string()),
                    validation_regex: None,
                },
            ],
            _ => vec![],
        }
    }

    /// Estimate execution time for BOFs
    fn estimate_execution_time(&self, bof_name: &str) -> u64 {
        match bof_name {
            "whoami" | "hostname" | "pwd" => 100,
            "ps" | "ls" | "ipconfig" => 500,
            "seatbelt" => 30000,
            "sharphound" => 60000,
            "mimikatz" => 5000,
            "inlineExecute-Assembly" => 10000,
            _ => 2000,
        }
    }

    /// Generate requirements for BOFs
    fn generate_requirements(&self, bof_name: &str) -> Vec<String> {
        match bof_name {
            "mimikatz" => vec!["Admin privileges".to_string(), "SeDebugPrivilege".to_string()],
            "sharphound" => vec!["Domain user privileges".to_string()],
            "inlineExecute-Assembly" => vec![".NET Framework".to_string()],
            "service_persist" => vec!["Admin privileges".to_string()],
            _ => vec![],
        }
    }

    /// Generate detection signatures for BOFs
    fn generate_detection_signatures(&self, bof_name: &str) -> Vec<String> {
        match bof_name {
            "mimikatz" => vec![
                "LSASS memory access".to_string(),
                "Credential dumping".to_string(),
                "Mimikatz strings".to_string(),
            ],
            "sharphound" => vec![
                "LDAP enumeration".to_string(),
                "BloodHound collector".to_string(),
            ],
            "inlineExecute-Assembly" => vec![
                "CLR loading".to_string(),
                ".NET assembly execution".to_string(),
            ],
            _ => vec![],
        }
    }

    /// Scan directory for BOF files with enhanced metadata detection
    fn scan_bof_directory(&mut self, dir_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !Path::new(dir_path).exists() {
            return Ok(());
        }

        let mut found_bofs = 0;
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("o") {
                if let Some(file_name) = path.file_stem().and_then(|s| s.to_str()) {
                    let metadata = fs::metadata(&path)?;
                    
                    // Check if we already have metadata for this BOF
                    let existing_metadata = {
                        let library = self.bof_library.lock().unwrap();
                        library.get(file_name).cloned()
                    };
                    
                    if existing_metadata.is_none() {
                        // Create basic metadata for discovered BOF
                        let bof_metadata = BofMetadata {
                            name: file_name.to_string(),
                            description: format!("BOF discovered in {}", dir_path),
                            author: "Unknown".to_string(),
                            version: "1.0".to_string(),
                            architecture: "x64".to_string(),
                            file_path: path.to_string_lossy().to_string(),
                            file_size: metadata.len(),
                            help_text: "No help available".to_string(),
                            usage_examples: vec![],
                            parameters: vec![],
                            tactics: vec![],
                            techniques: vec![],
                            opsec_level: OpsecLevel::Standard,
                            execution_time_estimate: 2000,
                            requirements: vec![],
                            detection_signatures: vec![],
                            last_updated: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                        };

                        let mut library = self.bof_library.lock().unwrap();
                        library.insert(file_name.to_string(), bof_metadata);
                        found_bofs += 1;
                    }
                }
            }
        }

        if found_bofs > 0 {
            println!("🔍 Discovered {} new BOF files in {}", found_bofs, dir_path);
        }
        Ok(())
    }

    /// Load BOF manifests (JSON metadata files)
    fn load_bof_manifests(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for directory in &self.bof_directories.clone() {
            let manifest_path = format!("{}manifest.json", directory);
            if Path::new(&manifest_path).exists() {
                match self.load_manifest_file(&manifest_path) {
                    Ok(count) => println!("📄 Loaded {} BOF entries from {}", count, manifest_path),
                    Err(e) => eprintln!("⚠️ Failed to load manifest {}: {}", manifest_path, e),
                }
            }
        }
        Ok(())
    }

    /// Load individual manifest file
    fn load_manifest_file(&mut self, manifest_path: &str) -> Result<usize, Box<dyn std::error::Error>> {
        let manifest_content = fs::read_to_string(manifest_path)?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_content)?;
        
        let mut loaded_count = 0;
        
        if let Some(bofs) = manifest.get("bofs").and_then(|v| v.as_array()) {
            let mut library = self.bof_library.lock().unwrap();
            
            for bof_entry in bofs {
                if let Ok(metadata) = serde_json::from_value::<BofMetadata>(bof_entry.clone()) {
                    library.insert(metadata.name.clone(), metadata);
                    loaded_count += 1;
                }
            }
        }
        
        Ok(loaded_count)
    }

    /// Validate BOF library integrity
    fn validate_bof_library(&self) -> Result<(), Box<dyn std::error::Error>> {
        let library = self.bof_library.lock().unwrap();
        let mut issues = Vec::new();
        
        for (name, metadata) in library.iter() {
            // Check if BOF file exists
            if !Path::new(&metadata.file_path).exists() {
                issues.push(format!("BOF file not found: {} ({})", name, metadata.file_path));
            }
            
            // Validate metadata
            if metadata.name.is_empty() {
                issues.push(format!("Empty name for BOF: {}", name));
            }
            
            if metadata.description.is_empty() {
                issues.push(format!("Empty description for BOF: {}", name));
            }
        }
        
        if !issues.is_empty() {
            eprintln!("⚠️ BOF library validation issues:");
            for issue in issues {
                eprintln!("  - {}", issue);
            }
        } else {
            println!("✅ BOF library validation passed");
        }
        
        Ok(())
    }

    /// Execute BOF on target agent with comprehensive tracking
    pub fn execute_bof_on_agent(&self, bof_name: &str, args: &str, agent_id: &str) -> Result<String, String> {
        let bof_metadata = {
            let library = self.bof_library.lock().unwrap();
            library.get(bof_name).cloned()
        };

        let metadata = bof_metadata.ok_or_else(|| format!("BOF '{}' not found in library", bof_name))?;

        // Create unique task ID
        let task_id = format!("bof-{}-{}-{}", 
            agent_id, 
            bof_name,
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
        );

        // Determine priority based on BOF type
        let priority = match metadata.opsec_level {
            OpsecLevel::Stealth => BofPriority::Normal,
            OpsecLevel::Careful => BofPriority::Normal,
            OpsecLevel::Standard => BofPriority::Normal,
            OpsecLevel::Aggressive => BofPriority::High,
            OpsecLevel::Loud => BofPriority::Critical,
        };

        // Calculate timeout based on estimated execution time
        let timeout_ms = metadata.execution_time_estimate * 3; // 3x estimate as timeout

        let bof_task = BofTask {
            id: task_id.clone(),
            bof_name: bof_name.to_string(),
            bof_path: metadata.file_path.clone(),
            arguments: args.to_string(),
            target_agent: agent_id.to_string(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            execution_status: BofExecutionStatus::Queued,
            result: None,
            priority,
            timeout_ms,
            retry_count: 0,
        };

        // Store task
        {
            let mut tasks = self.active_tasks.lock().unwrap();
            tasks.insert(task_id.clone(), bof_task);
        }

        // Create BOF command for agent
        let bof_command = format!("bof {} {}", bof_name, args);

        // Queue task for agent using existing listener infrastructure
        add_task_for_agent(agent_id, bof_command);

        println!("📋 BOF task '{}' queued for agent '{}' (OPSEC: {:?})", 
            task_id, agent_id, metadata.opsec_level);
        
        Ok(task_id)
    }

    /// Execute BOF locally for testing and validation
    pub fn execute_bof_local(&self, bof_name: &str, args: &str) -> Result<BofExecutionResult, String> {
        let bof_metadata = {
            let library = self.bof_library.lock().unwrap();
            library.get(bof_name).cloned()
        };

        let metadata = bof_metadata.ok_or_else(|| format!("BOF '{}' not found", bof_name))?;

        println!("🚀 Executing BOF '{}' locally (OPSEC: {:?})", bof_name, metadata.opsec_level);

        // Execute using the enhanced BOF executor
        let context = {
            let mut executor = self.executor.lock().unwrap();
            executor.execute(&metadata.file_path, args, "local")?
        };

        // Record execution history
        let history_entry = BofExecutionHistory {
            task_id: format!("local-{}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
            bof_name: bof_name.to_string(),
            agent_id: "local".to_string(),
            executed_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            duration_ms: context.execution_time_ms,
            success: context.exit_code == 0,
            output_size: context.beacon_output.len(),
            detection_events: vec![], // Would be populated by detection logic
        };

        {
            let mut history = self.execution_history.lock().unwrap();
            history.push(history_entry);
            
            // Keep only last 1000 entries
            if history.len() > 1000 {
                history.drain(0..history.len() - 1000);
            }
        }

        let result = BofExecutionResult::from(context);
        println!("✅ BOF '{}' completed locally in {}ms", bof_name, result.execution_time_ms);
        
        Ok(result)
    }

    /// Handle BOF execution result from agent
    pub fn handle_bof_result(&self, task_id: &str, output: &str, success: bool) {
        let mut tasks = self.active_tasks.lock().unwrap();
        
        if let Some(task) = tasks.get_mut(task_id) {
            let result = BofExecutionResult {
                success,
                output: output.to_string(),
                error: if success { String::new() } else { "BOF execution failed".to_string() },
                execution_time_ms: 0, // Would be provided by agent
                exit_code: if success { 0 } else { 1 },
            };

            task.result = Some(result);
            task.execution_status = if success { 
                BofExecutionStatus::Completed 
            } else { 
                BofExecutionStatus::Failed 
            };

            // Record in execution history
            let history_entry = BofExecutionHistory {
                task_id: task_id.to_string(),
                bof_name: task.bof_name.clone(),
                agent_id: task.target_agent.clone(),
                executed_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
                duration_ms: 0, // Would be provided by agent
                success,
                output_size: output.len(),
                detection_events: vec![], // Would be populated by detection system
            };

            {
                let mut history = self.execution_history.lock().unwrap();
                history.push(history_entry);
            }

            println!("📊 BOF task '{}' completed: {} (Output: {} bytes)", 
                task_id, if success { "SUCCESS" } else { "FAILED" }, output.len());
        }
    }

    /// Get available BOFs with filtering and sorting
    pub fn list_available_bofs(&self, filter: Option<&str>, opsec_level: Option<OpsecLevel>) -> Vec<BofMetadata> {
        let library = self.bof_library.lock().unwrap();
        
        library.values()
            .filter(|bof| {
                // Apply name filter
                if let Some(filter) = filter {
                    if !bof.name.to_lowercase().contains(&filter.to_lowercase()) &&
                       !bof.description.to_lowercase().contains(&filter.to_lowercase()) {
                        return false;
                    }
                }
                
                // Apply OPSEC level filter
                if let Some(ref level) = opsec_level {
                    if std::mem::discriminant(&bof.opsec_level) != std::mem::discriminant(level) {
                        return false;
                    }
                }
                
                true
            })
            .cloned()
            .collect()
    }

    /// Get BOF by name with validation
    pub fn get_bof_metadata(&self, name: &str) -> Option<BofMetadata> {
        let library = self.bof_library.lock().unwrap();
        library.get(name).cloned()
    }

    /// Get active tasks with filtering
    pub fn get_active_tasks(&self, agent_filter: Option<&str>) -> Vec<BofTask> {
        let tasks = self.active_tasks.lock().unwrap();
        
        tasks.values()
            .filter(|task| {
                if let Some(agent_filter) = agent_filter {
                    task.target_agent.contains(agent_filter)
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }

    /// Get execution history with analytics
    pub fn get_execution_history(&self, limit: Option<usize>) -> Vec<BofExecutionHistory> {
        let history = self.execution_history.lock().unwrap();
        let entries = if let Some(limit) = limit {
            history.iter().rev().take(limit).cloned().collect()
        } else {
            history.clone()
        };
        entries
    }

    /// Get execution statistics
    pub fn get_execution_stats(&self) -> HashMap<String, u64> {
        let history = self.execution_history.lock().unwrap();
        let mut stats = HashMap::new();
        
        let total_executions = history.len() as u64;
        let successful_executions = history.iter().filter(|h| h.success).count() as u64;
        let failed_executions = total_executions - successful_executions;
        let avg_duration = if total_executions > 0 {
            history.iter().map(|h| h.duration_ms).sum::<u64>() / total_executions
        } else {
            0
        };
        
        stats.insert("total_executions".to_string(), total_executions);
        stats.insert("successful_executions".to_string(), successful_executions);
        stats.insert("failed_executions".to_string(), failed_executions);
        stats.insert("avg_duration_ms".to_string(), avg_duration);
        
        stats
    }

    /// Cleanup completed tasks (keep only recent ones)
    pub fn cleanup_completed_tasks(&self, keep_recent_hours: u64) {
        let mut tasks = self.active_tasks.lock().unwrap();
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - (keep_recent_hours * 3600);
        
        tasks.retain(|_, task| {
            match task.execution_status {
                BofExecutionStatus::Completed | BofExecutionStatus::Failed | BofExecutionStatus::Cancelled => {
                    task.created_at > cutoff_time
                },
                _ => true, // Keep pending/executing tasks
            }
        });
    }

    /// Export BOF library to manifest file
    pub fn export_library_manifest(&self, output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let library = self.bof_library.lock().unwrap();
        let bofs: Vec<&BofMetadata> = library.values().collect();
        
        let manifest = serde_json::json!({
            "version": "1.0",
            "exported_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            "total_bofs": bofs.len(),
            "bofs": bofs
        });
        
        fs::write(output_path, serde_json::to_string_pretty(&manifest)?)?;
        println!("📤 Exported BOF library manifest to {}", output_path);
        
        Ok(())
    }
}

/// BOF Command Parser with enhanced argument handling
pub struct BofCommandParser;

impl BofCommandParser {
    /// Parse BOF command with comprehensive argument extraction
    pub fn parse_bof_command(command: &str) -> Option<(String, String, HashMap<String, String>)> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        
        if parts.len() >= 2 && parts[0] == "bof" {
            let bof_name = parts[1].to_string();
            let mut args = String::new();
            let mut flags = HashMap::new();
            
            let mut i = 2;
            while i < parts.len() {
                if parts[i].starts_with("--") {
                    // Handle flags
                    let flag_name = parts[i].trim_start_matches("--");
                    if i + 1 < parts.len() && !parts[i + 1].starts_with("--") {
                        flags.insert(flag_name.to_string(), parts[i + 1].to_string());
                        i += 2;
                    } else {
                        flags.insert(flag_name.to_string(), "true".to_string());
                        i += 1;
                    }
                } else {
                    // Regular arguments
                    if !args.is_empty() {
                        args.push(' ');
                    }
                    args.push_str(parts[i]);
                    i += 1;
                }
            }
            
            Some((bof_name, args, flags))
        } else {
            None
        }
    }

    /// Generate comprehensive help text with OPSEC considerations
    pub fn generate_help_text(metadata: &BofMetadata) -> String {
        let mut help = format!("🎯 BOF: {}\n", metadata.name);
        help.push_str(&format!("📝 Description: {}\n", metadata.description));
        help.push_str(&format!("👤 Author: {} (v{})\n", metadata.author, metadata.version));
        help.push_str(&format!("🏗️ Architecture: {}\n", metadata.architecture));
        help.push_str(&format!("🚨 OPSEC Level: {:?}\n", metadata.opsec_level));
        help.push_str(&format!("⏱️ Est. Execution Time: {}ms\n", metadata.execution_time_estimate));
        
        if !metadata.requirements.is_empty() {
            help.push_str(&format!("🔒 Requirements: {}\n", metadata.requirements.join(", ")));
        }
        
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
                help.push_str(&format!("  • bof {}\n", example));
            }
        }
        
        if !metadata.tactics.is_empty() {
            help.push_str(&format!("\n🎯 MITRE ATT&CK Tactics: {}\n", metadata.tactics.join(", ")));
        }
        
        if !metadata.techniques.is_empty() {
            help.push_str(&format!("🔍 MITRE ATT&CK Techniques: {}\n", metadata.techniques.join(", ")));
        }
        
        if !metadata.detection_signatures.is_empty() {
            help.push_str(&format!("\n⚠️ Detection Signatures: {}\n", metadata.detection_signatures.join(", ")));
        }
        
        help
    }
}

// Export main components with enhanced functionality
pub use BofManager as EnhancedBofSystem;
pub use BofCommandParser as EnhancedBofParser;