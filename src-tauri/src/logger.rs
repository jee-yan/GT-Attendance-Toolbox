use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub level: String,
    pub source: String,
    pub message: String,
    pub time: String,
}

pub struct Logger {
    entries: Mutex<Vec<LogEntry>>,
    app_handle: Mutex<Option<AppHandle>>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            app_handle: Mutex::new(None),
        }
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        *self.app_handle.lock().unwrap() = Some(handle);
    }

    fn add_entry(&self, level: &str, source: impl Into<String>, message: impl Into<String>) {
        let entry = LogEntry {
            level: level.to_string(),
            source: source.into(),
            message: message.into(),
            time: chrono::Local::now().format("%H:%M:%S").to_string(),
        };

        // 存储
        self.entries.lock().unwrap().push(entry.clone());

        // 推送到前端
        if let Some(handle) = self.app_handle.lock().unwrap().as_ref() {
            let _ = handle.emit("log-event", &entry);
        }
    }

    pub fn info(&self, source: impl Into<String>, message: impl Into<String>) {
        self.add_entry("info", source, message);
    }

    pub fn error(&self, source: impl Into<String>, message: impl Into<String>) {
        self.add_entry("error", source, message);
    }

    pub fn _warn(&self, source: impl Into<String>, message: impl Into<String>) {
        self.add_entry("warn", source, message);
    }
}
