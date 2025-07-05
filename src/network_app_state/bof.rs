// src/network_app_state/bof.rs - BOF (Beacon Object File) rendering and management
use eframe::egui::{Ui, Color32, RichText, ScrollArea, Button, Frame, Margin, Rounding, Stroke, TextEdit};
use tokio::runtime::Runtime;

use super::NetworkAppState;

/// Main BOF rendering function
pub fn render(app: &mut NetworkAppState, ui: &mut Ui) {
    let bg_medium = Color32::from_rgb(25, 25, 25);
    let accent_blue = Color32::from_rgb(100, 149, 237);
    let accent_green = Color32::from_rgb(152, 251, 152);
    let accent_red = Color32::from_rgb(255, 105, 97);
    let accent_yellow = Color32::from_rgb(255, 215, 0);
    let accent_purple = Color32::from_rgb(186, 85, 211);
    let text_primary = Color32::from_rgb(220, 220, 220);
    let text_secondary = Color32::from_rgb(170, 170, 170);

    ui.heading(RichText::new("⚡ BOF Execution & Management").color(accent_purple).size(18.0));
    
    // BOF statistics header
    render_bof_statistics_header(app, ui, bg_medium, accent_blue, accent_green, accent_yellow, text_secondary);

    ui.separator();

    // Tab navigation
    render_tab_navigation(app, ui, accent_green, text_primary);

    ui.separator();

    // Render appropriate tab content
    if app.show_bof_library_tab {
        render_bof_library_tab(app, ui, bg_medium, accent_blue, accent_green, accent_red, accent_yellow, text_primary, text_secondary);
    } else if app.show_bof_execution_tab {
        render_bof_execution_tab(app, ui, bg_medium, accent_blue, accent_green, accent_red, text_primary);
    } else if app.show_bof_stats_tab {
        render_bof_statistics_tab(app, ui, bg_medium, accent_blue, accent_green, accent_red, accent_yellow, text_secondary);
    }

    // BOF help window
    render_bof_help_window(app, ui, text_primary);
}

/// Render BOF statistics header
fn render_bof_statistics_header(
    app: &NetworkAppState,
    ui: &mut Ui,
    bg_medium: Color32,
    accent_blue: Color32,
    accent_green: Color32,
    accent_yellow: Color32,
    text_secondary: Color32,
) {
    Frame::none()
        .fill(bg_medium)
        .inner_margin(Margin::same(8.0))
        .rounding(Rounding::same(6.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("📚 {} BOFs Available", app.bof_library.len()))
                    .color(accent_blue).size(12.0));
                ui.separator();
                ui.label(RichText::new(format!("✅ {} Executions", 
                    app.bof_stats.get("total_executions").unwrap_or(&0)))
                    .color(accent_green).size(12.0));
                ui.separator();
                ui.label(RichText::new(format!("📦 {} Cached", 
                    app.bof_stats.get("cached_bofs").unwrap_or(&0)))
                    .color(accent_yellow).size(12.0));
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Real-time BOF Management")
                        .color(text_secondary).size(11.0));
                });
            });
        });
}

/// Render tab navigation
fn render_tab_navigation(app: &mut NetworkAppState, ui: &mut Ui, accent_green: Color32, text_primary: Color32) {
    ui.horizontal(|ui| {
        if ui.selectable_label(app.show_bof_library_tab, 
            RichText::new("📚 BOF Library").color(if app.show_bof_library_tab { accent_green } else { text_primary })).clicked() {
            app.show_bof_library_tab = true;
            app.show_bof_execution_tab = false;
            app.show_bof_stats_tab = false;
            refresh_bof_library(app);
        }
        
        if ui.selectable_label(app.show_bof_execution_tab, 
            RichText::new("🚀 Execute BOF").color(if app.show_bof_execution_tab { accent_green } else { text_primary })).clicked() {
            app.show_bof_library_tab = false;
            app.show_bof_execution_tab = true;
            app.show_bof_stats_tab = false;
        }
        
        if ui.selectable_label(app.show_bof_stats_tab, 
            RichText::new("📊 Statistics").color(if app.show_bof_stats_tab { accent_green } else { text_primary })).clicked() {
            app.show_bof_library_tab = false;
            app.show_bof_execution_tab = false;
            app.show_bof_stats_tab = true;
            refresh_bof_stats(app);
        }
    });
}

