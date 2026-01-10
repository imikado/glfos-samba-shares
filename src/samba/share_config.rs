use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct SambaShareConfig {
    pub name: String,
    pub path: String,
    pub browsable: bool,
    pub read_only: bool,
    pub guest_ok: bool,
    pub force_user: String,
    pub force_group: String,
}

impl SambaShareConfig {
    /// Path to the NixOS configuration file
    const CONFIG_PATH: &'static str = "/etc/nixos/customConfig/default.nix";

    pub fn new(
        name: String,
        path: String,
        browsable: bool,
        read_only: bool,
        guest_ok: bool,
        force_user: String,
        force_group: String,
    ) -> Self {
        Self {
            name,
            path,
            browsable,
            read_only,
            guest_ok,
            force_user,
            force_group,
        }
    }

    /// Load all Samba shares from NixOS configuration
    pub fn load_all() -> Result<Vec<Self>, String> {
        let file = fs::File::open(Self::CONFIG_PATH)
            .map_err(|e| format!("Failed to open {}: {}", Self::CONFIG_PATH, e))?;

        let reader = BufReader::new(file);
        let lines: Vec<String> = reader
            .lines()
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Failed to read {}: {}", Self::CONFIG_PATH, e))?;

        let mut shares = Vec::new();
        let mut in_samba_section = false;
        let mut in_settings_section = false;
        let mut in_share_block = false;
        let mut current_share_name = String::new();
        let mut current_share_props: HashMap<String, String> = HashMap::new();
        let mut section_brace_count = 0;
        let mut share_brace_count = 0;

        for line in lines {
            let trimmed = line.trim();

            // Look for services.samba
            if trimmed.contains("services.samba") && trimmed.contains("=") && trimmed.contains("{") {
                in_samba_section = true;
                continue;
            }

            if in_samba_section && !in_settings_section {
                // Look for settings section
                if trimmed.starts_with("settings") && trimmed.contains("=") {
                    in_settings_section = true;
                    continue;
                }
            }

            if in_settings_section {
                // Check if we're entering a share block (before counting braces)

                
                if !in_share_block && trimmed.contains("=") && trimmed.contains("{") {
                    
                    let cleaned_and_trimmed = trimmed.replace('"',"");

                    // Extract share name
                    if let Some(name) = cleaned_and_trimmed.split("=").nth(0) {

                        let trimmed_name=name.trim();

                        
                        current_share_name = trimmed_name.to_string();
                    }
                    in_share_block = true;
                    share_brace_count = 0;
                    // Count the opening brace on this line
                    share_brace_count += trimmed.matches('{').count() as i32;
                    continue;
                }

                // If we're in a share block, track share-level braces
                if in_share_block {
                    // Parse properties within share block (before checking for closing)
                    if trimmed.contains('=') && !trimmed.contains("= {") {
                        let parts: Vec<&str> = trimmed.splitn(2, '=').collect();
                        if parts.len() == 2 {
                            let key = parts[0].trim().trim_matches('"').to_string();
                            let value = parts[1]
                                .trim()
                                .trim_end_matches(';')
                                .trim_matches('"')
                                .to_string();
                            current_share_props.insert(key, value);
                        }
                    }

                    // Count closing braces
                    share_brace_count -= trimmed.matches('}').count() as i32;

                    // Check if we're leaving the share block
                    if share_brace_count <= 0 {
                        in_share_block = false;

                        // Create share from collected properties
                        let share = Self {
                            name: current_share_name.clone(),
                            path: current_share_props.get("path").cloned().unwrap_or_default(),
                            browsable: current_share_props
                                .get("browseable")
                                .map(|v| v == "yes")
                                .unwrap_or(true),
                            read_only: current_share_props
                                .get("read only")
                                .map(|v| v == "yes")
                                .unwrap_or(false),
                            guest_ok: current_share_props
                                .get("guest ok")
                                .map(|v| v == "yes")
                                .unwrap_or(false),
                            force_user: current_share_props
                                .get("force user")
                                .cloned()
                                .unwrap_or_default(),
                            force_group: current_share_props
                                .get("force group")
                                .cloned()
                                .unwrap_or_default(),
                        };

                        if current_share_name.clone().trim()!="global"{
                           shares.push(share);
                        }

                        current_share_props.clear();
                        current_share_name.clear();
                    }
                    continue;
                }

                // Track section-level braces to know when to exit
                section_brace_count += trimmed.matches('{').count() as i32;
                section_brace_count -= trimmed.matches('}').count() as i32;

                // Exit shares section when we close the main shares block
                if section_brace_count <= 0 && (trimmed == "};" || trimmed == "}") {
                    break;
                }
            }
        }

        Ok(shares)
    }

