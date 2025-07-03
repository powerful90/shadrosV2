use gtk::prelude::*;
use gtk::{Box, Button, ComboBoxText, Entry, Frame, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow};
use std::rc::Rc;
use std::cell::RefCell;

use crate::listener::{Listener, ListenerConfig, ListenerType};
use crate::agent::{AgentGenerator, AgentConfig};
use crate::bof::BofExecutor;

pub fn create_listener_section(listeners: Rc<RefCell<Vec<Listener>>>) -> Box {
    let frame = Frame::new(Some("Listeners"));
    let box_layout = Box::new(Orientation::Vertical, 5);
    
    // Add new listener section
    let new_listener_box = Box::new(Orientation::Horizontal, 5);
    
    let type_combo = ComboBoxText::new();
    type_combo.append(Some("http"), "HTTP");
    type_combo.append(Some("https"), "HTTPS");
    type_combo.append(Some("tcp"), "TCP");
    type_combo.append(Some("smb"), "SMB");
    type_combo.set_active_id(Some("http"));
    
    let host_entry = Entry::new();
    host_entry.set_placeholder_text(Some("Host (e.g., 0.0.0.0)"));
    
    let port_entry = Entry::new();
    port_entry.set_placeholder_text(Some("Port (e.g., 8080)"));
    
    let add_button = Button::with_label("Add Listener");
    
    new_listener_box.pack_start(&type_combo, false, false, 0);
    new_listener_box.pack_start(&host_entry, true, true, 0);
    new_listener_box.pack_start(&port_entry, false, false, 0);
    new_listener_box.pack_start(&add_button, false, false, 0);
    
    // Listeners list
    let listeners_list = ListBox::new();
    let scrolled_window = ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    scrolled_window.set_min_content_height(200);
    scrolled_window.add(&listeners_list);
    
    let listeners_clone = listeners.clone();
    add_button.connect_clicked(move |_| {
        let listener_type = match type_combo.active_id().unwrap().as_str() {
            "http" => ListenerType::Http,
            "https" => ListenerType::Https,
            "tcp" => ListenerType::Tcp,
            "smb" => ListenerType::Smb,
            _ => ListenerType::Http,
        };
        
        let host = host_entry.text().to_string();
        let port = port_entry.text().to_string().parse::<u16>().unwrap_or(8080);
        
        let config = ListenerConfig {
            listener_type,
            host,
            port,
        };
        
        let listener = Listener::new(config);
        
        // Add to the list
        let row = ListBoxRow::new();
        let row_box = Box::new(Orientation::Horizontal, 5);
        let label = Label::new(Some(&format!("{:?} - {}:{}", listener_type, host, port)));
        let start_btn = Button::with_label("Start");
        let stop_btn = Button::with_label("Stop");
        
        row_box.pack_start(&label, true, true, 0);
        row_box.pack_start(&start_btn, false, false, 0);
        row_box.pack_start(&stop_btn, false, false, 0);
        row.add(&row_box);
        
        listeners_list.add(&row);
        listeners_list.show_all();
        
        // Add to our listener vector
        listeners_clone.borrow_mut().push(listener);
        
        // Clear entries
        host_entry.set_text("");
        port_entry.set_text("");
    });
    
    box_layout.pack_start(&new_listener_box, false, false, 0);
    box_layout.pack_start(&scrolled_window, true, true, 0);
    
    frame.add(&box_layout);
    
    let section_box = Box::new(Orientation::Vertical, 5);
    section_box.pack_start(&frame, true, true, 0);
    
    section_box
}