/// Render BOF library tab
fn render_bof_library_tab(
    app: &mut NetworkAppState,
    ui: &mut Ui,
    bg_medium: Color32,
    accent_blue: Color32,
    accent_green: Color32,
    accent_red: Color32,
    accent_yellow: Color32,
    text_primary: Color32,
    text_secondary: Color32,
) {
    // Search controls
    render_bof_search_controls(app, ui, bg_medium, accent_blue, accent_green, text_primary);
    ui.add_space(5.0);

    // BOF library list
    let use_search_results = !app.bof_search_query.is_empty() && !app.bof_search_results.is_empty();
    
    if use_search_results {
        let search_results = app.bof_search_results.clone();
        if search_results.is_empty() {
            render_empty_bof_library(ui, text_secondary);
        } else {
            render_bof_list(app, ui, &search_results, bg_medium, accent_blue, accent_green, accent_red, accent_yellow, text_secondary);
        }
    } else {
        let library = app.bof_library.clone();
        if library.is_empty() {
            render_empty_bof_library(ui, text_secondary);
        } else {
            render_bof_list(app, ui, &library, bg_medium, accent_blue, accent_green, accent_red, accent_yellow, text_secondary);
        }
    }
}

/// Render BOF search controls
fn render_bof_search_controls(
    app: &mut NetworkAppState,
    ui: &mut Ui,
    bg_medium: Color32,
    accent_blue: Color32,
    accent_green: Color32,
    text_primary: Color32,
) {
    Frame::none()
        .fill(bg_medium)
        .inner_margin(Margin::same(8.0))
        .rounding(Rounding::same(4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔍 Search BOFs:").color(text_primary));
                
                let search_input = TextEdit::singleline(&mut app.bof_search_query)
                    .hint_text("Search by name, description, or tactic...")
                    .desired_width(200.0);
                    
                let response = ui.add(search_input);
                
                if response.changed() && !app.bof_search_query.trim().is_empty() {
                    search_bofs(app);
                }
                
                if ui.add(Button::new(RichText::new("🔍 Search").color(Color32::WHITE))
                    .fill(accent_blue)).clicked() {
                    search_bofs(app);
                }
                
                if ui.add(Button::new(RichText::new("🔄 Refresh").color(Color32::WHITE))
                    .fill(accent_green)).clicked() {
                    refresh_bof_library(app);
                }
                
                if ui.add(Button::new(RichText::new("🗑 Clear").color(Color32::WHITE))
                    .fill(Color32::GRAY)).clicked() {
                    app.bof_search_query.clear();
                    app.bof_search_results.clear();
                }
            });
        });
}

/// Render empty BOF library message
fn render_empty_bof_library(ui: &mut Ui, text_secondary: Color32) {
    ui.vertical_centered(|ui| {
        ui.add_space(30.0);
        ui.label(RichText::new("📭 No BOFs available")
            .color(text_secondary).size(14.0));
        ui.label(RichText::new("Click Refresh to load BOF library from server")
            .color(text_secondary).size(12.0));
        ui.add_space(30.0);
    });
}

