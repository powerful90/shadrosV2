// Enhanced NetworkAppState with improved beacon console design
use eframe::egui::{self, Context, Ui, Color32, RichText, ScrollArea, Button, TextEdit, TextStyle, Frame, Margin, Rounding, Stroke};
// Removed unused imports: TableBuilder, Column
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use tokio::runtime::Runtime;

use crate::client_api::{ClientApi, ServerMessage, ListenerInfo};
use crate::listener::{ListenerConfig, ListenerType};
use crate::agent::AgentConfig;
use crate::models::agent::Agent;

use std::collections::HashMap;

#[derive(PartialEq)]
enum Tab {
    Dashboard,
    Listeners,
    Agents,
    Bof,
    Settings,
}

#[derive(Clone, Debug)]
pub struct CommandEntry {
    pub timestamp: String,
    pub agent_id: String,
    pub command: String,
    pub output: Option<String>,
    pub success: bool,
    pub task_id: String,
}

#[derive(Clone, Debug)]
pub struct BeaconSession {
    pub agent_id: String,
    pub hostname: String,
    pub username: String,
    pub command_history: Vec<CommandEntry>,
    pub current_directory: String,
}

pub struct NetworkAppState {
    client_api: Arc<Mutex<ClientApi>>,
    runtime: Runtime,
    
    // UI state
    current_tab: Tab,
    
    // Listener form state
    listener_type: ListenerType,
    listener_host: String,
    listener_port: String,
    
    // Agent form state
    agent_listener_url: String,
    agent_format: String,
    agent_architecture: String,
    agent_sleep_time: String,
    agent_jitter: String,
    agent_injection: String,
    agent_output_path: String,
    
    // BOF form state
    bof_file_path: String,
    bof_args: String,
    bof_target: String,
    
    // Enhanced command execution state
    command_input: String,
    selected_agent: Option<String>,
    beacon_sessions: HashMap<String, BeaconSession>,
    active_beacon: Option<String>,
    command_counter: u32,
    
    // Status messages
    status_message: String,
    status_time: Option<Instant>,
    
    // Data from server
    listeners: Vec<ListenerInfo>,
    agents: Vec<Agent>,
    
    // Last server poll time
    last_poll: Instant,
    
    // UI state for beacon console
    show_beacon_console: bool,
    console_scroll_to_bottom: bool,
    command_input_focus: bool,
}

impl NetworkAppState {
    pub fn new(client_api: Arc<Mutex<ClientApi>>) -> Self {
        NetworkAppState {
            client_api,
            runtime: Runtime::new().unwrap(),
            
            current_tab: Tab::Dashboard,
            
            listener_type: ListenerType::Http,
            listener_host: "0.0.0.0".to_string(),
            listener_port: "8080".to_string(),
            
            agent_listener_url: "http://0.0.0.0:8080".to_string(),
            agent_format: "exe".to_string(),
            agent_architecture: "x64".to_string(),
            agent_sleep_time: "60".to_string(),
            agent_jitter: "10".to_string(),
            agent_injection: "self".to_string(),
            agent_output_path: "agent.exe".to_string(),
            
            bof_file_path: "".to_string(),
            bof_args: "".to_string(),
            bof_target: "all".to_string(),
            
            command_input: String::new(),
            selected_agent: None,
            beacon_sessions: HashMap::new(),
            active_beacon: None,
            command_counter: 0,
            
            status_message: "".to_string(),
            status_time: None,
            
            listeners: Vec::new(),
            agents: Vec::new(),
            
            last_poll: Instant::now(),
            
            show_beacon_console: false,
            console_scroll_to_bottom: false,
            command_input_focus: false,
        }
    }
    
    fn set_status(&mut self, message: &str) {
        self.status_message = message.to_string();
        self.status_time = Some(Instant::now());
    }
    
    fn poll_server(&mut self) {
        let client_api_clone = self.client_api.clone();
        let client_opt = client_api_clone.try_lock().ok();
        
        if let Some(mut client) = client_opt {
            while let Some(msg) = client.try_receive_message() {
                // Debug log all received messages
                match &msg {
                    ServerMessage::CommandResult { agent_id, task_id, output, success, .. } => {
                        println!("📥 CLIENT: Received CommandResult");
                        println!("   Agent: {}", agent_id);
                        println!("   Task: {}", task_id);
                        println!("   Success: {}", success);
                        println!("   Output length: {}", output.len());
                    },
                    _ => {
                        println!("📥 CLIENT: Received message: {:?}", std::mem::discriminant(&msg));
                    }
                }
                
                match msg {
                    ServerMessage::ListenersUpdate { listeners } => {
                        self.listeners = listeners;
                    },
                    ServerMessage::AgentsUpdate { agents } => {
                        self.agents = agents.clone();
                        // Update beacon sessions for new agents
                        for agent in agents {
                            if !self.beacon_sessions.contains_key(&agent.id) {
                                let session = BeaconSession {
                                    agent_id: agent.id.clone(),
                                    hostname: agent.hostname.clone(),
                                    username: agent.username.clone(),
                                    command_history: Vec::new(),
                                    current_directory: "C:\\".to_string(),
                                };
                                self.beacon_sessions.insert(agent.id.clone(), session);
                            }
                        }
                    },
                    ServerMessage::CommandResult { agent_id, task_id, command: _, output, success } => {
                        println!("📥 CLIENT: Processing CommandResult for agent {}", agent_id);
                        
                        // Find and update the command entry
                        if let Some(session) = self.beacon_sessions.get_mut(&agent_id) {
                            println!("🔍 CLIENT: Found session for agent: {}", agent_id);
                            println!("🔍 CLIENT: Session has {} commands", session.command_history.len());
                            
                            let mut updated = false;
                            
                            // Strategy 1: Find by exact task_id match
                            for cmd_entry in session.command_history.iter_mut().rev() {
                                if cmd_entry.task_id == task_id {
                                    println!("✅ CLIENT: Found exact task_id match, updating");
                                    cmd_entry.output = Some(output.clone());
                                    cmd_entry.success = success;
                                    updated = true;
                                    break;
                                }
                            }
                            
                            // Strategy 2: Find most recent pending command if no exact match
                            if !updated {
                                println!("⚠️ CLIENT: No exact task_id match, looking for pending command");
                                for cmd_entry in session.command_history.iter_mut().rev() {
                                    if cmd_entry.output.is_none() {
                                        println!("✅ CLIENT: Found pending command, updating");
                                        cmd_entry.output = Some(output.clone());
                                        cmd_entry.success = success;
                                        cmd_entry.task_id = task_id.clone(); // Update task_id
                                        updated = true;
                                        break;
                                    }
                                }
                            }
                            
                            // Strategy 3: Add as new entry if still not found
                            if !updated {
                                println!("⚠️ CLIENT: No matching command found, creating new entry");
                                let cmd_entry = CommandEntry {
                                    timestamp: format_timestamp(SystemTime::now()),
                                    agent_id: agent_id.clone(),
                                    command: "completed".to_string(), // We don't have the original command
                                    output: Some(output.clone()),
                                    success,
                                    task_id: task_id.clone(),
                                };
                                session.command_history.push(cmd_entry);
                                updated = true;
                            }
                            
                            if updated {
                                println!("✅ CLIENT: Command result updated successfully");
                                self.console_scroll_to_bottom = true;
                            }
                            
                            // Debug: Print current command history
                            println!("🔍 CLIENT: Current command history for {}:", agent_id);
                            for (i, cmd) in session.command_history.iter().enumerate() {
                                println!("   {}: {} -> {}", i, cmd.command, 
                                    if cmd.output.is_some() { "HAS OUTPUT" } else { "PENDING" });
                            }
                        } else {
                            println!("❌ CLIENT: No session found for agent: {}", agent_id);
                            println!("❌ CLIENT: Available sessions: {:?}", 
                                self.beacon_sessions.keys().collect::<Vec<_>>());
                        }
                        
                        // Update status
                        if success {
                            self.set_status(&format!("✅ Command completed on {}", agent_id));
                        } else {
                            self.set_status(&format!("❌ Command failed on {}", agent_id));
                        }
                    },
                    ServerMessage::Success { message } => {
                        self.set_status(&message);
                    },
                    ServerMessage::Error { message } => {
                        self.set_status(&format!("❌ Error: {}", message));
                    },
                    _ => {}
                }
            }
        }
        
        // Poll server periodically for updates
        if self.last_poll.elapsed() > Duration::from_secs(2) {
            let client_api_clone = self.client_api.clone();
            
            self.runtime.spawn_blocking(move || {
                let runtime = Runtime::new().unwrap();
                runtime.block_on(async {
                    if let Ok(client) = client_api_clone.try_lock() {
                        let _ = client.get_listeners().await;
                        let _ = client.get_agents().await;
                    }
                });
            });
            
            self.last_poll = Instant::now();
        }
    }
    