    /// Write a new Samba share configuration to NixOS
    pub fn write(&self) -> Result<(), String> {
        // Read the current configuration
        let file = fs::File::open(Self::CONFIG_PATH)
            .map_err(|e| format!("Failed to open {}: {}", Self::CONFIG_PATH, e))?;

        let reader = BufReader::new(file);
        let mut lines: Vec<String> = reader
            .lines()
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Failed to read {}: {}", Self::CONFIG_PATH, e))?;

        // Generate the share configuration
        let share_config = format!(
            r#"    "{}" = {{
      path = "{}";
      browseable = {};
      "read only" = {};
      "guest ok" = {};
      "force user" = "{}";
      "force group" = "{}";
    }};"#,
            self.name,
            self.path,
            if self.browsable { "yes" } else { "no" },
            if self.read_only { "yes" } else { "no" },
            if self.guest_ok { "yes" } else { "no" },
            self.force_user,
            self.force_group
        );

        // Find the services.samba settings section and add the new share
        let mut found_settings = false;
        let mut insert_index = None;
        let mut settings_brace_count = 0;
        let mut in_samba_section = false;
        let mut in_settings_section = false;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Look for services.samba
            if trimmed.contains("services.samba") && trimmed.contains("=") && trimmed.contains("{") {
                in_samba_section = true;
                continue;
            }

            if in_samba_section {
                // Look for settings section within services.samba
                if trimmed.starts_with("settings") && trimmed.contains("=") {
                    found_settings = true;
                    in_settings_section = true;
                    // Count the opening brace on the settings line
                    settings_brace_count = trimmed.matches('{').count() as i32;
                    continue;
                }

                if in_settings_section {
                    // Count braces in settings section
                    settings_brace_count += trimmed.matches('{').count() as i32;
                    settings_brace_count -= trimmed.matches('}').count() as i32;

                    // Check if we're at the closing brace of settings section
                    if settings_brace_count == 0 && trimmed == "};" {
                        insert_index = Some(i);
                        break;
                    }
                }
            }
        }

        if !found_settings {
            // services.samba.settings section not found, we need to add it
            // Find the closing brace of the main configuration
            let mut main_closing_brace_idx = None;

            for (i, line) in lines.iter().enumerate().rev() {
                let trimmed = line.trim();
                if trimmed == "}" {
                    main_closing_brace_idx = Some(i);
                    break;
                }
            }

            if let Some(idx) = main_closing_brace_idx {
                // Insert the entire services.samba section with settings
                let samba_section = format!(
                    r#"
  services.samba = {{
    enable = true;
    securityType = "user";
    openFirewall = true;
    settings = {{
        global = {{
          "workgroup" = "WORKGROUP";
          "server string" = "smbnix";
          "netbios name" = "smbnix";
          "security" = "user";
          #"use sendfile" = "yes";
          #"max protocol" = "smb2";
          # note: localhost is the ipv6 localhost ::1
          "hosts allow" = "192.168.0. 127.0.0.1 localhost";
          "hosts deny" = "0.0.0.0/0";
          "guest account" = "nobody";
          "map to guest" = "bad user";
        }};
{}
    }};
  }};"#,
                    share_config
                );
                lines.insert(idx, samba_section);
            } else {
                return Err(
                    "Could not find suitable location to add services.samba section".to_string(),
                );
            }
        } else {
            if let Some(idx) = insert_index {
                // Insert the new share before the closing brace
                lines.insert(idx, share_config);
            } else {
                return Err("Could not find end of services.samba.settings section".to_string());
            }
        }

        // Write back to the file
        let content = lines.join("\n");
        fs::write(Self::CONFIG_PATH, content)
            .map_err(|e| format!("Failed to write to {}: {}", Self::CONFIG_PATH, e))?;

        Ok(())
    }

    /// Update an existing Samba share configuration
    pub fn update(&self, old_name: &str) -> Result<(), String> {
        // Read the current configuration
        let file = fs::File::open(Self::CONFIG_PATH)
            .map_err(|e| format!("Failed to open {}: {}", Self::CONFIG_PATH, e))?;

        let reader = BufReader::new(file);
        let mut lines: Vec<String> = reader
            .lines()
            .collect::<Result<_, _>>()
            .map_err(|e| format!("Failed to read {}: {}", Self::CONFIG_PATH, e))?;

        // Find and remove the old share
        let mut in_samba_section = false;
        let mut in_settings_section = false;
        let mut in_target_share = false;
        let mut share_start_idx = None;
        let mut share_end_idx = None;
        let mut share_brace_count = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Look for services.samba
            if trimmed.contains("services.samba") && trimmed.contains("=") && trimmed.contains("{") {
                in_samba_section = true;
                continue;
            }

            if in_samba_section && !in_settings_section {
                if trimmed.starts_with("settings") && trimmed.contains("=") {
                    in_settings_section = true;
                    continue;
                }
            }

            if in_settings_section {
                // Check if this is the target share
                if !in_target_share && trimmed.starts_with('"') && trimmed.contains("= {") {
                    if let Some(name) = trimmed.split('"').nth(1) {
                        if name == old_name {
                            in_target_share = true;
                            share_start_idx = Some(i);
                            share_brace_count = trimmed.matches('{').count() as i32;
                        }
                    }
                    continue;
                }

                if in_target_share {
                    share_brace_count += trimmed.matches('{').count() as i32;
                    share_brace_count -= trimmed.matches('}').count() as i32;

                    if share_brace_count <= 0 {
                        share_end_idx = Some(i);
                        break;
                    }
                }
            }
        }

        if let (Some(start), Some(end)) = (share_start_idx, share_end_idx) {
            // Remove the old share (inclusive of both start and end)
            lines.drain(start..=end);

            // Generate the new share configuration
            let share_config = format!(
                r#"    "{}" = {{
      path = "{}";
      browseable = {};
      "read only" = {};
      "guest ok" = {};
      "force user" = "{}";
      "force group" = "{}";
    }};"#,
                self.name,
                self.path,
                if self.browsable { "yes" } else { "no" },
                if self.read_only { "yes" } else { "no" },
                if self.guest_ok { "yes" } else { "no" },
                self.force_user,
                self.force_group
            );

            // Insert the updated share at the same position
            lines.insert(start, share_config);
        } else {
            return Err(format!("Share '{}' not found in configuration", old_name));
        }

        // Write back to the file
        let content = lines.join("\n");
        fs::write(Self::CONFIG_PATH, content)
            .map_err(|e| format!("Failed to write to {}: {}", Self::CONFIG_PATH, e))?;

        Ok(())
    }
}

/// Get list of system users
pub fn get_system_users() -> Vec<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("cut -d: -f1 /etc/passwd | sort")
        .output();

    if let Ok(output) = output {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        vec!["root".to_string(), "nobody".to_string()]
    }
}

/// Get list of system groups
pub fn get_system_groups() -> Vec<String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("cut -d: -f1 /etc/group | sort")
        .output();

    if let Ok(output) = output {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|s| s.to_string())
            .collect()
    } else {
        vec!["root".to_string(), "nogroup".to_string()]
    }
}
 