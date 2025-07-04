// src/main.rs
mod listener;
mod agent;
mod bof;
mod crypto;
mod models;
mod utils;
mod gui;
mod client_api;
mod connection_dialog;
mod network_app_state;

use connection_dialog::ConnectionDialog;
use network_app_state::NetworkAppState;
use eframe::egui;

struct AppSwitcher {
    connection_dialog: ConnectionDialog,
    network_app_state: Option<NetworkAppState>,
}

impl AppSwitcher {
    fn new() -> Self {
        AppSwitcher {
            connection_dialog: ConnectionDialog::new(),
            network_app_state: None,
        }
    }
}

impl eframe::App for AppSwitcher {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        match &mut self.network_app_state {
            Some(network_app) => {
                // We're connected, show the main app
                network_app.update(ctx, frame);
            },
            None => {
                // Not connected yet, show the connection dialog
                self.connection_dialog.update(ctx, frame);
                
                // Check if connected using the public accessor method
                if self.connection_dialog.is_connected() {
                    if let Some(client_api) = self.connection_dialog.get_client_api() {
                        self.network_app_state = Some(NetworkAppState::new(client_api));
                    }
                }
            }
        }
    }
}

fn main() {
    // Initialize logging
    env_logger::init();
    
    // Create native options
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 800.0)),
        min_window_size: Some(egui::vec2(800.0, 600.0)),
        centered: true,
        ..Default::default()
    };
    
    // Run the application
    eframe::run_native(
        "C2 Framework",
        native_options,
        Box::new(|_cc| Box::new(AppSwitcher::new())),
    ).unwrap();
}