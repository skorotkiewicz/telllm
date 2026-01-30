use anyhow::{Context, Result};
use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::IpAddr;
use std::path::PathBuf;

pub struct ChatLogger {
    client_dir: PathBuf,
    current_date: String,
}

impl ChatLogger {
    pub fn new(logs_dir: &str, client_ip: IpAddr) -> Result<Self> {
        // Sanitize IP for directory name (replace : with -)
        let ip_str = client_ip.to_string().replace(':', "-");
        let client_dir = PathBuf::from(logs_dir).join(&ip_str);
        let chats_dir = client_dir.join("chats");
        
        // Create directories
        fs::create_dir_all(&chats_dir)
            .context("Failed to create chat logs directory")?;
        
        let current_date = Local::now().format("%d-%m-%y").to_string();
        
        Ok(Self {
            client_dir,
            current_date,
        })
    }

    fn chat_file_path(&self) -> PathBuf {
        self.client_dir
            .join("chats")
            .join(format!("{}.txt", self.current_date))
    }

    fn summary_file_path(&self) -> PathBuf {
        self.client_dir.join("summary.txt")
    }

    pub fn log_message(&self, role: &str, content: &str) -> Result<()> {
        let timestamp = Local::now().format("%H:%M:%S").to_string();
        let chat_path = self.chat_file_path();
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&chat_path)
            .context("Failed to open chat log file")?;

        writeln!(file, "[{}] {}: {}", timestamp, role.to_uppercase(), content)
            .context("Failed to write to chat log")?;

        Ok(())
    }

    pub fn log_session_start(&self) -> Result<()> {
        let timestamp = Local::now().format("%d-%m-%Y %H:%M:%S").to_string();
        let chat_path = self.chat_file_path();
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&chat_path)
            .context("Failed to open chat log file")?;

        writeln!(file, "\n--- Session started at {} ---\n", timestamp)
            .context("Failed to write session start")?;

        Ok(())
    }

    pub fn log_session_end(&self) -> Result<()> {
        let timestamp = Local::now().format("%d-%m-%Y %H:%M:%S").to_string();
        let chat_path = self.chat_file_path();
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&chat_path)
            .context("Failed to open chat log file")?;

        writeln!(file, "\n--- Session ended at {} ---\n", timestamp)
            .context("Failed to write session end")?;

        Ok(())
    }

    pub fn update_summary(&self, key: &str, value: &str) -> Result<()> {
        let summary_path = self.summary_file_path();
        
        // Read existing summary
        let existing = fs::read_to_string(&summary_path).unwrap_or_default();
        
        // Parse into key-value pairs
        let mut entries: Vec<(String, String)> = existing
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, ": ").collect();
                if parts.len() == 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect();

        // Update or add the key
        let key_lower = key.to_lowercase();
        if let Some(entry) = entries.iter_mut().find(|(k, _)| k.to_lowercase() == key_lower) {
            entry.1 = value.to_string();
        } else {
            entries.push((key.to_string(), value.to_string()));
        }

        // Always update last_seen
        let now = Local::now().format("%d-%m-%Y %H:%M:%S").to_string();
        if let Some(entry) = entries.iter_mut().find(|(k, _)| k == "last_seen") {
            entry.1 = now.clone();
        } else {
            entries.push(("last_seen".to_string(), now));
        }

        // Write back
        let content: String = entries
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(&summary_path, content + "\n")
            .context("Failed to write summary file")?;

        Ok(())
    }

    pub fn get_summary(&self) -> Option<String> {
        fs::read_to_string(self.summary_file_path()).ok()
    }

    /// Update just the last_seen timestamp in the summary
    pub fn touch_last_seen(&self) -> Result<()> {
        let summary_path = self.summary_file_path();
        
        // Read existing summary
        let existing = fs::read_to_string(&summary_path).unwrap_or_default();
        
        // Parse into key-value pairs
        let mut entries: Vec<(String, String)> = existing
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(2, ": ").collect();
                if parts.len() == 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect();

        // Update or add last_seen
        let now = Local::now().format("%d-%m-%Y %H:%M:%S").to_string();
        if let Some(entry) = entries.iter_mut().find(|(k, _)| k == "last_seen") {
            entry.1 = now;
        } else {
            entries.push(("last_seen".to_string(), now));
        }

        // Write back
        let content: String = entries
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\n");

        fs::write(&summary_path, content + "\n")
            .context("Failed to write summary file")?;

        Ok(())
    }
}