    fn execute_command(&mut self, agent_id: &str, command: &str) {
        if command.trim().is_empty() {
            self.set_status("❌ Command cannot be empty");
            return;
        }
        
        self.command_counter += 1;
        let task_id = format!("task-{}-{}", agent_id, self.command_counter);
        
        // Add command to history immediately (output will be updated when result comes back)
        let timestamp = format_timestamp(SystemTime::now());
        let cmd_entry = CommandEntry {
            timestamp: timestamp.clone(),
            agent_id: agent_id.to_string(),
            command: command.to_string(),
            output: None, // Will be filled when real result comes back
            success: false,
            task_id: task_id.clone(),
        };
        
        if let Some(session) = self.beacon_sessions.get_mut(agent_id) {
            session.command_history.push(cmd_entry);
        }
        
        // Send command through client API
        let client_api_clone = self.client_api.clone();
        let agent_id_clone = agent_id.to_string();
        let command_clone = command.to_string();
        
        self.runtime.spawn_blocking(move || {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                if let Ok(client) = client_api_clone.try_lock() {
                    match client.execute_command(&agent_id_clone, &command_clone).await {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!("Failed to execute command: {}", e);
                        }
                    }
                }
            });
        });
        
        self.set_status(&format!("📤 Command '{}' sent to beacon", command));
        self.command_input.clear();
        self.console_scroll_to_bottom = true;
        self.command_input_focus = true; // Keep focus on input
    }
    
    fn open_beacon_console(&mut self, agent_id: &str) {
        self.active_beacon = Some(agent_id.to_string());
        self.show_beacon_console = true;
        self.selected_agent = Some(agent_id.to_string());
        self.command_input_focus = true;
    }
    
    // Enhanced beacon console rendering with professional design
    fn render_beacon_console(&mut self, ctx: &Context) {
        let mut open = true;
        
        if let Some(beacon_id) = &self.active_beacon.clone() {
            let session = self.beacon_sessions.get(beacon_id).cloned();
            
            if let Some(session) = session {
                egui::Window::new("")
                    .open(&mut open)
                    .resizable(true)
                    .default_size([1000.0, 700.0])
                    .frame(Frame::none()
                        .fill(Color32::from_rgb(15, 15, 15))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(70, 70, 70))))
                    .show(ctx, |ui| {
                        self.render_professional_beacon_console(ui, &session);
                    });
            }
        }
        
        if !open {
            self.show_beacon_console = false;
            self.active_beacon = None;
        }
    }
    
    fn render_professional_beacon_console(&mut self, ui: &mut Ui, session: &BeaconSession) {
        // Custom dark theme colors
        let bg_dark = Color32::from_rgb(15, 15, 15);
        let bg_medium = Color32::from_rgb(25, 25, 25);
        let bg_light = Color32::from_rgb(35, 35, 35);
        let accent_blue = Color32::from_rgb(100, 149, 237);
        let accent_green = Color32::from_rgb(152, 251, 152);
        let accent_red = Color32::from_rgb(255, 105, 97);
        let accent_yellow = Color32::from_rgb(255, 215, 0);
        let text_primary = Color32::from_rgb(220, 220, 220);
        let text_secondary = Color32::from_rgb(170, 170, 170);
        
        // Header with beacon info
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Beacon status indicator
                    ui.label(RichText::new("🔴").size(14.0));
                    
                    // Beacon info
                    ui.label(RichText::new("BEACON")
                        .color(accent_blue)
                        .size(14.0)
                        .strong());
                    
                    ui.separator();
                    
                    ui.label(RichText::new(&session.agent_id)
                        .color(text_primary)
                        .monospace()
                        .size(12.0));
                    
                    ui.separator();
                    
                    ui.label(RichText::new(format!("{}@{}", session.username, session.hostname))
                        .color(accent_green)
                        .size(12.0));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("❌ Close").color(accent_red).size(11.0)).clicked() {
                            self.show_beacon_console = false;
                            self.active_beacon = None;
                        }
                        
                        if ui.button(RichText::new("🗑 Clear").color(accent_yellow).size(11.0)).clicked() {
                            if let Some(mut_session) = self.beacon_sessions.get_mut(&session.agent_id) {
                                mut_session.command_history.clear();
                            }
                        }
                        
                        ui.label(RichText::new(format!("Commands: {}", session.command_history.len()))
                            .color(text_secondary)
                            .size(11.0));
                    });
                });
            });
        
        ui.separator();
        
        // Main console area with terminal-like appearance
        let available_height = ui.available_height() - 100.0; // Reserve space for input and buttons
        
        Frame::none()
            .fill(bg_dark)
            .inner_margin(Margin::same(8.0))
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .max_height(available_height)
                    .auto_shrink([false, false])
                    .stick_to_bottom(self.console_scroll_to_bottom)
                    .show(ui, |ui| {
                        ui.style_mut().override_text_style = Some(TextStyle::Monospace);
                        
                        // Welcome message if no commands yet
                        if session.command_history.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(RichText::new("🚀 Beacon Console Ready")
                                    .color(accent_blue)
                                    .size(16.0)
                                    .strong());
                                ui.label(RichText::new(format!("Connected to {}@{}", session.username, session.hostname))
                                    .color(text_secondary)
                                    .size(12.0));
                                ui.label(RichText::new("Type 'help' for available commands")
                                    .color(text_secondary)
                                    .size(11.0));
                                ui.add_space(20.0);
                            });
                        }
                        
                        // Command history with improved styling
                        for (index, cmd_entry) in session.command_history.iter().enumerate() {
                            // Add spacing between commands
                            if index > 0 {
                                ui.add_space(8.0);
                            }
                            
                            // Command prompt line with enhanced styling
                            Frame::none()
                                .fill(bg_medium)
                                .inner_margin(Margin::symmetric(8.0, 4.0))
                                .rounding(Rounding::same(4.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("❯")
                                            .color(accent_blue)
                                            .size(14.0)
                                            .strong());
                                        
                                        ui.label(RichText::new(&cmd_entry.command)
                                            .color(text_primary)
                                            .size(12.0)
                                            .monospace());
                                        
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(RichText::new(&cmd_entry.timestamp)
                                                .color(text_secondary)
                                                .size(10.0));
                                        });
                                    });
                                });
                            
                            // Command output with status-based coloring
                            if let Some(output) = &cmd_entry.output {
                                Frame::none()
                                    .fill(bg_dark)
                                    .inner_margin(Margin::symmetric(12.0, 6.0))
                                    .show(ui, |ui| {
                                        // Status indicator
                                        let (status_icon, status_color) = if cmd_entry.success {
                                            ("✅", accent_green)
                                        } else {
                                            ("❌", accent_red)
                                        };
                                        
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(status_icon).size(12.0));
                                            ui.label(RichText::new("Output:")
                                                .color(status_color)
                                                .size(11.0)
                                                .strong());
                                        });
                                        
                                        // Output text with syntax highlighting
                                        for line in output.lines() {
                                            if line.trim().is_empty() {
                                                ui.add_space(2.0);
                                                continue;
                                            }
                                            
                                            let line_color = if cmd_entry.success {
                                                // Highlight different types of output
                                                if line.contains("Error") || line.contains("error") || line.contains("ERROR") {
                                                    accent_red
                                                } else if line.contains("Success") || line.contains("success") || line.contains("OK") {
                                                    accent_green
                                                } else if line.contains("Warning") || line.contains("warning") {
                                                    accent_yellow
                                                } else {
                                                    text_primary
                                                }
                                            } else {
                                                accent_red
                                            };
                                            
                                            ui.label(RichText::new(line)
                                                .color(line_color)
                                                .size(11.0)
                                                .monospace());
                                        }
                                    });
                            } else {
                                // Pending command indicator
                                Frame::none()
                                    .fill(bg_light)
                                    .inner_margin(Margin::symmetric(12.0, 4.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("⏳")
                                                .color(accent_yellow)
                                                .size(12.0));
                                            ui.label(RichText::new("Executing command...")
                                                .color(accent_yellow)
                                                .size(11.0)
                                                .italics());
                                        });
                                    });
                            }
                        }
                        
                        if self.console_scroll_to_bottom {
                            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                            self.console_scroll_to_bottom = false;
                        }
                    });
            });
        
        ui.separator();
        
        // Command input area with enhanced styling
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("❯")
                        .color(accent_blue)
                        .size(16.0)
                        .strong());
                    
                    let command_input = TextEdit::singleline(&mut self.command_input)
                        .desired_width(ui.available_width() - 100.0)
                        .hint_text("Enter command...")
                        .font(TextStyle::Monospace)
                        .text_color(text_primary);
                    
                    let response = ui.add(command_input);
                    
                    // Auto-focus on input
                    if self.command_input_focus {
                        response.request_focus();
                        self.command_input_focus = false;
                    }
                    
                    let execute_button = ui.add(
                        Button::new(RichText::new("Execute").color(Color32::WHITE))
                            .fill(accent_blue)
                            .rounding(Rounding::same(4.0))
                    );
                    
                    if (execute_button.clicked() || 
                        (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))) 
                        && !self.command_input.trim().is_empty() {
                        let command = self.command_input.clone();
                        self.execute_command(&session.agent_id, &command);
                    }
                });
                
                ui.add_space(4.0);
                
                // Quick command buttons with professional styling
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Quick Commands:")
                        .color(text_secondary)
                        .size(11.0));
                    
                    let quick_commands = vec![
                        ("help", "help", accent_blue),
                        ("whoami", "whoami", accent_green),
                        ("pwd", "pwd", accent_green),
                        ("dir", "dir", accent_green),
                        ("ipconfig", "ipconfig /all", accent_yellow),
                        ("tasklist", "tasklist", accent_yellow),
                        ("netstat", "netstat -an", accent_yellow),
                        ("systeminfo", "systeminfo", accent_red),
                    ];
                    
                    for (label, cmd, color) in quick_commands {
                        if ui.add(
                            Button::new(RichText::new(label).color(Color32::WHITE).size(10.0))
                                .fill(color)
                                .rounding(Rounding::same(3.0))
                                .min_size([0.0, 20.0].into())
                        ).clicked() {
                            self.execute_command(&session.agent_id, cmd);
                        }
                    }
                });
            });
    }
    
    fn add_listener(&mut self) {
        let port = self.listener_port.parse::<u16>().unwrap_or(8080);
        
        let config = ListenerConfig {
            listener_type: self.listener_type.clone(),
            host: self.listener_host.clone(),
            port,
        };
        
        let client_api_clone = self.client_api.clone();
        
        self.runtime.spawn_blocking(move || {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                if let Ok(client) = client_api_clone.try_lock() {
                    match client.add_listener(config).await {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!("Failed to add listener: {}", e);
                        }
                    }
                }
            });
        });
        
        self.set_status("📡 Adding listener...");
    }
    
    fn start_listener(&mut self, id: usize) {
        let client_api_clone = self.client_api.clone();
        
        self.runtime.spawn_blocking(move || {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                if let Ok(client) = client_api_clone.try_lock() {
                    match client.start_listener(id).await {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!("Failed to start listener: {}", e);
                        }
                    }
                }
            });
        });
        
        self.set_status("🚀 Starting listener...");
    }
    
    fn stop_listener(&mut self, id: usize) {
        let client_api_clone = self.client_api.clone();
        
        self.runtime.spawn_blocking(move || {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                if let Ok(client) = client_api_clone.try_lock() {
                    match client.stop_listener(id).await {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!("Failed to stop listener: {}", e);
                        }
                    }
                }
            });
        });
        
        self.set_status("⏹ Stopping listener...");
    }
    
    fn generate_agent(&mut self) {
        let sleep_time = self.agent_sleep_time.parse::<u32>().unwrap_or(60);
        let jitter = self.agent_jitter.parse::<u8>().unwrap_or(10);
        
        let config = AgentConfig {
            listener_url: self.agent_listener_url.clone(),
            format: self.agent_format.clone(),
            architecture: self.agent_architecture.clone(),
            sleep_time,
            jitter,
            injection: self.agent_injection.clone(),
            output_path: self.agent_output_path.clone(),
        };
        
        let client_api_clone = self.client_api.clone();
        
        self.runtime.spawn_blocking(move || {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                if let Ok(client) = client_api_clone.try_lock() {
                    match client.generate_agent(config).await {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!("Failed to generate agent: {}", e);
                        }
                    }
                }
            });
        });
        
        self.set_status("⚙️ Generating agent...");
    }
    
    fn browse_agent_output(&mut self) {
        if let Some(path) = rfd::FileDialog::new().save_file() {
            if let Some(path_str) = path.to_str() {
                self.agent_output_path = path_str.to_string();
            }
        }
    }
    
    // Dashboard rendering with professional styling
    fn render_dashboard(&mut self, ui: &mut Ui) {
        let bg_medium = Color32::from_rgb(25, 25, 25);
        let accent_blue = Color32::from_rgb(100, 149, 237);
        let accent_green = Color32::from_rgb(152, 251, 152);
        let accent_red = Color32::from_rgb(255, 105, 97);
        let text_primary = Color32::from_rgb(220, 220, 220);
        let text_secondary = Color32::from_rgb(170, 170, 170);
        
        ui.heading(RichText::new("🎯 Dashboard - Live C2 Status").color(accent_blue).size(18.0));
        ui.separator();
        
        let listener_count = self.listeners.len();
        let agent_count = self.agents.len();
        let active_listeners = self.listeners.iter().filter(|l| l.running).count();
        
        // Statistics cards with enhanced styling
        ui.horizontal(|ui| {
            Frame::none()
                .fill(bg_medium)
                .inner_margin(Margin::same(15.0))
                .rounding(Rounding::same(8.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("📡 {} Listeners", listener_count))
                            .color(accent_blue)
                            .size(16.0)
                            .strong());
                        ui.label(RichText::new(format!("{} active, {} stopped", active_listeners, listener_count - active_listeners))
                            .color(text_secondary)
                            .size(12.0));
                        if ui.add(Button::new(RichText::new("Manage Listeners").color(Color32::WHITE))
                            .fill(accent_blue)).clicked() {
                            self.current_tab = Tab::Listeners;
                        }
                    });
                });
            
            ui.add_space(10.0);
            
            Frame::none()
                .fill(bg_medium)
                .inner_margin(Margin::same(15.0))
                .rounding(Rounding::same(8.0))
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("🔴 {} Live Beacons", agent_count))
                            .color(accent_red)
                            .size(16.0)
                            .strong());
                        ui.label(RichText::new("Real-time beacon connections")
                            .color(text_secondary)
                            .size(12.0));
                        if ui.add(Button::new(RichText::new("View Beacons").color(Color32::WHITE))
                            .fill(accent_red)).clicked() {
                            self.current_tab = Tab::Agents;
                        }
                    });
                });
        });
        
        ui.separator();
        
        // Recent beacon activity
        let agents_data: Vec<_> = self.agents.iter().map(|agent| {
            let time_ago = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() - agent.last_seen;
            
            (agent.id.clone(), agent.username.clone(), agent.hostname.clone(), time_ago)
        }).collect();
        
        if !agents_data.is_empty() {
            ui.label(RichText::new("🕒 Recent Beacon Activity").color(accent_green).size(16.0).strong());
            
            Frame::none()
                .fill(bg_medium)
                .inner_margin(Margin::same(10.0))
                .rounding(Rounding::same(6.0))
                .show(ui, |ui| {
                    ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        for (agent_id, username, hostname, time_ago) in agents_data {
                            let status_color = if time_ago < 120 { accent_green } else { Color32::YELLOW };
                            
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("●").color(status_color).size(12.0));
                                ui.label(RichText::new(format!("{}@{}", username, hostname))
                                    .color(text_primary).size(12.0));
                                ui.label(RichText::new(format!("({})", format_time_ago(time_ago)))
                                    .color(text_secondary).size(11.0));
                                if ui.add(Button::new(RichText::new("🔗 Interact").color(Color32::WHITE).size(10.0))
                                    .fill(accent_blue)).clicked() {
                                    self.open_beacon_console(&agent_id);
                                }
                            });
                            ui.add_space(3.0);
                        }
                    });
                });
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.label(RichText::new("🚀 No beacons connected yet")
                    .color(text_secondary).size(14.0));
                ui.label(RichText::new("Generate and run an agent to see it here")
                    .color(text_secondary).size(12.0));
                ui.add_space(30.0);
            });
        }
    }
    
    fn render_listeners(&mut self, ui: &mut Ui) {
        let bg_medium = Color32::from_rgb(25, 25, 25);
        let accent_blue = Color32::from_rgb(100, 149, 237);
        let accent_green = Color32::from_rgb(152, 251, 152);
        let accent_red = Color32::from_rgb(255, 105, 97);
        let text_primary = Color32::from_rgb(220, 220, 220);
        
        ui.heading(RichText::new("📡 Listeners").color(accent_blue).size(18.0));
        ui.separator();
        
        // Add new listener form
        ui.collapsing(RichText::new("➕ Add New Listener").color(accent_green), |ui| {
            Frame::none()
                .fill(bg_medium)
                .inner_margin(Margin::same(10.0))
                .rounding(Rounding::same(6.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Type:").color(text_primary));
                        egui::ComboBox::from_id_source("listener_type")
                            .selected_text(format!("{:?}", self.listener_type))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.listener_type, ListenerType::Http, "HTTP");
                                ui.selectable_value(&mut self.listener_type, ListenerType::Https, "HTTPS");
                                ui.selectable_value(&mut self.listener_type, ListenerType::Tcp, "TCP");
                                ui.selectable_value(&mut self.listener_type, ListenerType::Smb, "SMB");
                            });
                        
                        ui.label(RichText::new("Host:").color(text_primary));
                        ui.text_edit_singleline(&mut self.listener_host);
                        
                        ui.label(RichText::new("Port:").color(text_primary));
                        ui.text_edit_singleline(&mut self.listener_port);
                        
                        if ui.add(Button::new(RichText::new("🚀 Add Listener").color(Color32::WHITE))
                            .fill(accent_blue)).clicked() {
                            self.add_listener();
                        }
                    });
                });
        });
        
        ui.separator();
        
        // List existing listeners
        ui.label(RichText::new("Active Listeners").color(accent_green).size(16.0).strong());
        
        if self.listeners.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(RichText::new("📭 No listeners configured")
                    .color(Color32::GRAY).size(14.0));
                ui.add_space(20.0);
            });
        } else {
            // Collect listener data to avoid borrowing issues
            let listeners_data: Vec<_> = self.listeners.iter().enumerate().map(|(index, listener)| {
                (index, listener.running, listener.config.listener_type.clone(), 
                 listener.config.host.clone(), listener.config.port)
            }).collect();
            
            for (index, running, listener_type, host, port) in listeners_data {
                Frame::none()
                    .fill(bg_medium)
                    .inner_margin(Margin::same(8.0))
                    .rounding(Rounding::same(4.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let status_color = if running { accent_green } else { accent_red };
                            let status_text = if running { "🟢 Running" } else { "🔴 Stopped" };
                            
                            ui.label(RichText::new(status_text).color(status_color));
                            ui.label(RichText::new(format!("{:?}", listener_type)).color(text_primary));
                            ui.label(RichText::new(format!("{}:{}", host, port)).color(text_primary));
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add_enabled(running, 
                                    Button::new(RichText::new("Stop").color(Color32::WHITE))
                                        .fill(accent_red)).clicked() {
                                    self.stop_listener(index);
                                }
                                
                                if ui.add_enabled(!running,
                                    Button::new(RichText::new("Start").color(Color32::WHITE))
                                        .fill(accent_green)).clicked() {
                                    self.start_listener(index);
                                }
                            });
                        });
                    });
                ui.add_space(5.0);
            }
        }
    }
    
    fn render_live_agents(&mut self, ui: &mut Ui) {
        let bg_medium = Color32::from_rgb(25, 25, 25);
        let accent_blue = Color32::from_rgb(100, 149, 237);
        let accent_green = Color32::from_rgb(152, 251, 152);
        let accent_red = Color32::from_rgb(255, 105, 97);
        let accent_yellow = Color32::from_rgb(255, 215, 0);
        let text_primary = Color32::from_rgb(220, 220, 220);
        let text_secondary = Color32::from_rgb(170, 170, 170);
        
        ui.heading(RichText::new("🔴 Live Beacons").color(accent_red).size(18.0));
        ui.separator();
        
        // Agent generation form
        ui.collapsing(RichText::new("⚙️ Generate New Agent").color(accent_blue), |ui| {
            Frame::none()
                .fill(bg_medium)
                .inner_margin(Margin::same(10.0))
                .rounding(Rounding::same(6.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Listener URL:").color(text_primary));
                        ui.text_edit_singleline(&mut self.agent_listener_url);
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Format:").color(text_primary));
                        egui::ComboBox::from_id_source("agent_format")
                            .selected_text(&self.agent_format)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.agent_format, "exe".to_string(), "Windows EXE");
                                ui.selectable_value(&mut self.agent_format, "dll".to_string(), "Windows DLL");
                            });
                        
                        ui.label(RichText::new("Architecture:").color(text_primary));
                        egui::ComboBox::from_id_source("agent_architecture")
                            .selected_text(&self.agent_architecture)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.agent_architecture, "x64".to_string(), "x64");
                                ui.selectable_value(&mut self.agent_architecture, "x86".to_string(), "x86");
                            });
                    });
                    
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Sleep Time:").color(text_primary));
                        ui.text_edit_singleline(&mut self.agent_sleep_time);
                        
                        ui.label(RichText::new("Jitter:").color(text_primary));
                        ui.text_edit_singleline(&mut self.agent_jitter);
                        
                        ui.label(RichText::new("Output:").color(text_primary));
                        ui.text_edit_singleline(&mut self.agent_output_path);
                        
                        if ui.button("📁").clicked() {
                            self.browse_agent_output();
                        }
                        
                        if ui.add(Button::new(RichText::new("🚀 Generate").color(Color32::WHITE))
                            .fill(accent_blue)).clicked() {
                            self.generate_agent();
                        }
                    });
                });
        });
        
        ui.separator();
        
        // Live beacons list
        ui.label(RichText::new(format!("Connected Beacons ({})", self.agents.len()))
            .color(accent_red).size(16.0).strong());
        
        if self.agents.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.label(RichText::new("🚫 No beacons connected")
                    .color(text_secondary).size(14.0));
                ui.label(RichText::new("Generate and run an agent to see it appear here")
                    .color(text_secondary).size(12.0));
                ui.add_space(30.0);
            });
        } else {
            ScrollArea::vertical().show(ui, |ui| {
                for agent in &self.agents.clone() {
                    let time_ago = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() - agent.last_seen;
                    
                    let (status_text, status_color) = if time_ago < 120 {
                        ("🟢 Online", accent_green)
                    } else if time_ago < 300 {
                        ("🟡 Idle", accent_yellow)
                    } else {
                        ("🔴 Offline", accent_red)
                    };
                    
                    Frame::none()
                        .fill(bg_medium)
                        .inner_margin(Margin::same(8.0))
                        .rounding(Rounding::same(4.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(status_text).color(status_color));
                                ui.label(RichText::new(&agent.id).color(text_primary).monospace());
                                ui.label(RichText::new(format!("{}@{}", agent.username, agent.hostname)).color(accent_green));
                                ui.label(RichText::new(format_time_ago(time_ago)).color(text_secondary));
                                
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.add(Button::new(RichText::new("❌ Kill").color(Color32::WHITE))
                                        .fill(accent_red)).clicked() {
                                        self.set_status(&format!("🔪 Terminating beacon {}", agent.id));
                                    }
                                    
                                    if ui.add(Button::new(RichText::new("🔗 Interact").color(Color32::WHITE))
                                        .fill(accent_blue)).clicked() {
                                        self.open_beacon_console(&agent.id);
                                    }
                                });
                            });
                        });
                    ui.add_space(3.0);
                }
            });
        }
    }
    
    fn render_bof(&mut self, ui: &mut Ui) {
        let accent_blue = Color32::from_rgb(100, 149, 237);
        let text_secondary = Color32::from_rgb(170, 170, 170);
        
        ui.heading(RichText::new("⚡ BOF Execution").color(accent_blue).size(18.0));
        ui.separator();
        
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label(RichText::new("🚧 Under Development").color(Color32::YELLOW).size(16.0));
            ui.label(RichText::new("BOF (Beacon Object File) execution will be implemented soon")
                .color(text_secondary).size(12.0));
            ui.label(RichText::new("Use the Beacons tab to interact with connected beacons")
                .color(text_secondary).size(12.0));
            ui.add_space(50.0);
        });
    }
    
    fn render_settings(&mut self, ui: &mut Ui) {
        let accent_blue = Color32::from_rgb(100, 149, 237);
        let text_secondary = Color32::from_rgb(170, 170, 170);
        
        ui.heading(RichText::new("⚙️ Settings").color(accent_blue).size(18.0));
        ui.separator();
        
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label(RichText::new("🔧 Configuration Panel").color(Color32::YELLOW).size(16.0));
            ui.label(RichText::new("Settings panel coming soon!")
                .color(text_secondary).size(12.0));
            ui.add_space(50.0);
        });
    }
}


