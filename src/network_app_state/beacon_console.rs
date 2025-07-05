// src/network_app_state/beacon_console.rs - Fixed to avoid borrowing conflicts
use eframe::egui::{Context, Ui, Color32, RichText, ScrollArea, Button, TextEdit, TextStyle, Frame, Margin, Rounding, Stroke};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::helpers::{BeaconSession, CommandEntry, format_timestamp};

pub struct BeaconConsole;

impl BeaconConsole {
    pub fn new() -> Self {
        BeaconConsole
    }
    
    // New simplified render method that doesn't take app_state
    pub fn render_window(
        ctx: &Context,
        show_beacon_console: &mut bool,
        active_beacon: &mut Option<String>,
        session: &BeaconSession,
        command_input: &mut String,
        command_input_focus: &mut bool,
        console_scroll_to_bottom: &mut bool,
        beacon_sessions: &mut HashMap<String, BeaconSession>,
        // Instead of taking app_state, we take a closure for command execution
        execute_command_fn: impl Fn(&str, &str),
    ) {
        let mut open = true;
        
        egui::Window::new("")
            .open(&mut open)
            .resizable(true)
            .default_size([1000.0, 700.0])
            .frame(Frame::none()
                .fill(Color32::from_rgb(15, 15, 15))
                .stroke(Stroke::new(1.0, Color32::from_rgb(70, 70, 70))))
            .show(ctx, |ui| {
                Self::render_console_content(
                    ui, 
                    session, 
                    command_input, 
                    command_input_focus, 
                    console_scroll_to_bottom,
                    beacon_sessions,
                    &execute_command_fn,
                );
            });
        
        if !open {
            *show_beacon_console = false;
            *active_beacon = None;
        }
    }
    
