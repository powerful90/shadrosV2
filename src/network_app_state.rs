// NetworkAppState Implementation
use eframe::egui::{self, Context, Ui, Color32, RichText, ScrollArea, Button};
use egui_extras::{TableBuilder, Column};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
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
        // Clone the client_api Arc to avoid holding the lock while processing messages
        let client_api_clone = self.client_api.clone();
        
        // Get a mutable reference to the client
        let mut client_opt = client_api_clone.try_lock().ok();
        
        if let Some(mut client) = client_opt {
            // Process any pending messages
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
        
        // Periodically request updates
        if self.last_poll.elapsed() > Duration::from_secs(5) {
            // Create separate copies for the runtime
            let client_api_clone = self.client_api.clone();
            
            // Schedule a task to get updates
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
    
    fn add_listener(&mut self) {
        let port = self.listener_port.parse::<u16>().unwrap_or(8080);
        
        let config = ListenerConfig {
            listener_type: self.listener_type.clone(),
            host: self.listener_host.clone(),
            port,
        };
        
        // Create separate copies for the runtime
        let client_api_clone = self.client_api.clone();
        
        // Schedule a task to add listener
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
        // Create separate copies for the runtime
        let client_api_clone = self.client_api.clone();
        
        // Schedule a task to start listener
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
        // Create separate copies for the runtime
        let client_api_clone = self.client_api.clone();
        
        // Schedule a task to stop listener
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
        
        // Create separate copies for the runtime
        let client_api_clone = self.client_api.clone();
        
        // Schedule a task to generate agent
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
    
    fn execute_bof(&mut self) {
        // Create copies for the runtime
        let client_api_clone = self.client_api.clone();
        let bof_path = self.bof_file_path.clone();
        let args = self.bof_args.clone();
        let target = self.bof_target.clone();
        
        // Schedule a task to execute BOF
        self.runtime.spawn_blocking(move || {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                if let Ok(client) = client_api_clone.try_lock() {
                    match client.execute_bof(&bof_path, &args, &target).await {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!("Failed to execute BOF: {}", e);
                        }
                    }
                }
            });
        });
        
        self.set_status("Executing BOF...");
    }
    
    fn browse_bof_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().pick_file() {
            if let Some(path_str) = path.to_str() {
                self.bof_file_path = path_str.to_string();
            }
        }
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
        // Poll the server for updates
        self.poll_server();
        
        // Handle status message timeout
        if let Some(time) = self.status_time {
            if time.elapsed() > Duration::from_secs(5) {
                self.status_message = "".to_string();
                self.status_time = None;
            }
        }
        
        // Request frequent repaints
        ctx.request_repaint_after(Duration::from_millis(100));
        
        // Top panel with status
        egui::TopBottomPanel::top("status_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("C2 Framework");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !self.status_message.is_empty() {
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
            if ui.selectable_label(self.current_tab == Tab::Agents, "Agents").clicked() {
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
                Tab::Agents => self.render_agents(ui),
                Tab::Bof => self.render_bof(ui),
                Tab::Settings => self.render_settings(ui),
            }
        });
    }
}

