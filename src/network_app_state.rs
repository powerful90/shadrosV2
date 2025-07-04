// Enhanced NetworkAppState with live agent management
use eframe::egui::{self, Context, Ui, Color32, RichText, ScrollArea, Button, TextEdit};
use egui_extras::{TableBuilder, Column};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
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
    
    // Command execution state
    command_input: String,
    selected_agent: Option<String>,
    command_history: Vec<(String, String, String)>, // (agent_id, command, timestamp)
    
    // Status messages
    status_message: String,
    status_time: Option<Instant>,
    
    // Data from server
    listeners: Vec<ListenerInfo>,
    agents: Vec<Agent>,
    
    // Last server poll time
    last_poll: Instant,
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
            command_history: Vec::new(),
            
            status_message: "".to_string(),
            status_time: None,
            
            listeners: Vec::new(),
            agents: Vec::new(),
            
            last_poll: Instant::now(),
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
                        self.agents = agents;
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
        if self.last_poll.elapsed() > Duration::from_secs(3) {
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
            self.set_status("Command cannot be empty");
            return;
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
        
        // Add to command history
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();
        
        self.command_history.push((
            agent_id.to_string(),
            command.to_string(),
            timestamp
        ));
        
        self.set_status(&format!("Command '{}' sent to agent {}", command, agent_id));
        self.command_input.clear();
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

// UI Rendering
impl NetworkAppState {
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
                    ui.heading(RichText::new(format!("🔴 {} Live Agents", agent_count)).color(Color32::RED));
                    ui.label("Real-time agent connections");
                    if ui.button("View Agents").clicked() {
                        self.current_tab = Tab::Agents;
                    }
                });
            });
        });
        
        ui.separator();
        
        // Recent agent activity
        if !self.agents.is_empty() {
            ui.heading("Recent Agent Activity");
            ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                for agent in &self.agents {
                    let time_ago = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs() - agent.last_seen;
                    
                    let status_color = if time_ago < 120 { Color32::GREEN } else { Color32::YELLOW };
                    
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("●").color(status_color));
                        ui.label(format!("{}@{}", agent.username, agent.hostname));
                        ui.label(format!("({} ago)", format_time_ago(time_ago)));
                    });
                }
            });
        } else {
            ui.label("No agents connected yet. Generate and run an agent to see it here.");
        }
        
        ui.separator();
        ui.heading("Quick Actions");
        
        ui.horizontal(|ui| {
            if ui.button("🚀 Add Listener").clicked() {
                self.current_tab = Tab::Listeners;
            }
            
            if ui.button("⚡ Generate Agent").clicked() {
                self.current_tab = Tab::Agents;
            }
        });
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
    
    fn render_live_agents(&mut self, ui: &mut Ui) {
        ui.heading("🔴 Live Agents");
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
        
        // Command execution panel
        if !self.agents.is_empty() {
            ui.heading("Command Execution");
            
            ui.horizontal(|ui| {
                ui.label("Target Agent:");
                egui::ComboBox::from_id_source("target_agent")
                    .selected_text(self.selected_agent.as_ref().unwrap_or(&"Select Agent".to_string()))
                    .show_ui(ui, |ui| {
                        for agent in &self.agents {
                            let display_text = format!("{}@{} ({})", agent.username, agent.hostname, agent.id);
                            ui.selectable_value(&mut self.selected_agent, Some(agent.id.clone()), display_text);
                        }
                    });
                
                ui.label("Command:");
                let response = ui.add(TextEdit::singleline(&mut self.command_input).hint_text("Enter command (e.g., whoami, dir, ipconfig)"));
                
                if ui.button("Execute").clicked() && self.selected_agent.is_some() {
                    if let Some(agent_id) = self.selected_agent.clone() {
                        let command = self.command_input.clone();
                        self.execute_command(&agent_id, &command);
                    }
                }
                
                // Execute on Enter key
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && self.selected_agent.is_some() {
                    if let Some(agent_id) = self.selected_agent.clone() {
                        let command = self.command_input.clone();
                        self.execute_command(&agent_id, &command);
                    }
                }
            });
            
            // Quick command buttons
            if self.selected_agent.is_some() {
                ui.horizontal(|ui| {
                    ui.label("Quick Commands:");
                    let quick_commands = vec![
                        ("whoami", "whoami"),
                        ("hostname", "hostname"),
                        ("dir", "dir"),
                        ("ipconfig", "ipconfig"),
                        ("tasklist", "tasklist"),
                        ("systeminfo", "systeminfo"),
                    ];
                    
                    for (label, cmd) in quick_commands {
                        if ui.button(label).clicked() {
                            if let Some(agent_id) = self.selected_agent.clone() {
                                self.execute_command(&agent_id, cmd);
                            }
                        }
                    }
                });
            }
            
            ui.separator();
        }
        
        // Live agents list
        ui.heading(format!("Connected Agents ({})", self.agents.len()));
        
        if self.agents.is_empty() {
            ui.label("No agents connected. Generate and run an agent to see it appear here.");
            ui.label("Agents will appear automatically when they connect to your listeners.");
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
                        header.col(|ui| { ui.heading("Hostname"); });
                        header.col(|ui| { ui.heading("Username"); });
                        header.col(|ui| { ui.heading("OS"); });
                        header.col(|ui| { ui.heading("Last Seen"); });
                        header.col(|ui| { ui.heading("Actions"); });
                    })
                    .body(|mut body| {
                        let agents_snapshot = self.agents.clone();
                        
                        for agent in agents_snapshot {
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
                            
                            body.row(30.0, |mut row| {
                                row.col(|ui| { 
                                    ui.label(RichText::new(status_text).color(status_color)); 
                                });
                                row.col(|ui| { 
                                    ui.label(RichText::new(&agent.id).monospace());
                                });
                                row.col(|ui| { ui.label(&agent.hostname); });
                                row.col(|ui| { ui.label(&agent.username); });
                                row.col(|ui| { ui.label(&agent.os_version); });
                                row.col(|ui| { 
                                    ui.label(format_time_ago(time_ago));
                                });
                                row.col(|ui| {
                                    if ui.button("Select").clicked() {
                                        self.selected_agent = Some(agent.id.clone());
                                    }
                                });
                            });
                        }
                    });
            });
            
            // Command history
            if !self.command_history.is_empty() {
                ui.separator();
                ui.heading("Recent Commands");
                ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                    for (agent_id, command, timestamp) in self.command_history.iter().rev().take(10) {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(agent_id).monospace().small());
                            ui.label(">");
                            ui.label(RichText::new(command).monospace());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(timestamp).small().weak());
                            });
                        });
                    }
                });
            }
        }
    }
    
    fn render_bof(&mut self, ui: &mut Ui) {
        ui.heading("BOF Execution");
        ui.separator();
        
        ui.label("BOF (Beacon Object File) execution will be implemented in a future version.");
        ui.label("Use the Agents tab to execute commands on connected agents.");
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