pub struct NetworkAppState {
    // ... your existing fields ...
    
    // ADD these new BOF-related fields:
    
    // BOF management
    bof_library: Vec<serde_json::Value>,
    bof_stats: HashMap<String, u64>,
    bof_search_results: Vec<serde_json::Value>,
    
    // BOF UI state
    bof_search_query: String,
    selected_bof_name: Option<String>,
    bof_args_input: String,
    bof_target_agent: Option<String>,
    show_bof_help: bool,
    bof_help_text: String,
    bof_help_name: String,
    
    // BOF execution state
    show_bof_library_tab: bool,
    show_bof_execution_tab: bool,
    show_bof_stats_tab: bool,
}

// ADD these to your NetworkAppState::new() method:
impl NetworkAppState {
    pub fn new(client_api: Arc<Mutex<ClientApi>>) -> Self {
        NetworkAppState {
            // ... your existing fields initialization ...
            
            // ADD these BOF field initializations:
            bof_library: Vec::new(),
            bof_stats: HashMap::new(),
            bof_search_results: Vec::new(),
            
            bof_search_query: String::new(),
            selected_bof_name: None,
            bof_args_input: String::new(),
            bof_target_agent: None,
            show_bof_help: false,
            bof_help_text: String::new(),
            bof_help_name: String::new(),
            
            show_bof_library_tab: true,
            show_bof_execution_tab: false,
            show_bof_stats_tab: false,
        }
    }

