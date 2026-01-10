use std::fs;
use std::path::PathBuf;

pub struct AppConfig {
    config_dir: PathBuf,
    config_file: PathBuf,
}

impl AppConfig {
    pub fn new() -> Self {
        let config_dir = if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config/samba-share")
        } else {
            PathBuf::from("/tmp/samba-share")
        };

        let config_file = config_dir.join("preferences.conf");

        Self {
            config_dir,
            config_file,
        }
    }

    pub fn ensure_config_dir(&self) -> std::io::Result<()> {
        if !self.config_dir.exists() {
            fs::create_dir_all(&self.config_dir)?;
        }
        Ok(())
    }

    pub fn should_show_welcome(&self) -> bool {
        // If file doesn't exist or can't be read, show welcome (default)
        if !self.config_file.exists() {
            return true;
        }

        match fs::read_to_string(&self.config_file) {
            Ok(content) => {
                // Look for "hide_welcome=true" line
                !content.lines().any(|line| line.trim() == "hide_welcome=true")
            }
            Err(_) => true, // Default to showing welcome on error
        }
    }

    pub fn set_hide_welcome(&self, hide: bool) {
        if let Err(e) = self.ensure_config_dir() {
            eprintln!("Failed to create config directory: {}", e);
            return;
        }

        let content = if hide {
            "hide_welcome=true\n"
        } else {
            "hide_welcome=false\n"
        };

        if let Err(e) = fs::write(&self.config_file, content) {
            eprintln!("Failed to write config file: {}", e);
        }
    }
}