// UI Rendering
impl NetworkAppState {
    fn render_dashboard(&mut self, ui: &mut Ui) {
        ui.heading("Dashboard");
        ui.separator();
        
        let listener_count = self.listeners.len();
        let agent_count = self.agents.len();
        
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.add_space(10.0);
                ui.heading(format!("{} Listeners", listener_count));
                ui.label("Configure and manage listeners to receive agent connections");
                if ui.button("Manage Listeners").clicked() {
                    self.current_tab = Tab::Listeners;
                }
            });
            
            ui.separator();
            
            ui.vertical(|ui| {
                ui.add_space(10.0);
                ui.heading(format!("{} Agents", agent_count));
                ui.label("View and interact with connected agents");
                if ui.button("Manage Agents").clicked() {
                    self.current_tab = Tab::Agents;
                }
            });
        });
        
        ui.separator();
        ui.heading("Quick Actions");
        
        ui.horizontal(|ui| {
            if ui.button("Add Listener").clicked() {
                self.current_tab = Tab::Listeners;
            }
            
            if ui.button("Generate Agent").clicked() {
                self.current_tab = Tab::Agents;
            }
            
            if ui.button("Execute BOF").clicked() {
                self.current_tab = Tab::Bof;
            }
        });
    }
    
    // Render listeners tab using server data
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
        ui.heading("Existing Listeners");
        
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
                        // Create a copy of the listeners
                        let listeners_snapshot = self.listeners.clone();
                        
                        for listener in listeners_snapshot {
                            let id = listener.id;
                            let running = listener.running;
                            
                            body.row(30.0, |mut row| {
                                row.col(|ui| { ui.label(format!("{:?}", listener.config.listener_type)); });
                                row.col(|ui| { ui.label(&listener.config.host); });
                                row.col(|ui| { ui.label(format!("{}", listener.config.port)); });
                                row.col(|ui| { 
                                    let status = if running { "Running" } else { "Stopped" };
                                    let color = if running { Color32::GREEN } else { Color32::RED };
                                    ui.label(RichText::new(status).color(color)); 
                                });
                                
                                row.col(|ui| {
                                    let start_btn = ui.add_enabled(!running, Button::new("Start"));
                                    if start_btn.clicked() {
                                        let id_copy = id;
                                        self.start_listener(id_copy);
                                    }
                                });
                                
                                row.col(|ui| {
                                    let stop_btn = ui.add_enabled(running, Button::new("Stop"));
                                    if stop_btn.clicked() {
                                        let id_copy = id;
                                        self.stop_listener(id_copy);
                                    }
                                });
                            });
                        }
                    });
            });
        }
    }
    
    fn render_agents(&mut self, ui: &mut Ui) {
        ui.heading("Agent Management");
        ui.separator();
        
        // Agent generation form
        ui.collapsing("Generate Agent", |ui| {
            // Configuration options
            ui.group(|ui| {
                ui.heading("Basic Configuration");
                
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
                            ui.selectable_value(&mut self.agent_format, "service".to_string(), "Windows Service");
                        });
                    
                    ui.label("Architecture:");
                    egui::ComboBox::from_id_source("agent_architecture")
                        .selected_text(&self.agent_architecture)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.agent_architecture, "x64".to_string(), "x64");
                            ui.selectable_value(&mut self.agent_architecture, "x86".to_string(), "x86");
                        });
                });
            });
            
            // Advanced options
            ui.group(|ui| {
                ui.heading("Advanced Options");
                
                ui.horizontal(|ui| {
                    ui.label("Sleep Time (seconds):");
                    ui.text_edit_singleline(&mut self.agent_sleep_time);
                    
                    ui.label("Jitter (%):");
                    ui.text_edit_singleline(&mut self.agent_jitter);
                });
                
                ui.horizontal(|ui| {
                    ui.label("Injection:");
                    egui::ComboBox::from_id_source("agent_injection")
                        .selected_text(&self.agent_injection)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.agent_injection, "self".to_string(), "Self");
                            ui.selectable_value(&mut self.agent_injection, "remote".to_string(), "Remote Process");
                        });
                });
            });
            
            // Output file
            ui.group(|ui| {
                ui.heading("Output");
                
                ui.horizontal(|ui| {
                    ui.label("Output File:");
                    ui.text_edit_singleline(&mut self.agent_output_path);
                    
                    if ui.button("Browse...").clicked() {
                        self.browse_agent_output();
                    }
                });
                
                if ui.button("Generate Agent").clicked() {
                    self.generate_agent();
                }
            });
        });
        
        ui.separator();
        
        // List connected agents
        ui.heading("Connected Agents");
        
        if self.agents.is_empty() {
            ui.label("No agents connected");
        } else {
            ScrollArea::vertical().show(ui, |ui| {
                TableBuilder::new(ui)
                    .column(Column::auto().at_least(100.0))
                    .column(Column::auto().at_least(150.0))
                    .column(Column::auto().at_least(150.0))
                    .column(Column::auto().at_least(100.0))
                    .column(Column::auto().at_least(100.0))
                    .column(Column::remainder())
                    .header(20.0, |mut header| {
                        header.col(|ui| { ui.heading("ID"); });
                        header.col(|ui| { ui.heading("Hostname"); });
                        header.col(|ui| { ui.heading("Username"); });
                        header.col(|ui| { ui.heading("OS"); });
                        header.col(|ui| { ui.heading("IP Address"); });
                        header.col(|ui| { ui.heading("Last Seen"); });
                    })
                    .body(|mut body| {
                        // Create a copy of the agents
                        let agents_snapshot = self.agents.clone();
                        
                        for agent in agents_snapshot {
                            body.row(30.0, |mut row| {
                                row.col(|ui| { ui.label(&agent.id); });
                                row.col(|ui| { ui.label(&agent.hostname); });
                                row.col(|ui| { ui.label(&agent.username); });
                                row.col(|ui| { ui.label(&agent.os_version); });
                                row.col(|ui| { ui.label(&agent.ip_address); });
                                row.col(|ui| { 
                                    // In a real app, format as date/time
                                    ui.label(format!("{}", agent.last_seen)); 
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
        
        // BOF execution form
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("BOF File:");
                ui.text_edit_singleline(&mut self.bof_file_path);
                
                if ui.button("Browse...").clicked() {
                    self.browse_bof_file();
                }
            });
            
            ui.horizontal(|ui| {
                ui.label("Arguments:");
                ui.text_edit_singleline(&mut self.bof_args);
            });
            
            ui.horizontal(|ui| {
                ui.label("Target:");
                egui::ComboBox::from_id_source("bof_target")
                    .selected_text(&self.bof_target)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.bof_target, "all".to_string(), "All Agents");
                        
                        // Create a copy of the agents
                        let agents_snapshot = self.agents.clone();
                        
                        for agent in agents_snapshot {
                            ui.selectable_value(
                                &mut self.bof_target, 
                                agent.id.clone(), 
                                format!("{} ({})", agent.hostname, agent.id)
                            );
                        }
                    });
            });
            
            if ui.button("Execute BOF").clicked() {
                self.execute_bof();
            }
        });
        
        ui.separator();
        
        // Execution history/results would go here
        ui.heading("Execution History");
        ui.label("No execution history available");
    }
    
    fn render_settings(&mut self, ui: &mut Ui) {
        ui.heading("Settings");
        ui.separator();
        
        ui.label("Settings will be implemented in a future version.");
    }
}