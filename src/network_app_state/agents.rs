// src/network_app_state/agents.rs - Agents management
use eframe::egui::{Ui, Color32, RichText, ScrollArea, Button, Frame, Margin, Rounding};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;

use crate::agent::AgentConfig;
use super::{NetworkAppState, helpers::format_time_ago};

pub fn render(app: &mut NetworkAppState, ui: &mut Ui) {
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
                    ui.text_edit_singleline(&mut app.agent_listener_url);
                });
                
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Format:").color(text_primary));
                    egui::ComboBox::from_id_source("agent_format")
                        .selected_text(&app.agent_format)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.agent_format, "exe".to_string(), "Windows EXE");
                            ui.selectable_value(&mut app.agent_format, "dll".to_string(), "Windows DLL");
                        });
                    
                    ui.label(RichText::new("Architecture:").color(text_primary));
                    egui::ComboBox::from_id_source("agent_architecture")
                        .selected_text(&app.agent_architecture)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.agent_architecture, "x64".to_string(), "x64");
                            ui.selectable_value(&mut app.agent_architecture, "x86".to_string(), "x86");
                        });
                });
                
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Sleep Time:").color(text_primary));
                    ui.text_edit_singleline(&mut app.agent_sleep_time);
                    
                    ui.label(RichText::new("Jitter:").color(text_primary));
                    ui.text_edit_singleline(&mut app.agent_jitter);
                    
                    ui.label(RichText::new("Output:").color(text_primary));
                    ui.text_edit_singleline(&mut app.agent_output_path);
                    
                    if ui.button("📁").clicked() {
                        browse_agent_output(app);
                    }
                    
                    if ui.add(Button::new(RichText::new("🚀 Generate").color(Color32::WHITE))
                        .fill(accent_blue)).clicked() {
                        generate_agent(app);
                    }
                });
            });
    });
    
    ui.separator();
    
    // Live beacons list
    ui.label(RichText::new(format!("Connected Beacons ({})", app.agents.len()))
        .color(accent_red).size(16.0).strong());
    
    if app.agents.is_empty() {
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
                                    app.set_status(&format!("🔪 Terminating beacon {}", agent.id));
                                }
                                
                                if ui.add(Button::new(RichText::new("🔗 Interact").color(Color32::WHITE))
                                    .fill(accent_blue)).clicked() {
                                    app.open_beacon_console(&agent.id);
                                }
                            });
                        });
                    });
                ui.add_space(3.0);
            }
        });
    }
}

fn generate_agent(app: &mut NetworkAppState) {
    let sleep_time = app.agent_sleep_time.parse::<u32>().unwrap_or(60);
    let jitter = app.agent_jitter.parse::<u8>().unwrap_or(10);
    
    let config = AgentConfig {
        listener_url: app.agent_listener_url.clone(),
        format: app.agent_format.clone(),
        architecture: app.agent_architecture.clone(),
        sleep_time,
        jitter,
        injection: app.agent_injection.clone(),
        output_path: app.agent_output_path.clone(),
    };
    
    let client_api_clone = app.client_api.clone();
    
    app.runtime.spawn_blocking(move || {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            if let Ok(client) = client_api_clone.try_lock() {
                let _ = client.generate_agent(config).await;
            }
        });
    });
    
    app.set_status("⚙️ Generating agent...");
}

fn browse_agent_output(app: &mut NetworkAppState) {
    if let Some(path) = rfd::FileDialog::new().save_file() {
        if let Some(path_str) = path.to_str() {
            app.agent_output_path = path_str.to_string();
        }
    }
}