/// Render BOF list
fn render_bof_list(
    app: &mut NetworkAppState,
    ui: &mut Ui,
    bofs_to_display: &[serde_json::Value],
    bg_medium: Color32,
    accent_blue: Color32,
    accent_green: Color32,
    accent_red: Color32,
    accent_yellow: Color32,
    text_secondary: Color32,
) {
    // Clone the data to avoid borrowing issues
    let bofs_display_data: Vec<_> = bofs_to_display.iter().map(|bof| {
        let name = bof.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
        let description = bof.get("description").and_then(|v| v.as_str()).unwrap_or("No description").to_string();
        let author = bof.get("author").and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
        let opsec_level = bof.get("opsec_level").and_then(|v| v.as_str()).unwrap_or("Standard").to_string();
        let tactics = bof.get("tactics").and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|t| t.as_str()).collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        let execution_time = bof.get("execution_time_estimate").and_then(|v| v.as_u64()).unwrap_or(0);
        let is_selected = app.selected_bof_name.as_ref() == Some(&name);

        (name, description, author, opsec_level, tactics, execution_time, is_selected)
    }).collect();

    ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
        for (name, description, author, opsec_level, tactics, execution_time, is_selected) in bofs_display_data {
            render_bof_card(
                app, ui, &name, &description, &author, &opsec_level, &tactics, execution_time, is_selected,
                bg_medium, accent_blue, accent_green, accent_red, accent_yellow, text_secondary
            );
            ui.add_space(3.0);
        }
    });
}

/// Render individual BOF card
fn render_bof_card(
    app: &mut NetworkAppState,
    ui: &mut Ui,
    name: &str,
    description: &str,
    author: &str,
    opsec_level: &str,
    tactics: &str,
    execution_time: u64,
    is_selected: bool,
    bg_medium: Color32,
    accent_blue: Color32,
    accent_green: Color32,
    accent_red: Color32,
    accent_yellow: Color32,
    text_secondary: Color32,
) {
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
                ui.vertical(|ui| {
                    // BOF name and OPSEC level
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(name).color(accent_blue).size(14.0).strong());
                        
                        let (opsec_icon, opsec_color) = match opsec_level {
                            "Stealth" => ("🟢", accent_green),
                            "Careful" => ("🟡", accent_yellow),
                            "Standard" => ("🟠", accent_yellow),
                            "Loud" => ("🔴", accent_red),
                            _ => ("⚪", text_secondary),
                        };
                        ui.label(RichText::new(opsec_icon).color(opsec_color));
                        ui.label(RichText::new(opsec_level).color(opsec_color).size(10.0));
                    });
                    
                    // Description
                    ui.label(RichText::new(description).color(text_secondary).size(11.0));
                    
                    // Author and execution time
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("by {}", author)).color(text_secondary).size(10.0));
                        if execution_time > 0 {
                            ui.separator();
                            ui.label(RichText::new(format!("~{}ms", execution_time)).color(text_secondary).size(10.0));
                        }
                    });
                    
                    // Tactics if available
                    if !tactics.is_empty() {
                        ui.label(RichText::new(format!("🎯 {}", tactics)).color(accent_yellow).size(10.0));
                    }
                });
                
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(Button::new(RichText::new("🚀 Execute").color(Color32::WHITE))
                        .fill(accent_green)).clicked() {
                        app.selected_bof_name = Some(name.to_string());
                        app.show_bof_execution_tab = true;
                        app.show_bof_library_tab = false;
                    }
                    
                    if ui.add(Button::new(RichText::new("ℹ️ Help").color(Color32::WHITE))
                        .fill(accent_blue)).clicked() {
                        get_bof_help(app, name);
                    }
                });
            });
        });
}

/// Render BOF execution tab
fn render_bof_execution_tab(
    app: &mut NetworkAppState,
    ui: &mut Ui,
    bg_medium: Color32,
    accent_blue: Color32,
    accent_green: Color32,
    accent_red: Color32,
    text_primary: Color32,
) {
    Frame::none()
        .fill(bg_medium)
        .inner_margin(Margin::same(10.0))
        .rounding(Rounding::same(6.0))
        .show(ui, |ui| {
            ui.label(RichText::new("🎯 BOF Execution Setup").color(accent_blue).size(16.0).strong());
            
            ui.add_space(10.0);
            
            // BOF selection
            render_bof_selection(app, ui, text_primary);
            
            // Arguments input
            render_bof_arguments_input(app, ui, text_primary);
            
            // Target agent selection
            render_target_agent_selection(app, ui, text_primary);
            
            ui.add_space(10.0);
            
            // Execution buttons
            render_execution_buttons(app, ui, accent_green, accent_red);
        });
}