    // ADD this method to handle BOF-related server messages in your poll_server method:
    fn handle_bof_messages(&mut self, msg: &ServerMessage) {
        match msg {
            ServerMessage::BofLibrary { bofs } => {
                self.bof_library = bofs.clone();
                println!("📚 Received BOF library with {} BOFs", bofs.len());
            },
            ServerMessage::BofStats { stats } => {
                self.bof_stats = stats.clone();
                println!("📊 Received BOF statistics");
            },
            ServerMessage::BofHelp { bof_name, help_text } => {
                self.bof_help_name = bof_name.clone();
                self.bof_help_text = help_text.clone();
                self.show_bof_help = true;
                println!("📖 Received help for BOF: {}", bof_name);
            },
            ServerMessage::BofSearchResults { results } => {
                self.bof_search_results = results.clone();
                println!("🔍 Received {} BOF search results", results.len());
            },
            _ => {} // Handle other messages normally
        }
    }

    // REPLACE your existing render_bof method with this enhanced version:
    fn render_bof(&mut self, ui: &mut Ui) {
        let bg_dark = Color32::from_rgb(15, 15, 15);
        let bg_medium = Color32::from_rgb(25, 25, 25);
        let bg_light = Color32::from_rgb(35, 35, 35);
        let accent_blue = Color32::from_rgb(100, 149, 237);
        let accent_green = Color32::from_rgb(152, 251, 152);
        let accent_red = Color32::from_rgb(255, 105, 97);
        let accent_yellow = Color32::from_rgb(255, 215, 0);
        let accent_purple = Color32::from_rgb(186, 85, 211);
        let text_primary = Color32::from_rgb(220, 220, 220);
        let text_secondary = Color32::from_rgb(170, 170, 170);

        ui.heading(RichText::new("⚡ BOF Execution & Management").color(accent_purple).size(18.0));
        
        // BOF statistics and quick info
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(8.0))
            .rounding(Rounding::same(6.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("📚 {} BOFs Available", self.bof_library.len()))
                        .color(accent_blue).size(12.0));
                    ui.separator();
                    ui.label(RichText::new(format!("✅ {} Executions", 
                        self.bof_stats.get("total_executions").unwrap_or(&0)))
                        .color(accent_green).size(12.0));
                    ui.separator();
                    ui.label(RichText::new(format!("📦 {} Cached", 
                        self.bof_stats.get("cached_bofs").unwrap_or(&0)))
                        .color(accent_yellow).size(12.0));
                });
            });

        ui.separator();

        // Tab navigation for BOF management
        ui.horizontal(|ui| {
            if ui.selectable_label(self.show_bof_library_tab, 
                RichText::new("📚 BOF Library").color(if self.show_bof_library_tab { accent_green } else { text_primary })).clicked() {
                self.show_bof_library_tab = true;
                self.show_bof_execution_tab = false;
                self.show_bof_stats_tab = false;
                
                // Request BOF library from server
                let client_api_clone = self.client_api.clone();
                self.runtime.spawn_blocking(move || {
                    let runtime = Runtime::new().unwrap();
                    runtime.block_on(async {
                        if let Ok(client) = client_api_clone.try_lock() {
                            let _ = client.get_bof_library().await;
                        }
                    });
                });
            }
            
            if ui.selectable_label(self.show_bof_execution_tab, 
                RichText::new("🚀 Execute BOF").color(if self.show_bof_execution_tab { accent_green } else { text_primary })).clicked() {
                self.show_bof_library_tab = false;
                self.show_bof_execution_tab = true;
                self.show_bof_stats_tab = false;
            }
            
            if ui.selectable_label(self.show_bof_stats_tab, 
                RichText::new("📊 Statistics").color(if self.show_bof_stats_tab { accent_green } else { text_primary })).clicked() {
                self.show_bof_library_tab = false;
                self.show_bof_execution_tab = false;
                self.show_bof_stats_tab = true;
                
                // Request BOF stats from server
                let client_api_clone = self.client_api.clone();
                self.runtime.spawn_blocking(move || {
                    let runtime = Runtime::new().unwrap();
                    runtime.block_on(async {
                        if let Ok(client) = client_api_clone.try_lock() {
                            let _ = client.get_bof_stats().await;
                        }
                    });
                });
            }
        });

        ui.separator();

        // Render appropriate tab content
        if self.show_bof_library_tab {
            self.render_bof_library(ui, bg_medium, accent_blue, accent_green, accent_red, accent_yellow, text_primary, text_secondary);
        } else if self.show_bof_execution_tab {
            self.render_bof_execution(ui, bg_medium, accent_blue, accent_green, accent_red, accent_yellow, text_primary, text_secondary);
        } else if self.show_bof_stats_tab {
            self.render_bof_statistics(ui, bg_medium, accent_blue, accent_green, accent_red, accent_yellow, text_primary, text_secondary);
        }

        // BOF help window
        if self.show_bof_help {
            let mut open = true;
            egui::Window::new(format!("📖 BOF Help: {}", self.bof_help_name))
                .open(&mut open)
                .resizable(true)
                .default_size([600.0, 500.0])
                .show(ui.ctx(), |ui| {
                    ScrollArea::vertical().show(ui, |ui| {
                        ui.label(RichText::new(&self.bof_help_text)
                            .color(text_primary).size(12.0).monospace());
                    });
                });
            
            if !open {
                self.show_bof_help = false;
            }
        }
    }

    // ADD this new method for BOF library rendering:
    fn render_bof_library(&mut self, ui: &mut Ui, bg_medium: Color32, accent_blue: Color32, accent_green: Color32, 
                         accent_red: Color32, accent_yellow: Color32, text_primary: Color32, text_secondary: Color32) {
        
        // Search controls
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(8.0))
            .rounding(Rounding::same(4.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔍 Search BOFs:").color(text_primary));
                    ui.text_edit_singleline(&mut self.bof_search_query);
                    
                    if ui.add(Button::new(RichText::new("Search").color(Color32::WHITE))
                        .fill(accent_blue)).clicked() {
                        self.search_bofs();
                    }
                    
                    if ui.add(Button::new(RichText::new("🔄 Refresh").color(Color32::WHITE))
                        .fill(accent_green)).clicked() {
                        self.refresh_bof_library();
                    }
                });
            });

        ui.add_space(5.0);

        // BOF library list
        let bofs_to_display = if self.bof_search_query.is_empty() || self.bof_search_results.is_empty() {
            &self.bof_library
        } else {
            &self.bof_search_results
        };

        if bofs_to_display.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.label(RichText::new("📭 No BOFs available")
                    .color(text_secondary).size(14.0));
                ui.label(RichText::new("Click Refresh to load BOF library from server")
                    .color(text_secondary).size(12.0));
                ui.add_space(30.0);
            });
        } else {
            ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                for bof in bofs_to_display {
                    let name = bof.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                    let description = bof.get("description").and_then(|v| v.as_str()).unwrap_or("No description");
                    let author = bof.get("author").and_then(|v| v.as_str()).unwrap_or("Unknown");
                    let opsec_level = bof.get("opsec_level").and_then(|v| v.as_str()).unwrap_or("Standard");
                    
                    let is_selected = self.selected_bof_name.as_ref() == Some(&name.to_string());
                    
                    Frame::none()
                        .fill(if is_selected { bg_medium } else { Color32::from_rgb(20, 20, 20) })
                        .inner_margin(Margin::same(8.0))
                        .rounding(Rounding::same(4.0))
                        .stroke(if is_selected { 
                            Stroke::new(1.0, accent_blue) 
                        } else { 
                            Stroke::new(0.5, Color32::from_rgb(60, 60, 60)) 
                        })
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // BOF info
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(name).color(accent_blue).size(14.0).strong());
                                        
                                        // OPSEC level indicator
                                        let (opsec_icon, opsec_color) = match opsec_level {
                                            "Stealth" => ("🟢", accent_green),
                                            "Careful" => ("🟡", accent_yellow),
                                            "Standard" => ("🟠", accent_yellow),
                                            "Loud" => ("🔴", accent_red),
                                            _ => ("⚪", text_secondary),
                                        };
                                        ui.label(RichText::new(opsec_icon).color(opsec_color));
                                    });
                                    
                                    ui.label(RichText::new(description).color(text_secondary).size(11.0));
                                    ui.label(RichText::new(format!("by {}", author)).color(text_secondary).size(10.0));
                                });
                                
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.add(Button::new(RichText::new("🚀 Execute").color(Color32::WHITE))
                                        .fill(accent_green)).clicked() {
                                        self.selected_bof_name = Some(name.to_string());
                                        self.show_bof_execution_tab = true;
                                        self.show_bof_library_tab = false;
                                    }
                                    
                                    if ui.add(Button::new(RichText::new("ℹ️ Help").color(Color32::WHITE))
                                        .fill(accent_blue)).clicked() {
                                        self.get_bof_help(name);
                                    }
                                });
                            });
                        });

                    ui.add_space(3.0);
                }
            });
        }
    }

    // ADD this new method for BOF execution rendering:
    fn render_bof_execution(&mut self, ui: &mut Ui, bg_medium: Color32, accent_blue: Color32, accent_green: Color32, 
                           accent_red: Color32, accent_yellow: Color32, text_primary: Color32, text_secondary: Color32) {
        
        // BOF selection and arguments
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(10.0))
            .rounding(Rounding::same(6.0))
            .show(ui, |ui| {
                ui.label(RichText::new("🎯 BOF Execution Setup").color(accent_blue).size(16.0).strong());
                
                ui.add_space(10.0);
                
                // BOF selection
                ui.horizontal(|ui| {
                    ui.label(RichText::new("BOF:").color(text_primary));
                    
                    let selected_text = self.selected_bof_name.as_ref().unwrap_or(&"Select BOF...".to_string()).clone();
                    egui::ComboBox::from_id_source("bof_selection")
                        .selected_text(&selected_text)
                        .show_ui(ui, |ui| {
                            for bof in &self.bof_library {
                                if let Some(name) = bof.get("name").and_then(|v| v.as_str()) {
                                    ui.selectable_value(&mut self.selected_bof_name, Some(name.to_string()), name);
                                }
                            }
                        });
                    
                    if self.selected_bof_name.is_some() {
                        if ui.button("ℹ️").clicked() {
                            if let Some(ref bof_name) = self.selected_bof_name {
                                self.get_bof_help(bof_name);
                            }
                        }
                    }
                });
                
                // Arguments input
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Arguments:").color(text_primary));
                    ui.text_edit_singleline(&mut self.bof_args_input);
                });
                
                // Target agent selection
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Target:").color(text_primary));
                    
                    let target_text = match &self.bof_target_agent {
                        Some(agent) => agent.clone(),
                        None => "Select Agent...".to_string(),
                    };
                    
                    egui::ComboBox::from_id_source("target_agent")
                        .selected_text(&target_text)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.bof_target_agent, Some("local".to_string()), "🧪 Local Test");
                            ui.selectable_value(&mut self.bof_target_agent, Some("all".to_string()), "📡 All Agents");
                            
                            for agent in &self.agents.clone() {
                                ui.selectable_value(
                                    &mut self.bof_target_agent, 
                                    Some(agent.id.clone()), 
                                    format!("🔴 {} ({}@{})", agent.id, agent.username, agent.hostname)
                                );
                            }
                        });
                });
                
                ui.add_space(10.0);
                
                // Execution buttons
                ui.horizontal(|ui| {
                    let can_execute = self.selected_bof_name.is_some() && self.bof_target_agent.is_some();
                    
                    if ui.add_enabled(can_execute, 
                        Button::new(RichText::new("🚀 Execute BOF").color(Color32::WHITE))
                            .fill(accent_green)).clicked() {
                        self.execute_selected_bof();
                    }
                    
                    if ui.add(Button::new(RichText::new("🗑️ Clear").color(Color32::WHITE))
                        .fill(accent_red)).clicked() {
                        self.bof_args_input.clear();
                        self.selected_bof_name = None;
                        self.bof_target_agent = None;
                    }
                });
            });
    }

    // ADD this new method for BOF statistics rendering:
    fn render_bof_statistics(&mut self, ui: &mut Ui, bg_medium: Color32, accent_blue: Color32, accent_green: Color32, 
                            accent_red: Color32, accent_yellow: Color32, text_primary: Color32, text_secondary: Color32) {
        
        ui.label(RichText::new("📊 BOF Execution Statistics").color(accent_blue).size(16.0).strong());
        
        // Statistics cards
        ui.horizontal_wrapped(|ui| {
            // Total BOFs
            Frame::none()
                .fill(bg_medium)
                .inner_margin(Margin::same(10.0))
                .rounding(Rounding::same(6.0))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(format!("{}", self.bof_stats.get("total_bofs").unwrap_or(&0)))
                            .color(accent_blue).size(24.0).strong());
                        ui.label(RichText::new("Total BOFs").color(text_secondary));
                    });
                });
            
            // Total Executions
            Frame::none()
                .fill(bg_medium)
                .inner_margin(Margin::same(10.0))
                .rounding(Rounding::same(6.0))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(format!("{}", self.bof_stats.get("total_executions").unwrap_or(&0)))
                            .color(accent_green).size(24.0).strong());
                        ui.label(RichText::new("Executions").color(text_secondary));
                    });
                });
            
            // Cached BOFs
            Frame::none()
                .fill(bg_medium)
                .inner_margin(Margin::same(10.0))
                .rounding(Rounding::same(6.0))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new(format!("{}", self.bof_stats.get("cached_bofs").unwrap_or(&0)))
                            .color(accent_yellow).size(24.0).strong());
                        ui.label(RichText::new("Cached").color(text_secondary));
                    });
                });
        });
        
        ui.add_space(20.0);
        
        // OPSEC Level Breakdown
        ui.label(RichText::new("🚨 BOFs by OPSEC Level").color(accent_blue).size(14.0).strong());
        
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(10.0))
            .rounding(Rounding::same(6.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("🟢 Stealth: {}", self.bof_stats.get("stealth_bofs").unwrap_or(&0)))
                        .color(accent_green));
                    ui.separator();
                    ui.label(RichText::new(format!("🟡 Careful: {}", self.bof_stats.get("careful_bofs").unwrap_or(&0)))
                        .color(accent_yellow));
                    ui.separator();
                    ui.label(RichText::new(format!("🟠 Standard: {}", self.bof_stats.get("standard_bofs").unwrap_or(&0)))
                        .color(accent_yellow));
                    ui.separator();
                    ui.label(RichText::new(format!("🔴 Loud: {}", self.bof_stats.get("loud_bofs").unwrap_or(&0)))
                        .color(accent_red));
                });
            });
    }

    // ADD these helper methods:
    fn search_bofs(&mut self) {
        if !self.bof_search_query.trim().is_empty() {
            let client_api_clone = self.client_api.clone();
            let query = self.bof_search_query.clone();
            
            self.runtime.spawn_blocking(move || {
                let runtime = Runtime::new().unwrap();
                runtime.block_on(async {
                    if let Ok(client) = client_api_clone.try_lock() {
                        let _ = client.search_bofs(&query).await;
                    }
                });
            });
        }
    }

    fn refresh_bof_library(&mut self) {
        let client_api_clone = self.client_api.clone();
        
        self.runtime.spawn_blocking(move || {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                if let Ok(client) = client_api_clone.try_lock() {
                    let _ = client.get_bof_library().await;
                }
            });
        });
        
        self.set_status("🔄 Refreshing BOF library...");
    }

    fn get_bof_help(&mut self, bof_name: &str) {
        let client_api_clone = self.client_api.clone();
        let name = bof_name.to_string();
        
        self.runtime.spawn_blocking(move || {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                if let Ok(client) = client_api_clone.try_lock() {
                    let _ = client.get_bof_help(&name).await;
                }
            });
        });
    }

    fn execute_selected_bof(&mut self) {
        if let (Some(ref bof_name), Some(ref target)) = (&self.selected_bof_name, &self.bof_target_agent) {
            let client_api_clone = self.client_api.clone();
            let name = bof_name.clone();
            let args = self.bof_args_input.clone();
            let target_clone = target.clone();
            
            self.runtime.spawn_blocking(move || {
                let runtime = Runtime::new().unwrap();
                runtime.block_on(async {
                    if let Ok(client) = client_api_clone.try_lock() {
                        let _ = client.execute_bof_by_name(&name, &args, &target_clone).await;
                    }
                });
            });
            
            self.set_status(&format!("🚀 Executing BOF '{}' on target '{}'", bof_name, target));
            
            // Clear inputs after execution
            self.bof_args_input.clear();
        }
    }

    // UPDATE your existing poll_server method to handle BOF messages:
    fn poll_server(&mut self) {
        let client_api_clone = self.client_api.clone();
        let client_opt = client_api_clone.try_lock().ok();
        
        if let Some(mut client) = client_opt {
            while let Some(msg) = client.try_receive_message() {
                // Handle BOF messages
                self.handle_bof_messages(&msg);
                
                // Handle existing messages (your existing code)
                match &msg {
                    ServerMessage::CommandResult { agent_id, task_id, output, success, .. } => {
                        // Your existing command result handling
                        println!("📥 CLIENT: Received CommandResult for agent {}", agent_id);
                        // ... your existing code ...
                    },
                    ServerMessage::ListenersUpdate { listeners } => {
                        self.listeners = listeners.clone();
                    },
                    ServerMessage::AgentsUpdate { agents } => {
                        self.agents = agents.clone();
                        // Update beacon sessions for new agents
                        for agent in agents {
                            if !self.beacon_sessions.contains_key(&agent.id) {
                                let session = BeaconSession {
                                    agent_id: agent.id.clone(),
                                    hostname: agent.hostname.clone(),
                                    username: agent.username.clone(),
                                    command_history: Vec::new(),
                                    current_directory: "C:\\".to_string(),
                                };
                                self.beacon_sessions.insert(agent.id.clone(), session);
                            }
                        }
                    },
                    // ... handle your other existing messages ...
                    _ => {}
                }
            }
        }
        
        // ... rest of your existing poll_server code ...
    }

