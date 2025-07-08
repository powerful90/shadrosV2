// src/network_app_state/bof.rs - COMPLETE FIXED BOF module
use eframe::egui::{Ui, Color32, RichText, ScrollArea, Button, Frame, Margin, Rounding, TextEdit};
use tokio::runtime::Runtime;

use super::NetworkAppState;

/// Main BOF rendering function - COMPLETELY SIMPLIFIED
pub fn render(app: &mut NetworkAppState, ui: &mut Ui) {
    let bg_medium = Color32::from_rgb(25, 25, 25);
    let accent_blue = Color32::from_rgb(100, 149, 237);
    let accent_green = Color32::from_rgb(152, 251, 152);
    let accent_red = Color32::from_rgb(255, 105, 97);
    let accent_yellow = Color32::from_rgb(255, 215, 0);
    let text_primary = Color32::from_rgb(220, 220, 220);
    let text_secondary = Color32::from_rgb(170, 170, 170);

    ui.heading(RichText::new("⚡ BOF Execution & Management").color(accent_blue).size(18.0));
    ui.separator();

    // BOF statistics header (simplified)
    Frame::none()
        .fill(bg_medium)
        .inner_margin(Margin::same(8.0))
        .rounding(Rounding::same(6.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📚 BOF Library Ready")
                    .color(accent_blue).size(12.0));
                ui.separator();
                ui.label(RichText::new("✅ Execution Engine Online")
                    .color(accent_green).size(12.0));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Real-time BOF Management")
                        .color(text_secondary).size(11.0));
                });
            });
        });

    ui.separator();

    // Main BOF execution form
    Frame::none()
        .fill(bg_medium)
        .inner_margin(Margin::same(10.0))
        .rounding(Rounding::same(6.0))
        .show(ui, |ui| {
            ui.label(RichText::new("🎯 BOF Execution Setup").color(accent_blue).size(16.0).strong());
            
            ui.add_space(10.0);
            
            // BOF file selection
            ui.horizontal(|ui| {
                ui.label(RichText::new("BOF File:").color(text_primary));
                ui.text_edit_singleline(&mut app.bof_search_query); // Reusing this field as file path
                
                if ui.button("📁 Browse").clicked() {
                    browse_bof_file(app);
                }
            });
            
            // Arguments input
            ui.horizontal(|ui| {
                ui.label(RichText::new("Arguments:").color(text_primary));
                let args_input = TextEdit::singleline(&mut app.bof_args_input)
                    .hint_text("Enter BOF arguments...")
                    .desired_width(ui.available_width() - 150.0);
                ui.add(args_input);
                
                if ui.button("📋 Examples").clicked() {
                    show_bof_examples(app);
                }
            });
            
            // Target agent selection
            ui.horizontal(|ui| {
                ui.label(RichText::new("Target:").color(text_primary));
                
                let target_text = match &app.bof_target_agent {
                    Some(agent) => agent.clone(),
                    None => "Select Agent...".to_string(),
                };
                
                egui::ComboBox::from_id_source("bof_target_agent")
                    .selected_text(&target_text)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut app.bof_target_agent, Some("all".to_string()), "📡 All Agents");
                        
                        for agent in &app.agents.clone() {
                            ui.selectable_value(
                                &mut app.bof_target_agent, 
                                Some(agent.id.clone()), 
                                format!("🔴 {} ({}@{})", agent.id, agent.username, agent.hostname)
                            );
                        }
                    });
            });
            
            ui.add_space(10.0);
            
            // Execution buttons
            ui.horizontal(|ui| {
                let can_execute = !app.bof_search_query.trim().is_empty() && app.bof_target_agent.is_some();
                
                if ui.add_enabled(can_execute, 
                    Button::new(RichText::new("🚀 Execute BOF").color(Color32::WHITE))
                        .fill(accent_green)).clicked() {
                    execute_bof(app);
                }
                
                if ui.add(Button::new(RichText::new("🗑️ Clear Form").color(Color32::WHITE))
                    .fill(accent_red)).clicked() {
                    clear_bof_form(app);
                }
                
                // Quick BOF buttons
                render_quick_bof_buttons(app, ui, accent_blue, accent_green, accent_yellow, accent_red);
            });
        });
    
    ui.separator();
    
    // BOF Library section (simplified)
    ui.label(RichText::new("📚 Available BOFs").color(accent_green).size(16.0).strong());
    
    Frame::none()
        .fill(bg_medium)
        .inner_margin(Margin::same(10.0))
        .rounding(Rounding::same(6.0))
        .show(ui, |ui| {
            ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                // Common BOF list
                let common_bofs = vec![
                    ("ps.o", "Process List", "List running processes", "Stealth"),
                    ("ls.o", "Directory List", "List directory contents", "Stealth"),
                    ("whoami.o", "Current User", "Get current user information", "Stealth"),
                    ("hostname.o", "System Name", "Get system hostname", "Stealth"),
                    ("ipconfig.o", "Network Config", "Get network configuration", "Careful"),
                    ("seatbelt.o", "System Enum", "Comprehensive system enumeration", "Loud"),
                ];
                
                for (filename, name, description, opsec) in &common_bofs { // FIXED: added &
                    render_bof_card(app, ui, filename, name, description, opsec, 
                        bg_medium, accent_blue, accent_green, accent_red, accent_yellow, text_secondary);
                    ui.add_space(3.0);
                }
                
                if common_bofs.is_empty() { // Now this works because we didn't move common_bofs
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new("📭 No BOFs available")
                            .color(text_secondary).size(14.0));
                        ui.label(RichText::new("Upload BOF files to see them here")
                            .color(text_secondary).size(12.0));
                        ui.add_space(20.0);
                    });
                }
            });
        });
    
    ui.separator();
    
    // Execution history section
    ui.label(RichText::new("⏱️ Recent BOF Executions").color(accent_blue).size(14.0).strong());
    
    Frame::none()
        .fill(bg_medium)
        .inner_margin(Margin::same(10.0))
        .rounding(Rounding::same(6.0))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("📊 Execution history coming soon")
                    .color(text_secondary).size(12.0));
                ui.label(RichText::new("Real-time BOF execution tracking")
                    .color(text_secondary).size(11.0));
            });
        });
}

