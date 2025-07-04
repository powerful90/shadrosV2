// Connection Dialog implementation
use eframe::egui::{self, Context, RichText, TextEdit, Color32};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use crate::client_api::ClientApi;

pub struct ConnectionDialog {
    server_address: String,
    port: String,
    password: String,
    connecting: bool,
    error_message: String,
    client_api: Option<Arc<Mutex<ClientApi>>>,
    runtime: Runtime,
    connected: bool,
}

impl ConnectionDialog {
    pub fn new() -> Self {
        ConnectionDialog {
            server_address: "localhost".to_string(),
            port: "50050".to_string(),
            password: "".to_string(),
            connecting: false,
            error_message: "".to_string(),
            client_api: None,
            runtime: Runtime::new().unwrap(),
            connected: false,
        }
    }
    
    // Public accessor methods for private fields
    pub fn is_connected(&self) -> bool {
        self.connected
    }
    
    pub fn get_client_api(&self) -> Option<Arc<Mutex<ClientApi>>> {
        self.client_api.clone()
    }
    
    pub fn connect(&mut self) -> Option<Arc<Mutex<ClientApi>>> {
        self.connecting = true;
        self.error_message = "".to_string();
        
        let address = format!("{}:{}", self.server_address, self.port);
        let password = self.password.clone();
        
        let mut client = ClientApi::new(address);
        
        // Use the runtime to execute async code
        let result = self.runtime.block_on(async {
            // Connect to the server
            if let Err(e) = client.connect().await {
                return Err(e);
            }
            
            // Authenticate
            match client.authenticate(&password).await {
                Ok(true) => Ok(()),
                Ok(false) => Err("Authentication failed".into()),
                Err(e) => Err(e),
            }
        });
        
        self.connecting = false;
        
        match result {
            Ok(_) => {
                let client_api = Arc::new(Mutex::new(client));
                self.client_api = Some(client_api.clone());
                self.connected = true;
                Some(client_api)
            },
            Err(e) => {
                self.error_message = e;
                None
            }
        }
    }
}

impl eframe::App for ConnectionDialog {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.heading("Connect to C2 Server");
                ui.add_space(20.0);
                
                ui.horizontal(|ui| {
                    ui.label("Server Address:");
                    ui.text_edit_singleline(&mut self.server_address);
                });
                
                ui.horizontal(|ui| {
                    ui.label("Port:");
                    ui.text_edit_singleline(&mut self.port);
                });
                
                ui.horizontal(|ui| {
                    ui.label("Password:");
                    ui.add(TextEdit::singleline(&mut self.password).password(true));
                });
                
                ui.add_space(10.0);
                
                if !self.error_message.is_empty() {
                    ui.label(RichText::new(&self.error_message).color(Color32::RED));
                    ui.add_space(10.0);
                }
                
                if self.connecting {
                    ui.label("Connecting...");
                } else {
                    if ui.button("Connect").clicked() {
                        self.connect();
                    }
                }
            });
        });
    }
}