use crate::samba::sudo_write::write_with_sudo;
use rnix::{Root, SyntaxKind, SyntaxNode};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone)]
pub struct RemoteSambaShareConfig {
    pub name: String,
    pub remote_path: String,
    pub fs_type: String,
    pub option_credentials: String,
    pub force_user: String,
    pub force_group: String,
}

impl RemoteSambaShareConfig {
    /// Path to the NixOS configuration file
    const CONFIG_PATH: &'static str = "/etc/nixos/customConfig/default.nix";

    pub fn new(
        name: String,
        remote_path: String,
        fs_type: String,
        option_credentials: String,
        force_user: String,
        force_group: String,
    ) -> Self {
        Self {
            name,
            remote_path,
            fs_type,
            option_credentials,
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

        // Search recursively for fileSystems."/mount/point" entries
        find_filesystem_entries(&root, &mut shares);

        Ok(shares)
    }

    /// Write a new remote filesystem configuration to NixOS
    pub fn write(&self) -> Result<(), String> {
        let mut content = fs::read_to_string(Self::CONFIG_PATH)
            .map_err(|e| format!("Failed to read {}: {}", Self::CONFIG_PATH, e))?;

        // Build the options list
        let mut options = Vec::new();
        if !self.option_credentials.is_empty() {
            options.push(format!("\"credentials={}\"", self.option_credentials));
        }
        options.push("\"x-systemd.automount\"".to_string());
        options.push("\"noauto\"".to_string());
        options.push("\"x-systemd.idle-timeout=300\"".to_string());
        options.push("\"x-systemd.device-timeout=10s\"".to_string());
        options.push("\"x-systemd.mount-timeout=10s\"".to_string());
        if !self.force_user.is_empty() {
            options.push(format!("\"uid={}\"", self.force_user));
        }
        if !self.force_group.is_empty() {
            options.push(format!("\"gid={}\"", self.force_group));
        }

        // Build the new entry
        let new_entry = format!(
            r#"fileSystems."{}" = {{
  device = "{}";
  fsType = "{}";
  options = [
    {}
  ];
}};

"#,
            self.name,
            self.remote_path,
            self.fs_type,
            options.join("\n    ")
        );

        // Find where to insert (before the closing brace of the module)
        // Look for the last closing brace
        if let Some(last_brace_pos) = content.rfind('}') {
            content.insert_str(last_brace_pos, &new_entry);
        } else {
            return Err("Could not find insertion point in config file".to_string());
        }

        // Write back to file with sudo
        write_with_sudo(Self::CONFIG_PATH, &content)?;

        Ok(())
    }

    /// Update an existing remote filesystem configuration
    pub fn update(&self, old_name: &str) -> Result<(), String> {
        let mut content = fs::read_to_string(Self::CONFIG_PATH)
            .map_err(|e| format!("Failed to read {}: {}", Self::CONFIG_PATH, e))?;

        // If name hasn't changed, update in place
        // Otherwise, delete old entry and add new one
        if old_name == self.name {
            // Update in place using regex with multiline flag
            // This pattern matches the entire fileSystems entry including nested braces
            let pattern = format!(
                r#"(?s)fileSystems\."{}"\s*=\s*\{{.*?\}};"#,
                regex::escape(old_name)
            );

            let re = regex::Regex::new(&pattern)
                .map_err(|e| format!("Failed to create regex: {}", e))?;

            if !re.is_match(&content) {
                return Err(format!("Could not find filesystem entry for '{}'", old_name));
            }

            // Build the options list
            let mut options = Vec::new();
            if !self.option_credentials.is_empty() {
                options.push(format!("\"credentials={}\"", self.option_credentials));
            }
            options.push("\"x-systemd.automount\"".to_string());
            options.push("\"noauto\"".to_string());
            options.push("\"x-systemd.idle-timeout=300\"".to_string());
            options.push("\"x-systemd.device-timeout=10s\"".to_string());
            options.push("\"x-systemd.mount-timeout=10s\"".to_string());
            if !self.force_user.is_empty() {
                options.push(format!("\"uid={}\"", self.force_user));
            }
            if !self.force_group.is_empty() {
                options.push(format!("\"gid={}\"", self.force_group));
            }

            // Build the replacement entry
            let replacement = format!(
                r#"fileSystems."{}" = {{
  device = "{}";
  fsType = "{}";
  options = [
    {}
  ];
}};"#,
                self.name,
                self.remote_path,
                self.fs_type,
                options.join("\n    ")
            );

            content = re.replace(&content, replacement.as_str()).to_string();
        } else {
            // Name changed - delete old and add new
            self.delete(old_name)?;
            return self.write();
        }

        // Write back to file with sudo
        write_with_sudo(Self::CONFIG_PATH, &content)?;

        Ok(())
    }

    /// Delete a remote filesystem configuration
    fn delete(&self, name: &str) -> Result<(), String> {
        let mut content = fs::read_to_string(Self::CONFIG_PATH)
            .map_err(|e| format!("Failed to read {}: {}", Self::CONFIG_PATH, e))?;

        // Delete the entry using regex with multiline flag
        // (?s) enables dotall mode where . matches newlines
        let pattern = format!(
            r#"(?s)fileSystems\."{}"\s*=\s*\{{.*?\}};[\n\r]*"#,
            regex::escape(name)
        );

        let re = regex::Regex::new(&pattern)
            .map_err(|e| format!("Failed to create regex: {}", e))?;

        if !re.is_match(&content) {
            return Err(format!("Could not find filesystem entry for '{}'", name));
        }

        content = re.replace(&content, "").to_string();

        // Write back to file with sudo
        write_with_sudo(Self::CONFIG_PATH, &content)?;

        Ok(())
    }
}