/// Render individual BOF card - SIMPLIFIED
fn render_bof_card(
    app: &mut NetworkAppState,
    ui: &mut Ui,
    filename: &str,
    name: &str,
    description: &str,
    opsec_level: &str,
    bg_medium: Color32,
    accent_blue: Color32,
    accent_green: Color32,
    accent_red: Color32,
    accent_yellow: Color32,
    _text_secondary: Color32, // prefixed to avoid warning
) {
    let is_selected = app.selected_bof_name.as_ref() == Some(&filename.to_string());
    
    Frame::none()
        .fill(if is_selected { bg_medium } else { Color32::from_rgb(20, 20, 20) })
        .inner_margin(Margin::same(8.0))
        .rounding(Rounding::same(4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    // BOF name and OPSEC level
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(name).color(accent_blue).size(14.0).strong());
                        
                        let (opsec_icon, opsec_color) = match opsec_level {
                            "Stealth" => ("🟢", accent_green),
                            "Careful" => ("🟡", accent_yellow),
                            "Standard" => ("🟠", accent_yellow),
                            "Loud" => ("🔴", accent_red),
                            _ => ("⚪", Color32::GRAY),
                        };
                        ui.label(RichText::new(opsec_icon).color(opsec_color));
                        ui.label(RichText::new(opsec_level).color(opsec_color).size(10.0));
                    });
                    
                    // Description
                    ui.label(RichText::new(description).color(Color32::GRAY).size(11.0));
                    
                    // Filename
                    ui.label(RichText::new(format!("📁 {}", filename)).color(Color32::GRAY).size(10.0));
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(Button::new(RichText::new("🚀 Execute").color(Color32::WHITE))
                        .fill(accent_green)).clicked() {
                        app.selected_bof_name = Some(filename.to_string());
                        app.bof_search_query = filename.to_string();
                        execute_bof(app);
                    }
                    
                    if ui.add(Button::new(RichText::new("📋 Select").color(Color32::WHITE))
                        .fill(accent_blue)).clicked() {
                        app.selected_bof_name = Some(filename.to_string());
                        app.bof_search_query = filename.to_string();
                    }
                });
            });
        });
}

