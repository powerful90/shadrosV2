// src/network_app_state/dashboard.rs - Dashboard rendering
use eframe::egui::{Ui, Color32, RichText, ScrollArea, Button, Frame, Margin, Rounding};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{NetworkAppState, Tab};
use super::helpers::format_time_ago;

pub fn render(app: &mut NetworkAppState, ui: &mut Ui) {
    let bg_medium = Color32::from_rgb(25, 25, 25);
    let accent_blue = Color32::from_rgb(100, 149, 237);
    let accent_green = Color32::from_rgb(152, 251, 152);
    let accent_red = Color32::from_rgb(255, 105, 97);
    let text_primary = Color32::from_rgb(220, 220, 220);
    let text_secondary = Color32::from_rgb(170, 170, 170);
    
    ui.heading(RichText::new("🎯 Dashboard - Live C2 Status").color(accent_blue).size(18.0));
    ui.separator();
    
    let listener_count = app.listeners.len();
    let agent_count = app.agents.len();
    let active_listeners = app.listeners.iter().filter(|l| l.running).count();
    
    // Statistics cards
    ui.horizontal(|ui| {
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(15.0))
            .rounding(Rounding::same(8.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("📡 {} Listeners", listener_count))
                        .color(accent_blue).size(16.0).strong());
                    ui.label(RichText::new(format!("{} active, {} stopped", active_listeners, listener_count - active_listeners))
                        .color(text_secondary).size(12.0));
                    if ui.add(Button::new(RichText::new("Manage Listeners").color(Color32::WHITE))
                        .fill(accent_blue)).clicked() {
                        app.current_tab = Tab::Listeners;
                    }
                });
            });
        
        ui.add_space(10.0);
        
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(15.0))
            .rounding(Rounding::same(8.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(format!("🔴 {} Live Beacons", agent_count))
                        .color(accent_red).size(16.0).strong());
                    ui.label(RichText::new("Real-time beacon connections")
                        .color(text_secondary).size(12.0));
                    if ui.add(Button::new(RichText::new("View Beacons").color(Color32::WHITE))
                        .fill(accent_red)).clicked() {
                        app.current_tab = Tab::Agents;
                    }
                });
            });
    });
    
    ui.separator();
    
    // Recent beacon activity
    let agents_data: Vec<_> = app.agents.iter().map(|agent| {
        let time_ago = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - agent.last_seen;
        
        (agent.id.clone(), agent.username.clone(), agent.hostname.clone(), time_ago)
    }).collect();
    
    if !agents_data.is_empty() {
        ui.label(RichText::new("🕒 Recent Beacon Activity").color(accent_green).size(16.0).strong());
        
        Frame::none()
            .fill(bg_medium)
            .inner_margin(Margin::same(10.0))
            .rounding(Rounding::same(6.0))
            .show(ui, |ui| {
                ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                    for (agent_id, username, hostname, time_ago) in agents_data {
                        let status_color = if time_ago < 120 { accent_green } else { Color32::YELLOW };
                        
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("●").color(status_color).size(12.0));
                            ui.label(RichText::new(format!("{}@{}", username, hostname))
                                .color(text_primary).size(12.0));
                            ui.label(RichText::new(format!("({})", format_time_ago(time_ago)))
                                .color(text_secondary).size(11.0));
                            if ui.add(Button::new(RichText::new("🔗 Interact").color(Color32::WHITE).size(10.0))
                                .fill(accent_blue)).clicked() {
                                app.open_beacon_console(&agent_id);
                            }
                        });
                        ui.add_space(3.0);
                    }
                });
            });
    } else {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(RichText::new("🚀 No beacons connected yet")
                .color(text_secondary).size(14.0));
            ui.label(RichText::new("Generate and run an agent to see it here")
                .color(text_secondary).size(12.0));
            ui.add_space(30.0);
        });
    }
}