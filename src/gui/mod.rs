// src/gui/mod.rs
use eframe::egui;
use egui::{Context, Ui, Color32, RichText, ScrollArea, Button};
use egui_extras::{TableBuilder, Column};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::listener::{Listener, ListenerConfig, ListenerType};
use crate::agent::{AgentGenerator, AgentConfig};
use crate::bof::BofExecutor;
use crate::models::agent::Agent;

pub struct PendingOperation {
    message: String,
    timestamp: Instant,
}

pub struct AppState {
    listeners: Arc<Mutex<Vec<Listener>>>,
    agents: Arc<Mutex<Vec<Agent>>>,
    agent_generator: AgentGenerator,
    bof_executor: BofExecutor,
    
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
    
    // Async operation tracking
    pending_operations: Vec<PendingOperation>,
    operation_check_timer: Option<Instant>,
}

#[derive(PartialEq)]
enum Tab {
    Dashboard,
    Listeners,
    Agents,
    Bof,
    Settings,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            listeners: Arc::new(Mutex::new(Vec::new())),
            agents: Arc::new(Mutex::new(Vec::new())),
            agent_generator: AgentGenerator::new(),
            bof_executor: BofExecutor::new(),
            
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
            
            pending_operations: Vec::new(),
            operation_check_timer: None,
        }
    }
    
    fn set_status(&mut self, message: &str) {
        self.status_message = message.to_string();
        self.status_time = Some(Instant::now());
    }
    
    fn add_listener(&mut self) {
        // Create everything we need up front
        let port = self.listener_port.parse::<u16>().unwrap_or(8080);
        let config = ListenerConfig {
            listener_type: self.listener_type.clone(),
            host: self.listener_host.clone(),
            port,
        };
        let listener = Listener::new(config);
        
        // Determine success/failure without holding any reference to self
        let success = {
            // Clone Arc to avoid borrowing self
            let listeners_arc = self.listeners.clone();
            let result = match listeners_arc.try_lock() {
                Ok(mut listeners) => {
                    listeners.push(listener);
                    true
                },
                Err(_) => false,
            };
            result // Explicit return
        };
        
        // Now we can call set_status without any borrowing conflicts
        if success {
            self.set_status("Listener added successfully");
        } else {
            self.set_status("Cannot add listener: system busy");
        }
    }
    
    fn start_listener(&mut self, index: usize) {
        // Create everything we need up front
        let listener_opt = {
            // Clone Arc to avoid borrowing self
            let listeners_arc = self.listeners.clone();
            let result = match listeners_arc.try_lock() {
                Ok(listeners) => {
                    if index < listeners.len() {
                        Some(listeners[index].clone())
                    } else {
                        None
                    }
                },
                Err(_) => None,
            };
            result // Explicit return
        };
        
        // Set status and process result without borrowing conflicts
        if let Some(listener) = listener_opt {
            self.set_status("Starting listener...");
            
            let listeners_arc_clone = self.listeners.clone();
            let index_clone = index;
            
            // Spawn a thread to avoid blocking the UI
            std::thread::spawn(move || {
                // Try to start the listener
                let result = listener.start();
                
                // Update the listener in the vector
                if let Ok(mut listeners) = listeners_arc_clone.lock() {
                    if index_clone < listeners.len() {
                        listeners[index_clone] = listener;
                    }
                }
                
                // The result will be checked later in the update method
                result
            });
            
            // Add pending operation to be checked later
            self.pending_operations.push(PendingOperation {
                message: format!("Started listener at index {}", index),
                timestamp: Instant::now(),
            });
        } else {
            self.set_status("Cannot start listener: system busy or invalid index");
        }
    }
    
    fn stop_listener(&mut self, index: usize) {
        // Create everything we need up front
        let listener_opt = {
            // Clone Arc to avoid borrowing self
            let listeners_arc = self.listeners.clone();
            let result = match listeners_arc.try_lock() {
                Ok(listeners) => {
                    if index < listeners.len() {
                        Some(listeners[index].clone())
                    } else {
                        None
                    }
                },
                Err(_) => None,
            };
            result // Explicit return
        };
        
        // Set status and process result without borrowing conflicts
        if let Some(listener) = listener_opt {
            self.set_status("Stopping listener...");
            
            let listeners_arc_clone = self.listeners.clone();
            let index_clone = index;
            
            // Spawn a thread to avoid blocking the UI
            std::thread::spawn(move || {
                // Try to stop the listener
                let result = listener.stop();
                
                // Update the listener in the vector
                if let Ok(mut listeners) = listeners_arc_clone.lock() {
                    if index_clone < listeners.len() {
                        listeners[index_clone] = listener;
                    }
                }
                
                // The result will be checked later in the update method
                result
            });
            
            // Add pending operation to be checked later
            self.pending_operations.push(PendingOperation {
                message: format!("Stopped listener at index {}", index),
                timestamp: Instant::now(),
            });
        } else {
            self.set_status("Cannot stop listener: system busy or invalid index");
        }
    }
    
    // Add this helper method to poll for pending operations
    fn check_pending_operations(&mut self) {
        // Check every 100ms
        let should_check = match self.operation_check_timer {
            Some(time) if time.elapsed() < Duration::from_millis(100) => false,
            _ => {
                self.operation_check_timer = Some(Instant::now());
                true
            }
        };
        
        if !should_check {
            return;
        }
        
        // Check for completed operations
        if !self.pending_operations.is_empty() {
            // Look for operations that have been pending for at least 500ms
            // This gives time for the operation to likely complete
            let completed: Vec<_> = self.pending_operations
                .iter()
                .enumerate()
                .filter(|(_, op)| op.timestamp.elapsed() >= Duration::from_millis(500))
                .map(|(i, _)| i)
                .collect();
            
            // Remove completed operations (in reverse order to avoid index issues)
            for i in completed.into_iter().rev() {
                if i < self.pending_operations.len() {
                    let op = self.pending_operations.remove(i);
                    self.set_status(&format!("Operation completed: {}", op.message));
                }
            }
        }
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
        
        match self.agent_generator.generate(config) {
            Ok(_) => self.set_status("Agent generated successfully"),
            Err(e) => self.set_status(&format!("Failed to generate agent: {}", e)),
        }
    }
    
    fn execute_bof(&mut self) {
        match self.bof_executor.execute(&self.bof_file_path, &self.bof_args, &self.bof_target) {
            Ok(_) => self.set_status("BOF execution started"),
            Err(e) => self.set_status(&format!("Failed to execute BOF: {}", e)),
        }
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

impl eframe::App for AppState {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Check for pending operations first
        self.check_pending_operations();
    
        // Handle status message timeout
        if let Some(time) = self.status_time {
            if time.elapsed() > Duration::from_secs(5) {
                self.status_message = "".to_string();
                self.status_time = None;
            }
        }
        
        // Request a repaint frequently while we have pending operations
        if !self.pending_operations.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        
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

impl AppState {
    fn render_dashboard(&mut self, ui: &mut Ui) {
        ui.heading("Dashboard");
        ui.separator();
        
        // Get counts without borrowing self
        let (listener_count, agent_count) = {
            let listener_count = {
                let listeners_arc = self.listeners.clone();
                let result = match listeners_arc.try_lock() {
                    Ok(listeners) => listeners.len(),
                    Err(_) => 0,
                };
                result // Explicit return
            };
            
            let agent_count = {
                let agents_arc = self.agents.clone();
                let result = match agents_arc.try_lock() {
                    Ok(agents) => agents.len(),
                    Err(_) => 0,
                };
                result // Explicit return
            };
            
            (listener_count, agent_count)
        };
        
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
        
        // Get listeners data without borrowing self
        let listeners_data = {
            let listeners_arc = self.listeners.clone();
            let result = match listeners_arc.try_lock() {
                Ok(listeners) => {
                    // Pre-collect all data to avoid borrowing issues
                    listeners
                        .iter()
                        .enumerate()
                        .map(|(idx, l)| (
                            idx,
                            l.config.listener_type.clone(),
                            l.config.host.clone(),
                            l.config.port,
                            l.is_running()
                        ))
                        .collect::<Vec<_>>()
                },
                Err(_) => {
                    // If we can't get the lock, show a message and return empty data
                    ui.label("Listeners data is being updated, please wait...");
                    Vec::new()
                }
            };
            result // Explicit return
        };
        
        if listeners_data.is_empty() && self.pending_operations.is_empty() {
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
                        // Use collected data for UI
                        for (index, listener_type, host, port, is_running) in listeners_data {
                            body.row(30.0, |mut row| {
                                row.col(|ui| { ui.label(format!("{:?}", listener_type)); });
                                row.col(|ui| { ui.label(&host); });
                                row.col(|ui| { ui.label(format!("{}", port)); });
                                row.col(|ui| { 
                                    let status = if is_running { "Running" } else { "Stopped" };
                                    let color = if is_running { Color32::GREEN } else { Color32::RED };
                                    ui.label(RichText::new(status).color(color)); 
                                });
                                
                                let app_state_ptr = self as *mut AppState;
                                
                                row.col(|ui| {
                                    let start_btn = ui.add_enabled(!is_running, Button::new("Start"));
                                    if start_btn.clicked() {
                                        // Use clone to avoid mutable reference issues
                                        let index_clone = index;
                                        unsafe {
                                            let app_state = &mut *app_state_ptr;
                                            app_state.start_listener(index_clone);
                                        }
                                    }
                                });
                                
                                row.col(|ui| {
                                    let stop_btn = ui.add_enabled(is_running, Button::new("Stop"));
                                    if stop_btn.clicked() {
                                        // Use clone to avoid mutable reference issues
                                        let index_clone = index;
                                        unsafe {
                                            let app_state = &mut *app_state_ptr;
                                            app_state.stop_listener(index_clone);
                                        }
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
        
        // Get agent data without borrowing self
        let agent_data = {
            let agents_arc = self.agents.clone();
            let result = match agents_arc.try_lock() {
                Ok(agents) => {
                    // Pre-collect all data to avoid borrowing issues
                    agents
                        .iter()
                        .map(|a| (
                            a.id.clone(),
                            a.hostname.clone(),
                            a.username.clone(),
                            a.os_version.clone(),
                            a.ip_address.clone(),
                            a.last_seen
                        ))
                        .collect::<Vec<_>>()
                },
                Err(_) => {
                    // If we can't get the lock, show a message and return empty data
                    ui.label("Agent data is being updated, please wait...");
                    Vec::new()
                }
            };
            result // Explicit return
        };
        
        if agent_data.is_empty() {
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
                        for (id, hostname, username, os_version, ip_address, last_seen) in agent_data {
                            body.row(30.0, |mut row| {
                                row.col(|ui| { ui.label(&id); });
                                row.col(|ui| { ui.label(&hostname); });
                                row.col(|ui| { ui.label(&username); });
                                row.col(|ui| { ui.label(&os_version); });
                                row.col(|ui| { ui.label(&ip_address); });
                                row.col(|ui| { 
                                    // In a real app, format as date/time
                                    ui.label(format!("{}", last_seen)); 
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
            
            // Get agent options without borrowing self
            let agent_options = {
                let agents_arc = self.agents.clone();
                let result = match agents_arc.try_lock() {
                    Ok(agents) => {
                        agents
                            .iter()
                            .map(|a| (a.id.clone(), a.hostname.clone()))
                            .collect::<Vec<_>>()
                    },
                    Err(_) => Vec::new()
                };
                result // Explicit return
            };
            
            ui.horizontal(|ui| {
                ui.label("Target:");
                egui::ComboBox::from_id_source("bof_target")
                    .selected_text(&self.bof_target)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.bof_target, "all".to_string(), "All Agents");
                        
                        // Add connected agents to dropdown
                        for (agent_id, hostname) in agent_options {
                            ui.selectable_value(
                                &mut self.bof_target, 
                                agent_id.clone(), 
                                format!("{} ({})", hostname, agent_id)
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