/// Render BOF selection dropdown
fn render_bof_selection(app: &mut NetworkAppState, ui: &mut Ui, text_primary: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("BOF:").color(text_primary));
        
        let selected_text = app.selected_bof_name.as_ref().unwrap_or(&"Select BOF...".to_string()).clone();
        egui::ComboBox::from_id_source("bof_selection")
            .selected_text(&selected_text)
            .show_ui(ui, |ui| {
                for bof in &app.bof_library {
                    if let Some(name) = bof.get("name").and_then(|v| v.as_str()) {
                        let description = bof.get("description").and_then(|v| v.as_str()).unwrap_or("");
                        let display_text = if description.len() > 50 {
                            format!("{} - {}...", name, &description[..50])
                        } else {
                            format!("{} - {}", name, description)
                        };
                        ui.selectable_value(&mut app.selected_bof_name, Some(name.to_string()), display_text);
                    }
                }
            });
        
        if app.selected_bof_name.is_some() {
            if ui.button("ℹ️").clicked() {
                let bof_name = app.selected_bof_name.clone().unwrap();
                get_bof_help(app, &bof_name);
            }
        }
    });
}

/// Render BOF arguments input
fn render_bof_arguments_input(app: &mut NetworkAppState, ui: &mut Ui, text_primary: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Arguments:").color(text_primary));
        
        let args_input = TextEdit::singleline(&mut app.bof_args_input)
            .hint_text("Enter BOF arguments...")
            .desired_width(ui.available_width() - 100.0);
            
        ui.add(args_input);
        
        if ui.button("📋 Examples").clicked() {
            show_bof_examples(app);
        }
    });
}

/// Render target agent selection
fn render_target_agent_selection(app: &mut NetworkAppState, ui: &mut Ui, text_primary: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Target:").color(text_primary));
        
        let target_text = match &app.bof_target_agent {
            Some(agent) => agent.clone(),
            None => "Select Agent...".to_string(),
        };
        
        egui::ComboBox::from_id_source("target_agent")
            .selected_text(&target_text)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut app.bof_target_agent, Some("local".to_string()), "🧪 Local Test");
                ui.selectable_value(&mut app.bof_target_agent, Some("all".to_string()), "📡 All Agents");
                
                for agent in &app.agents.clone() {
                    let status_emoji = "🔴"; // Could be dynamic based on agent status
                    ui.selectable_value(
                        &mut app.bof_target_agent, 
                        Some(agent.id.clone()), 
                        format!("{} {} ({}@{})", status_emoji, agent.id, agent.username, agent.hostname)
                    );
                }
            });
    });
}

/// Render execution buttons
fn render_execution_buttons(app: &mut NetworkAppState, ui: &mut Ui, accent_green: Color32, accent_red: Color32) {
    ui.horizontal(|ui| {
        let can_execute = app.selected_bof_name.is_some() && app.bof_target_agent.is_some();
        
        if ui.add_enabled(can_execute, 
            Button::new(RichText::new("🚀 Execute BOF").color(Color32::WHITE))
                .fill(accent_green)).clicked() {
            execute_selected_bof(app);
        }
        
        if ui.add(Button::new(RichText::new("🗑️ Clear Form").color(Color32::WHITE))
            .fill(accent_red)).clicked() {
            clear_bof_form(app);
        }
        
        // Quick BOF buttons
        render_quick_bof_buttons(app, ui);
    });
}

/// Render quick BOF execution buttons
fn render_quick_bof_buttons(app: &mut NetworkAppState, ui: &mut Ui) {
    ui.separator();
    ui.label(RichText::new("Quick BOFs:").color(Color32::from_rgb(170, 170, 170)).size(11.0));
    
    ui.horizontal_wrapped(|ui| {
        let quick_bofs = vec![
            ("ps", "Process List", Color32::from_rgb(100, 149, 237)),
            ("ls", "Directory List", Color32::from_rgb(152, 251, 152)),
            ("whoami", "Current User", Color32::from_rgb(255, 215, 0)),
            ("hostname", "System Name", Color32::from_rgb(255, 215, 0)),
            ("seatbelt", "System Enum", Color32::from_rgb(255, 105, 97)),
        ];
        
        for (bof_name, label, color) in quick_bofs {
            if ui.add(
                Button::new(RichText::new(label).color(Color32::WHITE).size(10.0))
                    .fill(color)
                    .rounding(Rounding::same(3.0))
                    .min_size([0.0, 25.0].into())
            ).clicked() {
                app.selected_bof_name = Some(bof_name.to_string());
                if app.bof_target_agent.is_some() {
                    execute_selected_bof(app);
                }
            }
        }
    });
}