/// Recursively find all fileSystems entries in the AST
/// Each entry is like: fileSystems."/media/blender" = { device = ...; fsType = ...; options = [...]; };
fn find_filesystem_entries(node: &SyntaxNode, shares: &mut Vec<RemoteSambaShareConfig>) {
    // Look for NODE_ATTRPATH_VALUE nodes
    if node.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
        // Check if this attrpath starts with "fileSystems"
        for child in node.children() {
            if child.kind() == SyntaxKind::NODE_ATTRPATH {
                // Get the first identifier in the attrpath
                let mut is_filesystems = false;
                let mut mount_point = String::new();

                for attrpath_child in child.children() {
                    if attrpath_child.kind() == SyntaxKind::NODE_IDENT {
                        let ident_text = attrpath_child.text().to_string();
                        if ident_text == "fileSystems" {
                            is_filesystems = true;
                        }
                    } else if attrpath_child.kind() == SyntaxKind::NODE_STRING {
                        // This is the mount point (e.g., "/media/blender")
                        let text = attrpath_child.text().to_string();
                        mount_point = text.trim_matches('"').to_string();
                    }
                }

                // If this is a fileSystems entry, parse its value
                if is_filesystems && !mount_point.is_empty() {
                    // Find the attr set value
                    for value_child in node.children() {
                        if value_child.kind() == SyntaxKind::NODE_ATTR_SET {
                            // Parse device, fsType, options from the attr set
                            let mut device = String::new();
                            let mut fs_type = String::new();
                            let mut options_list: Vec<String> = Vec::new();

                            for entry in value_child.children() {
                                if entry.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
                                    if let Some(key) = get_attrpath_name(&entry) {
                                        match key.as_str() {
                                            "device" => {
                                                device = get_attrvalue(&entry).unwrap_or_default();
                                            }
                                            "fsType" => {
                                                fs_type = get_attrvalue(&entry).unwrap_or_default();
                                            }
                                            "options" => {
                                                options_list = get_attrvalue_list(&entry).unwrap_or_default();
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }

                            // Only process CIFS/SMB shares
                            if fs_type == "cifs" {
                                // Extract credentials from options
                                let credentials = options_list
                                    .iter()
                                    .find(|opt| opt.starts_with("credentials="))
                                    .map(|opt| {
                                        opt.strip_prefix("credentials=").unwrap_or("").to_string()
                                    })
                                    .unwrap_or_default();

                                // Extract uid and gid from options
                                let uid = options_list
                                    .iter()
                                    .find(|opt| opt.starts_with("uid="))
                                    .and_then(|opt| opt.strip_prefix("uid="))
                                    .unwrap_or("1000");

                                let gid = options_list
                                    .iter()
                                    .find(|opt| opt.starts_with("gid="))
                                    .and_then(|opt| opt.strip_prefix("gid="))
                                    .unwrap_or("100");

                                shares.push(RemoteSambaShareConfig {
                                    name: mount_point.clone(),
                                    remote_path: device,
                                    fs_type,
                                    option_credentials: credentials,
                                    force_user: uid.to_string(),
                                    force_group: gid.to_string(),
                                });
                            }
                        }
                    }
                }
                break; // Only need to check the first ATTRPATH child
            }
        }
    }

    // Recursively search children
    for child in node.children() {
        find_filesystem_entries(&child, shares);
    }
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

/// Get a list value from an ATTRPATH_VALUE node
/// Returns a Vec of strings representing the list items
fn get_attrvalue_list(node: &SyntaxNode) -> Option<Vec<String>> {
    for child in node.children() {
        if child.kind() == SyntaxKind::NODE_LIST {
            let mut items = Vec::new();
            for list_child in child.children() {
                match list_child.kind() {
                    SyntaxKind::NODE_STRING => {
                        let text = list_child.text().to_string();
                        items.push(text.trim().trim_matches('"').to_string());
                    }
                    SyntaxKind::NODE_IDENT => {
                        items.push(list_child.text().to_string());
                    }
                    _ => {}
                }
            }
            return Some(items);
        }
    }
    None
}
