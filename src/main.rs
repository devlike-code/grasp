use std::{
    collections::HashMap,
    path::Path,
    sync::mpsc::{self, Sender},
    thread,
};

use editor_state::GraspEditorState;
use grasp_common::create_native_options;
use iso8601_timestamp::Timestamp;
use log::Record;
mod editor_state;
mod editor_state_machine;
mod grasp_common;
mod grasp_context_menu;
mod grasp_render;
mod grasp_sense;
mod grasp_transitions;
mod grasp_update;
mod utilities;

#[derive(Debug)]
struct SeqWriterData {
    level: String,
    line: String,
    message: String,
    timestamp: String,
    source: String,
}

#[derive(Clone)]
struct SeqWriter {
    sender: Sender<SeqWriterData>,
}

impl SeqWriter {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<SeqWriterData>();
        thread::spawn(move || {
            let req = reqwest::blocking::Client::new();

            loop {
                if let Ok(message) = rx.recv() {
                    let mut hash = HashMap::new();
                    hash.insert("@t", message.timestamp);
                    hash.insert("@m", message.message);
                    hash.insert("@l", message.level);
                    hash.insert("Line", message.line);
                    hash.insert("Source", message.source);
                    hash.insert("User", whoami::username());
                    hash.insert("Platform", whoami::platform().to_string());
                    req.post(
                        "http://localhost:5341/api/events/raw?clef&apiKey=XmGfOzkYRFtYq7b72L0r",
                    )
                    .json(&hash)
                    .send()
                    .unwrap();
                }
            }
        });
        Self { sender: tx }
    }
}

impl SeqWriter {
    fn send(&self, record: &Record) {
        let mut message = record.args().to_string();

        if let Some(p) = record.file() {
            let path = Path::new(p);
            if Path::is_absolute(path) {
                if p.contains("mosaic") {
                    message = format!("[MOSAIC] {}", message);
                } else {
                    message = format!("[EXTERN] {}", message);
                }
            } else {
                message = format!("[GRASP] {}", message);
            }
        }

        let _ = self.sender.send(SeqWriterData {
            source: record.file().unwrap_or_default().to_string(),
            level: record.level().to_string(),
            line: record.line().unwrap_or_default().to_string(),
            message,
            timestamp: Timestamp::now_utc().to_string(),
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let writer = SeqWriter::new();

    env_logger::builder()
        .format(move |buf, record| {
            writer.send(record);
            Ok(())
        })
        .filter_level(log::LevelFilter::Debug)
        .init();
    let app_name = "GRASP";
    let native_options = create_native_options();

    eframe::run_native(
        app_name,
        native_options,
        Box::new(|_| Box::new(GraspEditorState::new())),
    )
}