/// Render BOF statistics tab
fn render_bof_statistics_tab(
    app: &NetworkAppState,
    ui: &mut Ui,
    bg_medium: Color32,
    accent_blue: Color32,
    accent_green: Color32,
    accent_red: Color32,
    accent_yellow: Color32,
    text_secondary: Color32,
) {
    ui.label(RichText::new("📊 BOF Execution Statistics").color(accent_blue).size(16.0).strong());
    
    // Statistics cards
    render_statistics_cards(app, ui, bg_medium, accent_blue, accent_green, accent_yellow, text_secondary);
    
    ui.add_space(20.0);
    
    // OPSEC Level Breakdown
    render_opsec_breakdown(app, ui, bg_medium, accent_blue, accent_green, accent_red, accent_yellow, text_secondary);
    
    ui.add_space(20.0);
    
    // Recent executions
    render_recent_executions(ui, bg_medium, accent_blue, text_secondary);
}

/// Render statistics cards
fn render_statistics_cards(
    app: &NetworkAppState,
    ui: &mut Ui,
    bg_medium: Color32,
    accent_blue: Color32,
    accent_green: Color32,
    accent_yellow: Color32,
    text_secondary: Color32,
) {
    ui.horizontal_wrapped(|ui| {
        // Total BOFs
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(10.0))
            .rounding(Rounding::same(6.0))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(format!("{}", app.bof_stats.get("total_bofs").unwrap_or(&0)))
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
                    ui.label(RichText::new(format!("{}", app.bof_stats.get("total_executions").unwrap_or(&0)))
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
                    ui.label(RichText::new(format!("{}", app.bof_stats.get("cached_bofs").unwrap_or(&0)))
                        .color(accent_yellow).size(24.0).strong());
                    ui.label(RichText::new("Cached").color(text_secondary));
                });
            });
    });
}

/// Render OPSEC level breakdown
fn render_opsec_breakdown(
    app: &NetworkAppState,
    ui: &mut Ui,
    bg_medium: Color32,
    accent_blue: Color32,
    accent_green: Color32,
    accent_red: Color32,
    accent_yellow: Color32,
    text_secondary: Color32,
) {
    ui.label(RichText::new("🚨 BOFs by OPSEC Level").color(accent_blue).size(14.0).strong());
    
    Frame::none()
        .fill(bg_medium)
        .inner_margin(Margin::same(10.0))
        .rounding(Rounding::same(6.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("🟢 Stealth: {}", app.bof_stats.get("stealth_bofs").unwrap_or(&0)))
                    .color(accent_green));
                ui.separator();
                ui.label(RichText::new(format!("🟡 Careful: {}", app.bof_stats.get("careful_bofs").unwrap_or(&0)))
                    .color(accent_yellow));
                ui.separator();
                ui.label(RichText::new(format!("🟠 Standard: {}", app.bof_stats.get("standard_bofs").unwrap_or(&0)))
                    .color(accent_yellow));
                ui.separator();
                ui.label(RichText::new(format!("🔴 Loud: {}", app.bof_stats.get("loud_bofs").unwrap_or(&0)))
                    .color(accent_red));
            });
        });
}