impl eframe::App for NetworkAppState {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll_server();
        
        if let Some(time) = self.status_time {
            if time.elapsed() > Duration::from_secs(5) {
                self.status_message = "".to_string();
                self.status_time = None;
            }
        }
        
        // Request frequent repaints for live updates
        ctx.request_repaint_after(Duration::from_millis(1000));
        
        // Handle beacon console window
        if self.show_beacon_console {
            self.render_beacon_console(ctx);
        }
        
        // Top panel with enhanced status
        egui::TopBottomPanel::top("status_panel").show(ctx, |ui| {
            Frame::none()
                .fill(Color32::from_rgb(20, 20, 20))
                .inner_margin(Margin::same(8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🔴 C2 Framework")
                            .color(Color32::from_rgb(255, 105, 97))
                            .size(16.0)
                            .strong());
                        
                        ui.label(RichText::new("LIVE")
                            .color(Color32::from_rgb(152, 251, 152))
                            .size(12.0)
                            .strong());
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("Beacons: {} | Listeners: {}", 
                                self.agents.len(), self.listeners.len()))
                                .color(Color32::from_rgb(170, 170, 170))
                                .size(12.0));
                            
                            if !self.status_message.is_empty() {
                                ui.separator();
                                ui.label(RichText::new(&self.status_message)
                                    .color(Color32::from_rgb(100, 149, 237))
                                    .size(12.0));
                            }
                        });
                    });
                });
        });
        
        // Left panel with enhanced navigation
        egui::SidePanel::left("side_panel").min_width(200.0).show(ctx, |ui| {
            Frame::none()
                .fill(Color32::from_rgb(25, 25, 25))
                .inner_margin(Margin::same(8.0))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Navigation")
                            .color(Color32::from_rgb(100, 149, 237))
                            .size(14.0)
                            .strong());
                    });
                    ui.separator();
                    
                    let tab_color = Color32::from_rgb(100, 149, 237);
                    let selected_color = Color32::from_rgb(152, 251, 152);
                    
                    if ui.selectable_label(self.current_tab == Tab::Dashboard, 
                        RichText::new("📊 Dashboard").color(if self.current_tab == Tab::Dashboard { selected_color } else { tab_color })).clicked() {
                        self.current_tab = Tab::Dashboard;
                    }
                    if ui.selectable_label(self.current_tab == Tab::Listeners, 
                        RichText::new("📡 Listeners").color(if self.current_tab == Tab::Listeners { selected_color } else { tab_color })).clicked() {
                        self.current_tab = Tab::Listeners;
                    }
                    if ui.selectable_label(self.current_tab == Tab::Agents, 
                        RichText::new("🔴 Live Beacons").color(if self.current_tab == Tab::Agents { selected_color } else { tab_color })).clicked() {
                        self.current_tab = Tab::Agents;
                    }
                    if ui.selectable_label(self.current_tab == Tab::Bof, 
                        RichText::new("⚡ BOF Execute").color(if self.current_tab == Tab::Bof { selected_color } else { tab_color })).clicked() {
                        self.current_tab = Tab::Bof;
                    }
                    if ui.selectable_label(self.current_tab == Tab::Settings, 
                        RichText::new("⚙️ Settings").color(if self.current_tab == Tab::Settings { selected_color } else { tab_color })).clicked() {
                        self.current_tab = Tab::Settings;
                    }
                });
        });
        
        // Main panel with content
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_tab {
                Tab::Dashboard => self.render_dashboard(ui),
                Tab::Listeners => self.render_listeners(ui),
                Tab::Agents => self.render_live_agents(ui),
                Tab::Bof => self.render_bof(ui),
                Tab::Settings => self.render_settings(ui),
            }
        });
    }
}

fn format_time_ago(seconds: u64) -> String {
    match seconds {
        0..=59 => format!("{}s ago", seconds),
        60..=3599 => format!("{}m ago", seconds / 60),
        3600..=86399 => format!("{}h ago", seconds / 3600),
        _ => format!("{}d ago", seconds / 86400),
    }
}

fn format_timestamp(time: SystemTime) -> String {
    let duration = time.duration_since(UNIX_EPOCH).unwrap();
    let secs = duration.as_secs();
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}