/// Render quick BOF execution buttons
fn render_quick_bof_buttons(
    app: &mut NetworkAppState, 
    ui: &mut Ui,
    accent_blue: Color32,
    accent_green: Color32,
    accent_yellow: Color32,
    accent_red: Color32,
) {
    ui.separator();
    ui.label(RichText::new("Quick BOFs:").color(Color32::GRAY).size(11.0));
    
    ui.horizontal_wrapped(|ui| {
        let quick_bofs = vec![
            ("ps.o", "Process List", accent_blue),
            ("ls.o", "Directory List", accent_green),
            ("whoami.o", "Current User", accent_yellow),
            ("hostname.o", "System Name", accent_yellow),
            ("seatbelt.o", "System Enum", accent_red),
        ];
        
        for (bof_file, label, color) in quick_bofs {
            if ui.add(
                Button::new(RichText::new(label).color(Color32::WHITE).size(10.0))
                    .fill(color)
                    .rounding(Rounding::same(3.0))
                    .min_size([0.0, 25.0].into())
            ).clicked() {
                app.selected_bof_name = Some(bof_file.to_string());
                app.bof_search_query = bof_file.to_string();
                if app.bof_target_agent.is_some() {
                    execute_bof(app);
                }
            }
        }
    });
}

// Helper functions for BOF operations

/// Browse for BOF file
fn browse_bof_file(app: &mut NetworkAppState) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("BOF Files", &["o", "obj", "coff"])
        .add_filter("All Files", &["*"])
        .pick_file() 
    {
        if let Some(path_str) = path.to_str() {
            app.bof_search_query = path_str.to_string();
            // Extract filename for selection
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                app.selected_bof_name = Some(filename.to_string());
            }
        }
    }
}

/// Execute BOF - FIXED borrow checker issues
fn execute_bof(app: &mut NetworkAppState) {
    if app.bof_search_query.trim().is_empty() {
        app.set_status("❌ Please select a BOF file");
        return;
    }
    
    if app.bof_target_agent.is_none() {
        app.set_status("❌ Please select a target agent");
        return;
    }
    
    let bof_path = app.bof_search_query.clone();
    let args = app.bof_args_input.clone();
    let target = app.bof_target_agent.clone().unwrap();
    
    // Clone these for the status message BEFORE moving into closure
    let bof_name = app.selected_bof_name.clone().unwrap_or_else(|| bof_path.clone());
    let target_clone = target.clone();
    
    let client_api_clone = app.client_api.clone();
    
    app.runtime.spawn_blocking(move || {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            if let Ok(client) = client_api_clone.try_lock() {
                let _ = client.execute_bof(&bof_path, &args, &target).await;
            }
        });
    });
    
    app.set_status(&format!("🚀 Executing BOF '{}' on target '{}'", bof_name, target_clone));
    
    // Clear args after execution for next use
    app.bof_args_input.clear();
}

/// Clear BOF form
fn clear_bof_form(app: &mut NetworkAppState) {
    app.bof_search_query.clear();
    app.bof_args_input.clear();
    app.selected_bof_name = None;
    app.bof_target_agent = None;
    app.set_status("🗑️ BOF form cleared");
}

/// Show BOF examples
fn show_bof_examples(app: &mut NetworkAppState) {
    // This populates common arguments based on selected BOF
    if let Some(ref bof_name) = app.selected_bof_name {
        let example_args = match bof_name.as_str() {
            "ps.o" | "ps" => "-v",
            "ls.o" | "ls" => "C:\\Windows",
            "seatbelt.o" | "seatbelt" => "All",
            "sharphound.o" | "sharphound" => "All",
            "whoami.o" | "whoami" => "/all",
            "ipconfig.o" | "ipconfig" => "/all",
            _ => "",
        };
        
        if !example_args.is_empty() {
            app.bof_args_input = example_args.to_string();
            app.set_status(&format!("📋 Example arguments loaded for '{}'", bof_name));
        } else {
            app.set_status("💡 No example arguments available for this BOF");
        }
    } else {
        app.set_status("💡 Select a BOF first to see example arguments");
    }
}