/// Render recent executions placeholder
fn render_recent_executions(ui: &mut Ui, bg_medium: Color32, accent_blue: Color32, _text_secondary: Color32) {
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

/// Render BOF help window
fn render_bof_help_window(app: &mut NetworkAppState, ui: &mut Ui, text_primary: Color32) {
    if app.show_bof_help {
        let mut open = true;
        egui::Window::new(format!("📖 BOF Help: {}", app.bof_help_name))
            .open(&mut open)
            .resizable(true)
            .default_size([600.0, 500.0])
            .show(ui.ctx(), |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    ui.label(RichText::new(&app.bof_help_text)
                        .color(text_primary).size(12.0).monospace());
                });
            });
        
        if !open {
            app.show_bof_help = false;
        }
    }
}

// Helper functions for BOF operations

/// Refresh BOF library from server
fn refresh_bof_library(app: &mut NetworkAppState) {
    let client_api_clone = app.client_api.clone();
    
    app.runtime.spawn_blocking(move || {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            if let Ok(client) = client_api_clone.try_lock() {
                let _ = client.get_bof_library().await;
            }
        });
    });
    
    app.set_status("🔄 Refreshing BOF library...");
}

/// Search BOFs
fn search_bofs(app: &mut NetworkAppState) {
    if !app.bof_search_query.trim().is_empty() {
        let client_api_clone = app.client_api.clone();
        let query = app.bof_search_query.clone();
        
        app.runtime.spawn_blocking(move || {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                if let Ok(client) = client_api_clone.try_lock() {
                    let _ = client.search_bofs(&query).await;
                }
            });
        });
        
        app.set_status("🔍 Searching BOFs...");
    }
}

/// Get BOF help
fn get_bof_help(app: &mut NetworkAppState, bof_name: &str) {
    let client_api_clone = app.client_api.clone();
    let name = bof_name.to_string();
    
    app.runtime.spawn_blocking(move || {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            if let Ok(client) = client_api_clone.try_lock() {
                let _ = client.get_bof_help(&name).await;
            }
        });
    });
    
    app.set_status(&format!("📖 Getting help for BOF '{}'", bof_name));
}

/// Execute selected BOF
fn execute_selected_bof(app: &mut NetworkAppState) {
    if let (Some(ref bof_name), Some(ref target)) = (&app.selected_bof_name, &app.bof_target_agent) {
        let client_api_clone = app.client_api.clone();
        let name = bof_name.clone();
        let args = app.bof_args_input.clone();
        let target_clone = target.clone();
        
        app.runtime.spawn_blocking(move || {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                if let Ok(client) = client_api_clone.try_lock() {
                    let _ = client.execute_bof_by_name(&name, &args, &target_clone).await;
                }
            });
        });
        
        app.set_status(&format!("🚀 Executing BOF '{}' on target '{}'", bof_name, target));
        
        // Clear args after execution for next use
        app.bof_args_input.clear();
    } else {
        app.set_status("❌ Please select both a BOF and target agent");
    }
}

/// Clear BOF form
fn clear_bof_form(app: &mut NetworkAppState) {
    app.bof_args_input.clear();
    app.selected_bof_name = None;
    app.bof_target_agent = None;
    app.set_status("🗑️ BOF form cleared");
}

/// Show BOF examples (placeholder)
fn show_bof_examples(app: &mut NetworkAppState) {
    // This could populate common arguments based on selected BOF
    if let Some(ref bof_name) = app.selected_bof_name {
        let example_args = match bof_name.as_str() {
            "ps" => "-v",
            "ls" => "C:\\Windows",
            "seatbelt" => "All",
            "sharphound" => "All",
            "inlineExecute-Assembly" => "Seatbelt.exe All",
            _ => "",
        };
        
        if !example_args.is_empty() {
            app.bof_args_input = example_args.to_string();
            app.set_status(&format!("📋 Example arguments loaded for '{}'", bof_name));
        }
    }
}

/// Refresh BOF statistics
fn refresh_bof_stats(app: &mut NetworkAppState) {
    let client_api_clone = app.client_api.clone();
    
    app.runtime.spawn_blocking(move || {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            if let Ok(client) = client_api_clone.try_lock() {
                let _ = client.get_bof_stats().await;
            }
        });
    });
    
    app.set_status("📊 Refreshing BOF statistics...");
}