pub fn create_agent_section(agent_generator: AgentGenerator) -> Box {
    let frame = Frame::new(Some("Agent Generation"));
    let box_layout = Box::new(Orientation::Vertical, 5);
    
    // Configuration options
    let config_box = Box::new(Orientation::Horizontal, 5);
    
    let listener_combo = ComboBoxText::new();
    listener_combo.append(Some("http-8080"), "HTTP - 0.0.0.0:8080");
    
    let format_combo = ComboBoxText::new();
    format_combo.append(Some("exe"), "Windows EXE");
    format_combo.append(Some("dll"), "Windows DLL");
    format_combo.append(Some("service"), "Windows Service");
    format_combo.set_active_id(Some("exe"));
    
    let architecture_combo = ComboBoxText::new();
    architecture_combo.append(Some("x64"), "x64");
    architecture_combo.append(Some("x86"), "x86");
    architecture_combo.set_active_id(Some("x64"));
    
    config_box.pack_start(&Label::new(Some("Listener:")), false, false, 0);
    config_box.pack_start(&listener_combo, true, true, 0);
    config_box.pack_start(&Label::new(Some("Format:")), false, false, 0);
    config_box.pack_start(&format_combo, true, true, 0);
    config_box.pack_start(&Label::new(Some("Architecture:")), false, false, 0);
    config_box.pack_start(&architecture_combo, true, true, 0);
    
    // Advanced options
    let advanced_box = Box::new(Orientation::Horizontal, 5);
    
    let sleep_entry = Entry::new();
    sleep_entry.set_placeholder_text(Some("Sleep time (seconds)"));
    sleep_entry.set_text("60");
    
    let jitter_entry = Entry::new();
    jitter_entry.set_placeholder_text(Some("Jitter (%)"));
    jitter_entry.set_text("10");
    
    let inject_combo = ComboBoxText::new();
    inject_combo.append(Some("self"), "Self");
    inject_combo.append(Some("remote"), "Remote Process");
    inject_combo.set_active_id(Some("self"));
    
    advanced_box.pack_start(&Label::new(Some("Sleep:")), false, false, 0);
    advanced_box.pack_start(&sleep_entry, true, true, 0);
    advanced_box.pack_start(&Label::new(Some("Jitter:")), false, false, 0);
    advanced_box.pack_start(&jitter_entry, true, true, 0);
    advanced_box.pack_start(&Label::new(Some("Injection:")), false, false, 0);
    advanced_box.pack_start(&inject_combo, true, true, 0);
    
    // Generate button
    let generate_box = Box::new(Orientation::Horizontal, 5);
    let output_entry = Entry::new();
    output_entry.set_placeholder_text(Some("Output filename"));
    
    let generate_button = Button::with_label("Generate Agent");
    
    generate_box.pack_start(&output_entry, true, true, 0);
    generate_box.pack_start(&generate_button, false, false, 0);
    
    generate_button.connect_clicked(move |_| {
        let config = AgentConfig {
            listener_url: listener_combo.active_text().unwrap().to_string(),
            format: format_combo.active_id().unwrap().to_string(),
            architecture: architecture_combo.active_id().unwrap().to_string(),
            sleep_time: sleep_entry.text().to_string().parse::<u32>().unwrap_or(60),
            jitter: jitter_entry.text().to_string().parse::<u8>().unwrap_or(10),
            injection: inject_combo.active_id().unwrap().to_string(),
            output_path: output_entry.text().to_string(),
        };
        
        // Generate the agent
        match agent_generator.generate(config) {
            Ok(_) => {
                let dialog = gtk::MessageDialog::new(
                    None::<&gtk::Window>,
                    gtk::DialogFlags::MODAL,
                    gtk::MessageType::Info,
                    gtk::ButtonsType::Ok,
                    "Agent generated successfully!"
                );
                dialog.run();
                dialog.close();
            },
            Err(e) => {
                let dialog = gtk::MessageDialog::new(
                    None::<&gtk::Window>,
                    gtk::DialogFlags::MODAL,
                    gtk::MessageType::Error,
                    gtk::ButtonsType::Ok,
                    &format!("Failed to generate agent: {}", e)
                );
                dialog.run();
                dialog.close();
            }
        }
    });
    
    box_layout.pack_start(&config_box, false, false, 0);
    box_layout.pack_start(&advanced_box, false, false, 0);
    box_layout.pack_start(&generate_box, false, false, 0);
    
    frame.add(&box_layout);
    
    let section_box = Box::new(Orientation::Vertical, 5);
    section_box.pack_start(&frame, true, true, 0);
    
    section_box
}

