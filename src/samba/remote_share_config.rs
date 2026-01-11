use rnix::{Root, SyntaxKind, SyntaxNode};
use std::collections::HashMap;
use std::fs;
use std::process::Command;

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

        //fileSystems

        // Find fileSystems attrset
        if let Some(file_systems_attrset) = find_file_systems_root(&root) {
            // Iterate through all filesystem entries
            for child in file_systems_attrset.children() {
                if child.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
                    // Get the mount point (e.g., "/media/blender")
                    if let Some(mount_point) = get_attrpath_name(&child) {
                        // Get the attrset with device, fsType, options
                        let mut device = String::new();
                        let mut fs_type = String::new();
                        let mut options_list: Vec<String> = Vec::new();

                        // Parse the filesystem entry attributes
                        for attr_child in child.children() {
                            if attr_child.kind() == SyntaxKind::NODE_ATTR_SET {
                                for entry in attr_child.children() {
                                    if entry.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
                                        if let Some(key) = get_attrpath_name(&entry) {
                                            match key.as_str() {
                                                "device" => {
                                                    device =
                                                        get_attrvalue(&entry).unwrap_or_default();
                                                }
                                                "fsType" => {
                                                    fs_type =
                                                        get_attrvalue(&entry).unwrap_or_default();
                                                }
                                                "options" => {
                                                    options_list = get_attrvalue_list(&entry)
                                                        .unwrap_or_default();
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Only process CIFS/SMB shares
                        if fs_type == "cifs" {
                            // Extract credentials from options (first item starting with "credentials=")
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
                                name: mount_point,
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
        }

        Ok(shares)
    }

    /// Write a new remote filesystem configuration to NixOS
    pub fn write(&self) -> Result<(), String> {
        // TODO: Implement writing fileSystems entries
        Err("Write operation not yet implemented for remote shares".to_string())
    }

    /// Update an existing remote filesystem configuration
    pub fn update(&self, old_name: &str) -> Result<(), String> {
        // TODO: Implement updating fileSystems entries
        Err("Update operation not yet implemented for remote shares".to_string())
    }
}

/// Find the root fileSystems attrset node
fn find_file_systems_root(node: &SyntaxNode) -> Option<SyntaxNode> {
    // Recursively search for fileSystems
    for child in node.children() {
        // Look for ATTRPATH_VALUE nodes
        if child.kind() == SyntaxKind::NODE_ATTRPATH_VALUE {
            // Check if this attrpath is "fileSystems"
            for path_child in child.children() {
                if path_child.kind() == SyntaxKind::NODE_ATTRPATH {
                    let path_text = path_child.text().to_string().trim().to_string();
                    // Check if this is the root fileSystems
                    if path_text.contains("fileSystems") {
                        // Return the ATTR_SET that contains all filesystem entries

                        return Some(child);
                    }
                }
            }
        }

        // Recursively search
        if let Some(found) = find_file_systems_root(&child) {
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
