use rnix::{Root, SyntaxKind, SyntaxNode};
use std::collections::HashMap;
use std::fs;
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

    /// Load all Samba shares from NixOS configuration using rnix parser
    pub fn load_all() -> Result<Vec<Self>, String> {
        let content = fs::read_to_string(Self::CONFIG_PATH)
            .map_err(|e| format!("Failed to read {}: {}", Self::CONFIG_PATH, e))?;

        let parsed = Root::parse(&content);
        let root = parsed.syntax();

        let mut shares = Vec::new();

        // Find services.samba.settings attrset
        if let Some(settings_attrset) = find_samba_settings(&root) {
            // Iterate through all entries in the settings attrset
            for child in settings_attrset.children() {
                if child.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
                    if let Some((name, props)) = parse_attrset_entry(&child) {
                        // Skip the "global" section
                        if name != "global" {
                            shares.push(SambaShareConfig {
                                name,
                                path: props.get("path").cloned().unwrap_or_default(),
                                browsable: props
                                    .get("browseable")
                                    .map(|v| v == "yes")
                                    .unwrap_or(true),
                                read_only: props
                                    .get("read only")
                                    .map(|v| v == "yes")
                                    .unwrap_or(false),
                                guest_ok: props
                                    .get("guest ok")
                                    .map(|v| v == "yes")
                                    .unwrap_or(false),
                                force_user: props.get("force user").cloned().unwrap_or_default(),
                                force_group: props.get("force group").cloned().unwrap_or_default(),
                            });
                        }
                    }
                }
            }
        }

        Ok(shares)
    }

    /// Write a new Samba share configuration to NixOS
    pub fn write(&self) -> Result<(), String> {
        let content = fs::read_to_string(Self::CONFIG_PATH)
            .map_err(|e| format!("Failed to read {}: {}", Self::CONFIG_PATH, e))?;

        // Parse to validate syntax
        let parsed = Root::parse(&content);
        if !parsed.errors().is_empty() {
            return Err("Configuration file has syntax errors".to_string());
        }

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

        let root = parsed.syntax();

        // Find the settings attrset to determine insertion point
        if let Some(settings_attrset) = find_samba_settings(&root) {
            // Get the text range of the settings attrset
            let range = settings_attrset.text_range();
            let settings_end: usize = range.end().into();

            // Find the position just before the closing brace
            // We need to insert before the last }
            let before_closing = content[..settings_end]
                .rfind('}')
                .ok_or("Could not find closing brace of settings section")?;

            let before = &content[..before_closing];
            let after = &content[before_closing..];
            let new_content = format!("{}\n{}\n{}", before, share_config, after);

            fs::write(Self::CONFIG_PATH, new_content)
                .map_err(|e| format!("Failed to write {}: {}", Self::CONFIG_PATH, e))?;
        } else {
            // No settings section exists, create entire samba section
            let lines: Vec<&str> = content.lines().collect();
            let mut insert_idx = None;

            for (i, line) in lines.iter().enumerate().rev() {
                if line.trim() == "}" {
                    insert_idx = Some(i);
                    break;
                }
            }

            if let Some(idx) = insert_idx {
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

                let mut new_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
                new_lines.insert(idx, samba_section);
                let new_content = new_lines.join("\n");

                fs::write(Self::CONFIG_PATH, new_content)
                    .map_err(|e| format!("Failed to write {}: {}", Self::CONFIG_PATH, e))?;
            } else {
                return Err(
                    "Could not find suitable location to add services.samba section".to_string(),
                );
            }
        }

        Ok(())
    }

    /// Update an existing Samba share configuration
    pub fn update(&self, old_name: &str) -> Result<(), String> {
        let content = fs::read_to_string(Self::CONFIG_PATH)
            .map_err(|e| format!("Failed to read {}: {}", Self::CONFIG_PATH, e))?;

        let parsed = Root::parse(&content);
        let root = parsed.syntax();

        // Find the settings attrset
        if let Some(settings_attrset) = find_samba_settings(&root) {
            // Find the specific share entry
            for child in settings_attrset.children() {
                if child.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
                    if let Some(name) = get_attrpath_name(&child) {
                        if name == old_name {
                            // Found the share to update
                            let range = child.text_range();
                            let start: usize = range.start().into();
                            let end: usize = range.end().into();

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

                            // Replace the old share with the new one
                            let before = &content[..start];
                            let after = &content[end..];
                            let new_content = format!("{}{}{}", before, share_config, after);

                            fs::write(Self::CONFIG_PATH, new_content).map_err(|e| {
                                format!("Failed to write {}: {}", Self::CONFIG_PATH, e)
                            })?;

                            return Ok(());
                        }
                    }
                }
            }
        }

        Err(format!("Share '{}' not found in configuration", old_name))
    }
}