pub fn create_bof_section(bof_executor: BofExecutor, listeners: Rc<RefCell<Vec<Listener>>>) -> Box {
    let frame = Frame::new(Some("BOF Execution"));
    let box_layout = Box::new(Orientation::Vertical, 5);
    
    // BOF file selection
    let file_box = Box::new(Orientation::Horizontal, 5);
    let file_entry = Entry::new();
    file_entry.set_placeholder_text(Some("BOF file path"));
    
    let browse_button = Button::with_label("Browse");
    browse_button.connect_clicked(move |_| {
        let file_chooser = gtk::FileChooserDialog::new(
            Some("Select BOF File"),
            None::<&gtk::Window>,
            gtk::FileChooserAction::Open,
            &[("Cancel", gtk::ResponseType::Cancel), ("Open", gtk::ResponseType::Accept)]
        );
        
        if file_chooser.run() == gtk::ResponseType::Accept {
            if let Some(filename) = file_chooser.filename() {
                if let Some(path) = filename.to_str() {
                    file_entry.set_text(path);
                }
            }
        }
        
        file_chooser.close();
    });
    
    file_box.pack_start(&file_entry, true, true, 0);
    file_box.pack_start(&browse_button, false, false, 0);
    
    // Arguments
    let args_box = Box::new(Orientation::Horizontal, 5);
    let args_entry = Entry::new();
    args_entry.set_placeholder_text(Some("Arguments (comma-separated)"));
    
    args_box.pack_start(&Label::new(Some("Arguments:")), false, false, 0);
    args_box.pack_start(&args_entry, true, true, 0);
    
    // Target selection
    let target_box = Box::new(Orientation::Horizontal, 5);
    let target_combo = ComboBoxText::new();
    target_combo.append(Some("all"), "All Agents");
    
    target_box.pack_start(&Label::new(Some("Target:")), false, false, 0);
    target_box.pack_start(&target_combo, true, true, 0);
    
    // Execute button
    let execute_box = Box::new(Orientation::Horizontal, 5);
    let execute_button = Button::with_label("Execute BOF");
    
    execute_box.pack_end(&execute_button, false, false, 0);
    
    execute_button.connect_clicked(move |_| {
        let bof_path = file_entry.text().to_string();
        let args = args_entry.text().to_string();
        let target = target_combo.active_text().unwrap().to_string();
        
        // Execute the BOF
        match bof_executor.execute(&bof_path, &args, &target) {
            Ok(_) => {
                let dialog = gtk::MessageDialog::new(
                    None::<&gtk::Window>,
                    gtk::DialogFlags::MODAL,
                    gtk::MessageType::Info,
                    gtk::ButtonsType::Ok,
                    "BOF execution started!"
                );
                dialog.run();
                dialog.close();
            },
            Err(e) => {
                let dialog = gtk::MessageDialog::new(
                    None::<&gtk::Window>,
                    gtk::DialogFlags::MODAL,
                    gtk::MessageType::Error,
                    gtk::ButtonsType::Ok,
                    &format!("Failed to execute BOF: {}", e)
                );
                dialog.run();
                dialog.close();
            }
        }
    });
    
    box_layout.pack_start(&file_box, false, false, 0);
    box_layout.pack_start(&args_box, false, false, 0);
    box_layout.pack_start(&target_box, false, false, 0);
    box_layout.pack_start(&execute_box, false, false, 0);
    
    frame.add(&box_layout);
    
    let section_box = Box::new(Orientation::Vertical, 5);
    section_box.pack_start(&frame, true, true, 0);
    
    section_box
}