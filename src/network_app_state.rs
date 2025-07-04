// Enhanced NetworkAppState with Cobalt Strike-like interface
use eframe::egui::{self, Context, Ui, Color32, RichText, ScrollArea, Button, TextEdit, TextStyle};
use egui_extras::{TableBuilder, Column};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::collections::HashMap;
use tokio::runtime::Runtime;

use crate::client_api::{ClientApi, ServerMessage, ListenerInfo};
use crate::listener::{ListenerConfig, ListenerType};
use crate::agent::AgentConfig;
use crate::models::agent::Agent;

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
                        // Update the command entry with real output from the agent
                        if let Some(session) = self.beacon_sessions.get_mut(&agent_id) {
                            if let Some(cmd_entry) = session.command_history.iter_mut().rev().find(|c| c.task_id == task_id) {
                                cmd_entry.output = Some(output);
                                cmd_entry.success = success;
                            }
                        }
                        self.console_scroll_to_bottom = true;
                        
                        // Also show in status for immediate feedback
                        if success {
                            self.set_status(&format!("Command completed on {}", agent_id));
                        } else {
                            self.set_status(&format!("Command failed on {}", agent_id));
                        }
                    },
                    ServerMessage::Success { message } => {
                        self.set_status(&message);
                    },
                    ServerMessage::Error { message } => {
                        self.set_status(&format!("Error: {}", message));
                    },
                    _ => {}
                }
            }
        }
        
        // Poll server periodically
        if self.last_poll.elapsed() > Duration::from_secs(2) { // Faster polling for real-time feel
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
    
    // Remove the simulate_command_output method since we now get real results from the server
    fn execute_command(&mut self, agent_id: &str, command: &str) {
        if command.trim().is_empty() {
            self.set_status("Command cannot be empty");
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
        
        self.set_status(&format!("Command '{}' sent to beacon", command));
        self.command_input.clear();
        self.console_scroll_to_bottom = true;
    }
    
    fn open_beacon_console(&mut self, agent_id: &str) {
        self.active_beacon = Some(agent_id.to_string());
        self.show_beacon_console = true;
        self.selected_agent = Some(agent_id.to_string());
    }
    
    // [Previous methods for add_listener, start_listener, etc. remain the same...]
    
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
        
        self.set_status("Adding listener...");
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
        
        self.set_status("Starting listener...");
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
        
        self.set_status("Stopping listener...");
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
        
        self.set_status("Generating agent...");
    }
    
    fn browse_agent_output(&mut self) {
        if let Some(path) = rfd::FileDialog::new().save_file() {
            if let Some(path_str) = path.to_str() {
                self.agent_output_path = path_str.to_string();
            }
        }
    }
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
        
        // Top panel with status
        egui::TopBottomPanel::top("status_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("C2 Framework - Live");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Agents: {} | Listeners: {}", self.agents.len(), self.listeners.len()));
                    if !self.status_message.is_empty() {
                        ui.separator();
                        ui.label(RichText::new(&self.status_message).color(Color32::GREEN));
                    }
                });
            });
        });
        
        // Left panel with tabs
        egui::SidePanel::left("side_panel").min_width(200.0).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Navigation");
            });
            ui.separator();
            
            if ui.selectable_label(self.current_tab == Tab::Dashboard, "Dashboard").clicked() {
                self.current_tab = Tab::Dashboard;
            }
            if ui.selectable_label(self.current_tab == Tab::Listeners, "Listeners").clicked() {
                self.current_tab = Tab::Listeners;
            }
            if ui.selectable_label(self.current_tab == Tab::Agents, "🔴 Live Agents").clicked() {
                self.current_tab = Tab::Agents;
            }
            if ui.selectable_label(self.current_tab == Tab::Bof, "BOF Execution").clicked() {
                self.current_tab = Tab::Bof;
            }
            if ui.selectable_label(self.current_tab == Tab::Settings, "Settings").clicked() {
                self.current_tab = Tab::Settings;
            }
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