/// Find the services.samba.settings attrset node
fn find_samba_settings(node: &SyntaxNode) -> Option<SyntaxNode> {
    // Recursively search for services.samba.settings
    for child in node.children() {
        // Look for ATTRPATH_VALUE nodes
        if child.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
            // Check if this attrpath contains "services" and "samba"
            for path_child in child.children() {
                if path_child.kind() == SyntaxKind::NODE_ATTRPATH {
                    let path_text = path_child.text().to_string();
                    // Check if this is services.samba
                    if path_text.contains("services") && path_text.contains("samba") {
                        // Found services.samba, now look for settings inside its attrset
                        for value_child in child.children() {
                            if value_child.kind() == SyntaxKind::NODE_ATTR_SET {
                                // Look for the "settings" entry inside this attrset
                                if let Some(settings_attrset) =
                                    find_direct_attrset(&value_child, "settings")
                                {
                                    return Some(settings_attrset);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Recursively search
        if let Some(found) = find_samba_settings(&child) {
            return Some(found);
        }
    }

    None
}

/// Find a direct child attrset by name (not nested deeper)
fn find_direct_attrset(parent_attrset: &SyntaxNode, name: &str) -> Option<SyntaxNode> {
    for child in parent_attrset.children() {
        if child.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
            // Check if this entry has the name we're looking for
            for path_child in child.children() {
                if path_child.kind() == SyntaxKind::NODE_ATTRPATH {
                    let path_text = path_child.text().to_string().trim().to_string();
                    if path_text == name {
                        // Return the ATTR_SET that is the value of this entry
                        for value_child in child.children() {
                            if value_child.kind() == SyntaxKind::NODE_ATTR_SET {
                                return Some(value_child);
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

/// Get the name from an ATTRPATH_VALUE node
fn get_attrpath_name(node: &SyntaxNode) -> Option<String> {
    for child in node.children() {
        if child.kind() == SyntaxKind::NODE_ATTRPATH {
            // Get all identifiers/strings in the path
            let mut parts = Vec::new();
            for path_part in child.children() {
                match path_part.kind() {
                    SyntaxKind::NODE_IDENT => {
                        parts.push(path_part.text().to_string());
                    }
                    SyntaxKind::NODE_STRING => {
                        let text = path_part.text().to_string();
                        parts.push(text.trim_matches('"').to_string());
                    }
                    _ => {}
                }
            }
            // If it's a single identifier or quoted string, return it
            if parts.len() == 1 {
                return Some(parts[0].clone());
            }
            // If it's a path like "services.samba", join with dot
            return Some(parts.join("."));
        }
    }
    None
}

/// Parse an ATTRPATH_VALUE entry and extract name and properties
fn parse_attrset_entry(node: &SyntaxNode) -> Option<(String, HashMap<String, String>)> {
    let name = get_attrpath_name(node)?;
    let mut props = HashMap::new();

    // Find the ATTR_SET value
    for child in node.children() {
        if child.kind() == SyntaxKind::NODE_ATTR_SET {
            // Parse all entries in this attrset
            for entry_child in child.children() {
                if entry_child.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
                    if let Some(key) = get_attrpath_name(&entry_child) {
                        if let Some(value) = get_attrvalue(&entry_child) {
                            props.insert(key, value);
                        }
                    }
                }
            }
        }
    }

    Some((name, props))
}

/// Get the value from an ATTRPATH_VALUE node
fn get_attrvalue(node: &SyntaxNode) -> Option<String> {
    for child in node.children() {
        match child.kind() {
            SyntaxKind::NODE_STRING => {
                let text = child.text().to_string();
                return Some(text.trim().trim_matches('"').to_string());
            }
            SyntaxKind::NODE_IDENT => {
                return Some(child.text().to_string());
            }
            _ => {}
        }
    }
    None
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
