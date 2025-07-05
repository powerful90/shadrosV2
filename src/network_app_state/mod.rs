// src/network_app_state/mod.rs - Final fixed version
use eframe::egui::{Context, Color32, RichText, Frame, Margin};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use tokio::runtime::Runtime;

use crate::client_api::{ClientApi, ServerMessage, ListenerInfo};
use crate::listener::ListenerType;
use crate::models::agent::Agent;

// Sub-modules
mod beacon_console;
mod dashboard;
mod listeners;
mod agents;
mod bof;
mod helpers;

// Re-export public components
pub use beacon_console::BeaconConsole;
pub use helpers::{CommandEntry, BeaconSession, format_timestamp};

#[derive(PartialEq)]
enum Tab {
    Dashboard,
    Listeners,
    Agents,
    Bof,
    Settings,
}

pub struct NetworkAppState {
    pub client_api: Arc<Mutex<ClientApi>>,
    pub runtime: Runtime,
    
    // UI state
    pub current_tab: Tab,
    
    // Listener form state
    pub listener_type: ListenerType,
    pub listener_host: String,
    pub listener_port: String,
    
    // Agent form state
    pub agent_listener_url: String,
    pub agent_format: String,
    pub agent_architecture: String,
    pub agent_sleep_time: String,
    pub agent_jitter: String,
    pub agent_injection: String,
    pub agent_output_path: String,
    
    // BOF form state
    pub bof_library: Vec<serde_json::Value>,
    pub bof_stats: HashMap<String, u64>,
    pub bof_search_results: Vec<serde_json::Value>,
    pub bof_search_query: String,
    pub selected_bof_name: Option<String>,
    pub bof_args_input: String,
    pub bof_target_agent: Option<String>,
    pub show_bof_help: bool,
    pub bof_help_text: String,
    pub bof_help_name: String,
    pub show_bof_library_tab: bool,
    pub show_bof_execution_tab: bool,
    pub show_bof_stats_tab: bool,
    
    // Enhanced command execution state
    pub command_input: String,
    pub selected_agent: Option<String>,
    pub beacon_sessions: HashMap<String, BeaconSession>,
    pub active_beacon: Option<String>,
    pub command_counter: u32,
    
    // Status messages
    status_message: String,
    status_time: Option<Instant>,
    
    // Data from server
    pub listeners: Vec<ListenerInfo>,
    pub agents: Vec<Agent>,
    
    // Last server poll time
    last_poll: Instant,
    
    // UI state for beacon console
    pub show_beacon_console: bool,
    pub console_scroll_to_bottom: bool,
    pub command_input_focus: bool,
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
            
            // BOF field initializations
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
    
    pub fn set_status(&mut self, message: &str) {
        self.status_message = message.to_string();
        self.status_time = Some(Instant::now());
    }

    // Handle BOF-related server messages
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
    
    fn poll_server(&mut self) {
        let client_api_clone = self.client_api.clone();
        let client_opt = client_api_clone.try_lock().ok();
        
        if let Some(mut client) = client_opt {
            while let Some(msg) = client.try_receive_message() {
                // Handle BOF messages
                self.handle_bof_messages(&msg);

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
                        // Find and update the command entry
                        if let Some(session) = self.beacon_sessions.get_mut(&agent_id) {
                            let mut updated = false;
                            
                            // Find by exact task_id match
                            for cmd_entry in session.command_history.iter_mut().rev() {
                                if cmd_entry.task_id == task_id {
                                    cmd_entry.output = Some(output.clone());
                                    cmd_entry.success = success;
                                    updated = true;
                                    break;
                                }
                            }
                            
                            // Find most recent pending command if no exact match
                            if !updated {
                                for cmd_entry in session.command_history.iter_mut().rev() {
                                    if cmd_entry.output.is_none() {
                                        cmd_entry.output = Some(output.clone());
                                        cmd_entry.success = success;
                                        cmd_entry.task_id = task_id.clone();
                                        updated = true;
                                        break;
                                    }
                                }
                            }
                            
                            // Add as new entry if still not found
                            if !updated {
                                let cmd_entry = CommandEntry {
                                    timestamp: format_timestamp(SystemTime::now()),
                                    agent_id: agent_id.clone(),
                                    command: "completed".to_string(),
                                    output: Some(output.clone()),
                                    success,
                                    task_id: task_id.clone(),
                                };
                                session.command_history.push(cmd_entry);
                            }
                            
                            self.console_scroll_to_bottom = true;
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
    
    pub fn execute_command(&mut self, agent_id: &str, command: &str) {
        if command.trim().is_empty() {
            self.set_status("❌ Command cannot be empty");
            return;
        }
        
        self.command_counter += 1;
        let task_id = format!("task-{}-{}", agent_id, self.command_counter);
        
        // Add command to history immediately
        let timestamp = format_timestamp(SystemTime::now());
        let cmd_entry = CommandEntry {
            timestamp,
            agent_id: agent_id.to_string(),
            command: command.to_string(),
            output: None,
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
                    let _ = client.execute_command(&agent_id_clone, &command_clone).await;
                }
            });
        });
        
        self.set_status(&format!("📤 Command '{}' sent to beacon", command));
        self.command_input.clear();
        self.console_scroll_to_bottom = true;
        self.command_input_focus = true;
    }
    
    pub fn open_beacon_console(&mut self, agent_id: &str) {
        self.active_beacon = Some(agent_id.to_string());
        self.show_beacon_console = true;
        self.selected_agent = Some(agent_id.to_string());
        self.command_input_focus = true;
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
        
        // Handle beacon console window - simplified approach
        if self.show_beacon_console {
            if let Some(beacon_id) = &self.active_beacon.clone() {
                let session = self.beacon_sessions.get(beacon_id).cloned();
                if let Some(session) = session {
                    // Simple no-op closure - we'll handle command execution differently
                    let execute_command_closure = |_agent_id: &str, _command: &str| {
                        // Commands will be handled through the UI directly
                    };
                    
                    BeaconConsole::render_window(
                        ctx,
                        &mut self.show_beacon_console,
                        &mut self.active_beacon,
                        &session,
                        &mut self.command_input,
                        &mut self.command_input_focus,
                        &mut self.console_scroll_to_bottom,
                        &mut self.beacon_sessions,
                        execute_command_closure,
                    );
                }
            }
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
                Tab::Dashboard => dashboard::render(self, ui),
                Tab::Listeners => listeners::render(self, ui),
                Tab::Agents => agents::render(self, ui),
                Tab::Bof => bof::render(self, ui),
                Tab::Settings => self.render_settings(ui),
            }
        });
    }
}

impl NetworkAppState {
    fn render_settings(&mut self, ui: &mut eframe::egui::Ui) {
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