// UI Rendering Implementation
impl NetworkAppState {
    fn render_beacon_console(&mut self, ctx: &Context) {
        let mut open = true;
        
        if let Some(beacon_id) = &self.active_beacon.clone() {
            let session = self.beacon_sessions.get(beacon_id).cloned();
            
            if let Some(session) = session {
                egui::Window::new(format!("Beacon {} - {}@{}", beacon_id, session.username, session.hostname))
                    .open(&mut open)
                    .resizable(true)
                    .default_size([800.0, 600.0])
                    .show(ctx, |ui| {
                        self.render_beacon_console_content(ui, &session);
                    });
            }
        }
        
        if !open {
            self.show_beacon_console = false;
            self.active_beacon = None;
        }
    }
    
    fn render_beacon_console_content(&mut self, ui: &mut Ui, session: &BeaconSession) {
        ui.horizontal(|ui| {
            ui.label(format!("Beacon {} - {}@{}", session.agent_id, session.username, session.hostname));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear").clicked() {
                    if let Some(mut_session) = self.beacon_sessions.get_mut(&session.agent_id) {
                        mut_session.command_history.clear();
                    }
                }
            });
        });
        
        ui.separator();
        
        // Command output area
        let available_height = ui.available_height() - 60.0; // Reserve space for input
        ScrollArea::vertical()
            .max_height(available_height)
            .auto_shrink([false, false])
            .stick_to_bottom(self.console_scroll_to_bottom)
            .show(ui, |ui| {
                ui.style_mut().override_text_style = Some(TextStyle::Monospace);
                
                for cmd_entry in &session.command_history {
                    // Command prompt line
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("[{}]", cmd_entry.timestamp)).color(Color32::GRAY).small());
                        ui.label(RichText::new(format!("beacon> ")).color(Color32::LIGHT_BLUE));
                        ui.label(RichText::new(&cmd_entry.command).color(Color32::WHITE));
                    });
                    
                    // Output
                    if let Some(output) = &cmd_entry.output {
                        for line in output.lines() {
                            ui.label(RichText::new(line).color(if cmd_entry.success { Color32::LIGHT_GREEN } else { Color32::LIGHT_RED }));
                        }
                    } else {
                        ui.label(RichText::new("Tasked beacon to run command...").color(Color32::YELLOW));
                    }
                    
                    ui.add_space(5.0);
                }
                
                if self.console_scroll_to_bottom {
                    ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    self.console_scroll_to_bottom = false;
                }
            });
        
        ui.separator();
        
        // Command input
        ui.horizontal(|ui| {
            ui.label(RichText::new("beacon> ").color(Color32::LIGHT_BLUE));
            let response = ui.add(TextEdit::singleline(&mut self.command_input)
                .desired_width(ui.available_width() - 80.0)
                .hint_text("Enter command..."));
            
            if ui.button("Execute").clicked() && !self.command_input.trim().is_empty() {
                let command = self.command_input.clone();
                self.execute_command(&session.agent_id, &command);
            }
            
            // Execute on Enter key
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !self.command_input.trim().is_empty() {
                let command = self.command_input.clone();
                self.execute_command(&session.agent_id, &command);
            }
        });
        
        // Quick command buttons with help
        ui.horizontal_wrapped(|ui| {
            ui.label("Quick Commands:");
            
            let quick_commands = vec![
                ("help", "help"),
                ("whoami", "whoami"),
                ("hostname", "hostname"),
                ("pwd", "pwd"),
                ("dir", "dir"),
                ("ls", "ls -la"),
                ("ipconfig", "ipconfig /all"),
                ("ifconfig", "ifconfig -a"),
                ("tasklist", "tasklist"),
                ("ps", "ps aux"),
                ("netstat", "netstat -an"),
                ("systeminfo", "systeminfo"),
                ("uname", "uname -a"),
            ];
            
            for (label, cmd) in quick_commands {
                if ui.small_button(label).clicked() {
                    self.execute_command(&session.agent_id, cmd);
                }
            }
        });
        
        ui.separator();
        
        // Advanced commands section
        ui.collapsing("Advanced Commands", |ui| {
            ui.horizontal_wrapped(|ui| {
                let advanced_commands = vec![
                    ("Get-Process", "powershell Get-Process"),
                    ("Get-Service", "powershell Get-Service"),
                    ("Get-EventLog", "powershell Get-EventLog -LogName System -Newest 10"),
                    ("Get-LocalUser", "powershell Get-LocalUser"),
                    ("Get-LocalGroup", "powershell Get-LocalGroup"),
                    ("Get-NetTCPConnection", "powershell Get-NetTCPConnection"),
                    ("Get-ChildItem", "powershell Get-ChildItem -Force"),
                    ("Get-WmiObject", "powershell Get-WmiObject Win32_ComputerSystem"),
                ];
                
                for (label, cmd) in advanced_commands {
                    if ui.small_button(label).clicked() {
                        self.execute_command(&session.agent_id, cmd);
                    }
                }
            });
        });
    }
    
    fn render_dashboard(&mut self, ui: &mut Ui) {
        ui.heading("Dashboard - Live C2 Status");
        ui.separator();
        
        let listener_count = self.listeners.len();
        let agent_count = self.agents.len();
        let active_listeners = self.listeners.iter().filter(|l| l.running).count();
        
        // Statistics cards
        ui.horizontal(|ui| {
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.add_space(10.0);
                    ui.heading(RichText::new(format!("{} Listeners", listener_count)).color(Color32::BLUE));
                    ui.label(format!("{} active, {} stopped", active_listeners, listener_count - active_listeners));
                    if ui.button("Manage Listeners").clicked() {
                        self.current_tab = Tab::Listeners;
                    }
                });
            });
            
            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.add_space(10.0);
                    ui.heading(RichText::new(format!("🔴 {} Live Beacons", agent_count)).color(Color32::RED));
                    ui.label("Real-time beacon connections");
                    if ui.button("View Beacons").clicked() {
                        self.current_tab = Tab::Agents;
                    }
                });
            });
        });
        
        ui.separator();
        
        // Recent beacon activity - collect agent data first to avoid borrowing issues
        let agents_data: Vec<_> = self.agents.iter().map(|agent| {
            let time_ago = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() - agent.last_seen;
            
            (agent.id.clone(), agent.username.clone(), agent.hostname.clone(), time_ago)
        }).collect();
        
        if !agents_data.is_empty() {
            ui.heading("Recent Beacon Activity");
            ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for (agent_id, username, hostname, time_ago) in agents_data {
                    let status_color = if time_ago < 120 { Color32::GREEN } else { Color32::YELLOW };
                    
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("●").color(status_color));
                        ui.label(format!("{}@{}", username, hostname));
                        ui.label(format!("({} ago)", format_time_ago(time_ago)));
                        if ui.small_button("Interact").clicked() {
                            self.open_beacon_console(&agent_id);
                        }
                    });
                }
            });
        } else {
            ui.label("No beacons connected yet. Generate and run an agent to see it here.");
        }
    }
    
    fn render_live_agents(&mut self, ui: &mut Ui) {
        ui.heading("🔴 Live Beacons");
        ui.separator();
        
        // Agent generation form
        ui.collapsing("Generate New Agent", |ui| {
            ui.group(|ui| {
                ui.heading("Configuration");
                
                ui.horizontal(|ui| {
                    ui.label("Listener URL:");
                    ui.text_edit_singleline(&mut self.agent_listener_url);
                });
                
                ui.horizontal(|ui| {
                    ui.label("Format:");
                    egui::ComboBox::from_id_source("agent_format")
                        .selected_text(&self.agent_format)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.agent_format, "exe".to_string(), "Windows EXE");
                            ui.selectable_value(&mut self.agent_format, "dll".to_string(), "Windows DLL");
                        });
                    
                    ui.label("Architecture:");
                    egui::ComboBox::from_id_source("agent_architecture")
                        .selected_text(&self.agent_architecture)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.agent_architecture, "x64".to_string(), "x64");
                            ui.selectable_value(&mut self.agent_architecture, "x86".to_string(), "x86");
                        });
                });
                
                ui.horizontal(|ui| {
                    ui.label("Sleep Time (seconds):");
                    ui.text_edit_singleline(&mut self.agent_sleep_time);
                    
                    ui.label("Jitter (%):");
                    ui.text_edit_singleline(&mut self.agent_jitter);
                });
                
                ui.horizontal(|ui| {
                    ui.label("Output File:");
                    ui.text_edit_singleline(&mut self.agent_output_path);
                    
                    if ui.button("Browse...").clicked() {
                        self.browse_agent_output();
                    }
                });
                
                if ui.button("🚀 Generate Agent").clicked() {
                    self.generate_agent();
                }
            });
        });
        
        ui.separator();
        
        // Live beacons list
        ui.heading(format!("Connected Beacons ({})", self.agents.len()));
        
        if self.agents.is_empty() {
            ui.label("No beacons connected. Generate and run an agent to see it appear here.");
            ui.label("Beacons will appear automatically when they connect to your listeners.");
        } else {
            ScrollArea::vertical().show(ui, |ui| {
                TableBuilder::new(ui)
                    .column(Column::auto().at_least(80.0))
                    .column(Column::auto().at_least(120.0))
                    .column(Column::auto().at_least(100.0))
                    .column(Column::auto().at_least(100.0))
                    .column(Column::auto().at_least(100.0))
                    .column(Column::auto().at_least(80.0))
                    .column(Column::remainder())
                    .header(20.0, |mut header| {
                        header.col(|ui| { ui.heading("Status"); });
                        header.col(|ui| { ui.heading("ID"); });
                        header.col(|ui| { ui.heading("Computer"); });
                        header.col(|ui| { ui.heading("User"); });
                        header.col(|ui| { ui.heading("Process"); });
                        header.col(|ui| { ui.heading("Last Seen"); });
                        header.col(|ui| { ui.heading("Actions"); });
                    })
                    .body(|mut body| {
                        // Collect agent data to avoid borrowing issues
                        let agents_data: Vec<_> = self.agents.iter().map(|agent| {
                            let time_ago = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs() - agent.last_seen;
                            
                            let (status_text, status_color) = if time_ago < 120 {
                                ("🟢 Online", Color32::GREEN)
                            } else if time_ago < 300 {
                                ("🟡 Idle", Color32::YELLOW)
                            } else {
                                ("🔴 Offline", Color32::RED)
                            };
                            
                            (agent.id.clone(), agent.hostname.clone(), agent.username.clone(), 
                             status_text, status_color, time_ago)
                        }).collect();
                        
                        for (agent_id, hostname, username, status_text, status_color, time_ago) in agents_data {
                            body.row(30.0, |mut row| {
                                row.col(|ui| { 
                                    ui.label(RichText::new(status_text).color(status_color)); 
                                });
                                row.col(|ui| { 
                                    ui.label(RichText::new(&agent_id).monospace());
                                });
                                row.col(|ui| { ui.label(&hostname); });
                                row.col(|ui| { ui.label(&username); });
                                row.col(|ui| { ui.label("beacon.exe"); }); // Process name
                                row.col(|ui| { 
                                    ui.label(format_time_ago(time_ago));
                                });
                                row.col(|ui| {
                                    ui.horizontal(|ui| {
                                        if ui.button("Interact").clicked() {
                                            self.open_beacon_console(&agent_id);
                                        }
                                        if ui.small_button("Kill").clicked() {
                                            // Would send kill command to beacon
                                            self.set_status(&format!("Terminating beacon {}", agent_id));
                                        }
                                    });
                                });
                            });
                        }
                    });
            });
        }
    }
    
    fn render_listeners(&mut self, ui: &mut Ui) {
        ui.heading("Listeners");
        ui.separator();
        
        // Add new listener form
        ui.collapsing("Add New Listener", |ui| {
            ui.horizontal(|ui| {
                ui.label("Type:");
                egui::ComboBox::from_id_source("listener_type")
                    .selected_text(format!("{:?}", self.listener_type))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.listener_type, ListenerType::Http, "HTTP");
                        ui.selectable_value(&mut self.listener_type, ListenerType::Https, "HTTPS");
                        ui.selectable_value(&mut self.listener_type, ListenerType::Tcp, "TCP");
                        ui.selectable_value(&mut self.listener_type, ListenerType::Smb, "SMB");
                    });
                
                ui.label("Host:");
                ui.text_edit_singleline(&mut self.listener_host);
                
                ui.label("Port:");
                ui.text_edit_singleline(&mut self.listener_port);
                
                if ui.button("Add Listener").clicked() {
                    self.add_listener();
                }
            });
        });
        
        ui.separator();
        
        // List existing listeners
        ui.heading("Active Listeners");
        
        if self.listeners.is_empty() {
            ui.label("No listeners configured");
        } else {
            ScrollArea::vertical().show(ui, |ui| {
                TableBuilder::new(ui)
                    .column(Column::auto().at_least(100.0))
                    .column(Column::auto().at_least(150.0))
                    .column(Column::auto().at_least(80.0))
                    .column(Column::auto().at_least(80.0))
                    .column(Column::auto().at_least(80.0))
                    .column(Column::remainder())
                    .header(20.0, |mut header| {
                        header.col(|ui| { ui.heading("Type"); });
                        header.col(|ui| { ui.heading("Host"); });
                        header.col(|ui| { ui.heading("Port"); });
                        header.col(|ui| { ui.heading("Status"); });
                        header.col(|ui| { ui.heading("Start"); });
                        header.col(|ui| { ui.heading("Stop"); });
                    })
                    .body(|mut body| {
                        let listeners_snapshot = self.listeners.clone();
                        
                        for listener in listeners_snapshot {
                            let id = listener.id;
                            let running = listener.running;
                            
                            body.row(30.0, |mut row| {
                                row.col(|ui| { ui.label(format!("{:?}", listener.config.listener_type)); });
                                row.col(|ui| { ui.label(&listener.config.host); });
                                row.col(|ui| { ui.label(format!("{}", listener.config.port)); });
                                row.col(|ui| { 
                                    let status = if running { "🟢 Running" } else { "🔴 Stopped" };
                                    let color = if running { Color32::GREEN } else { Color32::RED };
                                    ui.label(RichText::new(status).color(color)); 
                                });
                                
                                row.col(|ui| {
                                    let start_btn = ui.add_enabled(!running, Button::new("Start"));
                                    if start_btn.clicked() {
                                        self.start_listener(id);
                                    }
                                });
                                
                                row.col(|ui| {
                                    let stop_btn = ui.add_enabled(running, Button::new("Stop"));
                                    if stop_btn.clicked() {
                                        self.stop_listener(id);
                                    }
                                });
                            });
                        }
                    });
            });
        }
    }
    
    fn render_bof(&mut self, ui: &mut Ui) {
        ui.heading("BOF Execution");
        ui.separator();
        
        ui.label("BOF (Beacon Object File) execution will be implemented in a future version.");
        ui.label("Use the Beacons tab to interact with connected beacons.");
    }
    
    fn render_settings(&mut self, ui: &mut Ui) {
        ui.heading("Settings");
        ui.separator();
        
        ui.label("Settings panel - coming soon!");
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