    fn render_console_content(
        ui: &mut Ui,
        session: &BeaconSession,
        command_input: &mut String,
        command_input_focus: &mut bool,
        console_scroll_to_bottom: &mut bool,
        beacon_sessions: &mut HashMap<String, BeaconSession>,
        execute_command_fn: &impl Fn(&str, &str),
    ) {
        // Colors
        let bg_dark = Color32::from_rgb(15, 15, 15);
        let bg_medium = Color32::from_rgb(25, 25, 25);
        let accent_blue = Color32::from_rgb(100, 149, 237);
        let accent_green = Color32::from_rgb(152, 251, 152);
        let accent_red = Color32::from_rgb(255, 105, 97);
        let accent_yellow = Color32::from_rgb(255, 215, 0);
        let text_primary = Color32::from_rgb(220, 220, 220);
        let text_secondary = Color32::from_rgb(170, 170, 170);
        
        // Header
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("🔴").size(14.0));
                    ui.label(RichText::new("BEACON").color(accent_blue).size(14.0).strong());
                    ui.separator();
                    ui.label(RichText::new(&session.agent_id).color(text_primary).monospace().size(12.0));
                    ui.separator();
                    ui.label(RichText::new(format!("{}@{}", session.username, session.hostname)).color(accent_green).size(12.0));
                    
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format!("Commands: {}", session.command_history.len())).color(text_secondary).size(11.0));
                        
                        if ui.button(RichText::new("🗑 Clear").color(accent_yellow).size(11.0)).clicked() {
                            if let Some(mut_session) = beacon_sessions.get_mut(&session.agent_id) {
                                mut_session.command_history.clear();
                            }
                        }
                    });
                });
            });
        
        ui.separator();
        
        // Console area
        let available_height = ui.available_height() - 100.0;
        
        Frame::none()
            .fill(bg_dark)
            .inner_margin(Margin::same(8.0))
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .max_height(available_height)
                    .auto_shrink([false, false])
                    .stick_to_bottom(*console_scroll_to_bottom)
                    .show(ui, |ui| {
                        ui.style_mut().override_text_style = Some(TextStyle::Monospace);
                        
                        if session.command_history.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(20.0);
                                ui.label(RichText::new("🚀 Beacon Console Ready").color(accent_blue).size(16.0).strong());
                                ui.label(RichText::new(format!("Connected to {}@{}", session.username, session.hostname)).color(text_secondary).size(12.0));
                                ui.label(RichText::new("Type 'help' for available commands").color(text_secondary).size(11.0));
                                ui.add_space(20.0);
                            });
                        }
                        
                        // Command history
                        for (index, cmd_entry) in session.command_history.iter().enumerate() {
                            if index > 0 {
                                ui.add_space(8.0);
                            }
                            
                            // Command prompt
                            Frame::none()
                                .fill(bg_medium)
                                .inner_margin(Margin::symmetric(8.0, 4.0))
                                .rounding(Rounding::same(4.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("❯").color(accent_blue).size(14.0).strong());
                                        ui.label(RichText::new(&cmd_entry.command).color(text_primary).size(12.0).monospace());
                                        
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(RichText::new(&cmd_entry.timestamp).color(text_secondary).size(10.0));
                                        });
                                    });
                                });
                            
                            // Command output
                            if let Some(output) = &cmd_entry.output {
                                Frame::none()
                                    .fill(bg_dark)
                                    .inner_margin(Margin::symmetric(12.0, 6.0))
                                    .show(ui, |ui| {
                                        let (status_icon, status_color) = if cmd_entry.success {
                                            ("✅", accent_green)
                                        } else {
                                            ("❌", accent_red)
                                        };
                                        
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(status_icon).size(12.0));
                                            ui.label(RichText::new("Output:").color(status_color).size(11.0).strong());
                                        });
                                        
                                        for line in output.lines() {
                                            if line.trim().is_empty() {
                                                ui.add_space(2.0);
                                                continue;
                                            }
                                            
                                            let line_color = if cmd_entry.success {
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
                                            
                                            ui.label(RichText::new(line).color(line_color).size(11.0).monospace());
                                        }
                                    });
                            } else {
                                // Pending command
                                Frame::none()
                                    .fill(Color32::from_rgb(35, 35, 35))
                                    .inner_margin(Margin::symmetric(12.0, 4.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("⏳").color(accent_yellow).size(12.0));
                                            ui.label(RichText::new("Executing command...").color(accent_yellow).size(11.0).italics());
                                        });
                                    });
                            }
                        }
                        
                        if *console_scroll_to_bottom {
                            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                            *console_scroll_to_bottom = false;
                        }
                    });
            });
        
        ui.separator();
        
        // Command input
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("❯").color(accent_blue).size(16.0).strong());
                    
                    let command_input_widget = TextEdit::singleline(command_input)
                        .desired_width(ui.available_width() - 100.0)
                        .hint_text("Enter command...")
                        .font(TextStyle::Monospace)
                        .text_color(text_primary);
                    
                    let response = ui.add(command_input_widget);
                    
                    if *command_input_focus {
                        response.request_focus();
                        *command_input_focus = false;
                    }
                    
                    let execute_button = ui.add(
                        Button::new(RichText::new("Execute").color(Color32::WHITE))
                            .fill(accent_blue)
                            .rounding(Rounding::same(4.0))
                    );
                    
                    if (execute_button.clicked() || 
                        (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))) 
                        && !command_input.trim().is_empty() {
                        let command = command_input.clone();
                        
                        // Add command to history immediately
                        let timestamp = format_timestamp(SystemTime::now());
                        let cmd_entry = CommandEntry {
                            timestamp,
                            agent_id: session.agent_id.clone(),
                            command: command.clone(),
                            output: None,
                            success: false,
                            task_id: format!("task-{}-{}", session.agent_id, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
                        };
                        
                        if let Some(mut_session) = beacon_sessions.get_mut(&session.agent_id) {
                            mut_session.command_history.push(cmd_entry);
                        }
                        
                        // Call the execution function
                        execute_command_fn(&session.agent_id, &command);
                        command_input.clear();
                    }
                });
                
                ui.add_space(4.0);
                
                // Quick commands
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Quick Commands:").color(text_secondary).size(11.0));
                    
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
                            // Add command to history immediately
                            let timestamp = format_timestamp(SystemTime::now());
                            let cmd_entry = CommandEntry {
                                timestamp,
                                agent_id: session.agent_id.clone(),
                                command: cmd.to_string(),
                                output: None,
                                success: false,
                                task_id: format!("task-{}-{}", session.agent_id, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()),
                            };
                            
                            if let Some(mut_session) = beacon_sessions.get_mut(&session.agent_id) {
                                mut_session.command_history.push(cmd_entry);
                            }
                            
                            execute_command_fn(&session.agent_id, cmd);
                        }
                    }
                });
            });
    }
}