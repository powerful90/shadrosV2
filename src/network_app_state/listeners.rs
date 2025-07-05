// src/network_app_state/listeners.rs - Listeners management
use eframe::egui::{Ui, Color32, RichText, Button, Frame, Margin, Rounding};
use tokio::runtime::Runtime;

use crate::listener::{ListenerConfig, ListenerType};
use super::NetworkAppState;

pub fn render(app: &mut NetworkAppState, ui: &mut Ui) {
    let bg_medium = Color32::from_rgb(25, 25, 25);
    let accent_blue = Color32::from_rgb(100, 149, 237);
    let accent_green = Color32::from_rgb(152, 251, 152);
    let accent_red = Color32::from_rgb(255, 105, 97);
    let text_primary = Color32::from_rgb(220, 220, 220);
    
    ui.heading(RichText::new("📡 Listeners").color(accent_blue).size(18.0));
    ui.separator();
    
    // Add new listener form
    ui.collapsing(RichText::new("➕ Add New Listener").color(accent_green), |ui| {
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(10.0))
            .rounding(Rounding::same(6.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Type:").color(text_primary));
                    egui::ComboBox::from_id_source("listener_type")
                        .selected_text(format!("{:?}", app.listener_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.listener_type, ListenerType::Http, "HTTP");
                            ui.selectable_value(&mut app.listener_type, ListenerType::Https, "HTTPS");
                            ui.selectable_value(&mut app.listener_type, ListenerType::Tcp, "TCP");
                            ui.selectable_value(&mut app.listener_type, ListenerType::Smb, "SMB");
                        });
                    
                    ui.label(RichText::new("Host:").color(text_primary));
                    ui.text_edit_singleline(&mut app.listener_host);
                    
                    ui.label(RichText::new("Port:").color(text_primary));
                    ui.text_edit_singleline(&mut app.listener_port);
                    
                    if ui.add(Button::new(RichText::new("🚀 Add Listener").color(Color32::WHITE))
                        .fill(accent_blue)).clicked() {
                        add_listener(app);
                    }
                });
            });
    });
    
    ui.separator();
    
    // List existing listeners
    ui.label(RichText::new("Active Listeners").color(accent_green).size(16.0).strong());
    
    if app.listeners.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(RichText::new("📭 No listeners configured")
                .color(Color32::GRAY).size(14.0));
            ui.add_space(20.0);
        });
    } else {
        let listeners_data: Vec<_> = app.listeners.iter().enumerate().map(|(index, listener)| {
            (index, listener.running, listener.config.listener_type.clone(), 
             listener.config.host.clone(), listener.config.port)
        }).collect();
        
        for (index, running, listener_type, host, port) in listeners_data {
            Frame::none()
                .fill(bg_medium)
                .inner_margin(Margin::same(8.0))
                .rounding(Rounding::same(4.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let status_color = if running { accent_green } else { accent_red };
                        let status_text = if running { "🟢 Running" } else { "🔴 Stopped" };
                        
                        ui.label(RichText::new(status_text).color(status_color));
                        ui.label(RichText::new(format!("{:?}", listener_type)).color(text_primary));
                        ui.label(RichText::new(format!("{}:{}", host, port)).color(text_primary));
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add_enabled(running, 
                                Button::new(RichText::new("Stop").color(Color32::WHITE))
                                    .fill(accent_red)).clicked() {
                                stop_listener(app, index);
                            }
                            
                            if ui.add_enabled(!running,
                                Button::new(RichText::new("Start").color(Color32::WHITE))
                                    .fill(accent_green)).clicked() {
                                start_listener(app, index);
                            }
                        });
                    });
                });
            ui.add_space(5.0);
        }
    }
}

fn add_listener(app: &mut NetworkAppState) {
    let port = app.listener_port.parse::<u16>().unwrap_or(8080);
    
    let config = ListenerConfig {
        listener_type: app.listener_type.clone(),
        host: app.listener_host.clone(),
        port,
    };
    
    let client_api_clone = app.client_api.clone();
    
    app.runtime.spawn_blocking(move || {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            if let Ok(client) = client_api_clone.try_lock() {
                let _ = client.add_listener(config).await;
            }
        });
    });
    
    app.set_status("📡 Adding listener...");
}

fn start_listener(app: &mut NetworkAppState, id: usize) {
    let client_api_clone = app.client_api.clone();
    
    app.runtime.spawn_blocking(move || {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            if let Ok(client) = client_api_clone.try_lock() {
                let _ = client.start_listener(id).await;
            }
        });
    });
    
    app.set_status("🚀 Starting listener...");
}

fn stop_listener(app: &mut NetworkAppState, id: usize) {
    let client_api_clone = app.client_api.clone();
    
    app.runtime.spawn_blocking(move || {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            if let Ok(client) = client_api_clone.try_lock() {
                let _ = client.stop_listener(id).await;
            }
        });
    });
    
    app.set_status("⏹ Stopping listener...");
}