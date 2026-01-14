use serde::Deserialize;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use users::{get_current_gid, get_current_uid};

/// Represents a mounted CIFS/SMB share
#[derive(Debug, Clone, Deserialize)]
pub struct MountedShare {
    pub source: String,      // //server/share
    pub target: String,      // /media/blender
    pub fstype: String,      // cifs
    pub options: String,     // rw,credentials=...,uid=1000
    #[serde(default)]
    pub is_mounted: bool,
}

/// Options for mounting a CIFS share
#[derive(Debug, Clone)]
pub struct MountOptions {
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub additional_opts: Vec<String>,
}

impl Default for MountOptions {
    fn default() -> Self {
        Self {
            uid: Some(get_current_uid()),
            gid: Some(get_current_gid()),
            additional_opts: vec![
                "x-systemd.automount".to_string(),
                "noauto".to_string(),
                "x-systemd.idle-timeout=300".to_string(),
            ],
        }
    }
}

/// RAII guard for temporary credentials file
/// Automatically deletes the file when dropped
struct CredentialsFile {
    path: PathBuf,
}

impl CredentialsFile {
    /// Create a new credentials file with secure permissions
    fn new(username: &str, password: &str) -> Result<Self, String> {
        // Create unique filename using process ID and timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let path = PathBuf::from(format!("/tmp/smb_creds_{}_{}", std::process::id(), timestamp));

        // Write credentials
        let content = format!("username={}\npassword={}\n", username, password);
        fs::write(&path, content)
            .map_err(|e| format!("Failed to create credentials file: {}", e))?;

        // Set permissions to 0600 (owner read/write only)
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set credentials file permissions: {}", e))?;

        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for CredentialsFile {
    fn drop(&mut self) {
        // Securely delete the credentials file
        let _ = fs::remove_file(&self.path);
    }
}

/// List all CIFS shares (both configured and currently mounted)
/// Combines NixOS configuration with actual mount status
pub fn list_all_shares() -> Result<Vec<MountedShare>, String> {
    use super::remote_share_config::RemoteSambaShareConfig;
    use std::collections::HashMap;

    // Get configured shares from NixOS config
    let configured = RemoteSambaShareConfig::load_all().unwrap_or_default();

    // Get currently mounted shares from system
    let mounted = list_cifs_mounts().unwrap_or_default();

    // Create a map of mounted shares by target path
    let mounted_map: HashMap<String, &MountedShare> = mounted
        .iter()
        .map(|s| (s.target.clone(), s))
        .collect();

    // Combine configured and mounted shares
    let mut result = Vec::new();

    // Add all configured shares with mount status
    for config in configured {
        let is_mounted = mounted_map.contains_key(&config.name);
        let mounted_share = mounted_map.get(&config.name);

        result.push(MountedShare {
            source: config.remote_path.clone(),
            target: config.name.clone(),
            fstype: config.fs_type.clone(),
            options: if let Some(m) = mounted_share {
                m.options.clone()
            } else {
                // Build options string from config
                let mut opts = vec![
                    format!("credentials={}", config.option_credentials),
                    format!("uid={}", config.force_user),
                    format!("gid={}", config.force_group),
                ];
                opts.join(",")
            },
            is_mounted,
        });
    }

    // Add any mounted shares that aren't in the config
    for share in mounted {
        if !result.iter().any(|s| s.target == share.target) {
            result.push(share);
        }
    }

    Ok(result)
}

/// List all currently mounted CIFS shares from the system
pub fn list_cifs_mounts() -> Result<Vec<MountedShare>, String> {
    // Try using findmnt with JSON output first
    if let Ok(shares) = list_cifs_mounts_findmnt() {
        return Ok(shares);
    }

    // Fallback to parsing /proc/mounts
    list_cifs_mounts_proc()
}

/// List CIFS mounts using findmnt command (preferred method)
fn list_cifs_mounts_findmnt() -> Result<Vec<MountedShare>, String> {
    let output = Command::new("findmnt")
        .args(&["-t", "cifs", "--json", "-o", "SOURCE,TARGET,FSTYPE,OPTIONS"])
        .output()
        .map_err(|e| format!("Failed to run findmnt: {}", e))?;

    if !output.status.success() {
        return Err("findmnt command failed".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    #[derive(Deserialize)]
    struct FindmntOutput {
        filesystems: Vec<FindmntFilesystem>,
    }

    #[derive(Deserialize)]
    struct FindmntFilesystem {
        source: String,
        target: String,
        fstype: String,
        options: String,
    }

    let parsed: FindmntOutput = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse findmnt JSON: {}", e))?;

    Ok(parsed
        .filesystems
        .into_iter()
        .map(|fs| MountedShare {
            source: fs.source,
            target: fs.target,
            fstype: fs.fstype,
            options: fs.options,
            is_mounted: true,
        })
        .collect())
}

/// List CIFS mounts by parsing /proc/mounts (fallback method)
fn list_cifs_mounts_proc() -> Result<Vec<MountedShare>, String> {
    let content = fs::read_to_string("/proc/mounts")
        .map_err(|e| format!("Failed to read /proc/mounts: {}", e))?;

    let mut shares = Vec::new();

    for line in content.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 && parts[2] == "cifs" {
            shares.push(MountedShare {
                source: parts[0].to_string(),
                target: parts[1].to_string(),
                fstype: parts[2].to_string(),
                options: parts[3].to_string(),
                is_mounted: true,
            });
        }
    }

    Ok(shares)
}

/// Check if a specific mount point is currently mounted
pub fn is_mounted(mount_point: &Path) -> bool {
    if let Ok(shares) = list_cifs_mounts() {
        shares.iter().any(|s| Path::new(&s.target) == mount_point)
    } else {
        false
    }
}

/// Mount a CIFS/SMB share
///
/// # Arguments
/// * `remote_url` - The SMB share path (e.g., "//server/share")
/// * `mount_point` - Local directory to mount to
/// * `username` - SMB username
/// * `password` - SMB password
/// * `options` - Additional mount options
///
/// # Security
/// - Credentials are written to a temporary file with 0600 permissions
/// - The credentials file is automatically deleted after mounting
/// - Never passes passwords via command line arguments
pub fn mount_share(
    remote_url: &str,
    mount_point: &Path,
    username: &str,
    password: &str,
    options: MountOptions,
) -> Result<(), String> {
    // Validate inputs
    validate_remote_url(remote_url)?;
    validate_mount_point(mount_point)?;

    // Check if already mounted
    if is_mounted(mount_point) {
        return Err(format!(
            "Mount point {} is already mounted",
            mount_point.display()
        ));
    }

    // Create mount point directory if it doesn't exist
    if !mount_point.exists() {
        fs::create_dir_all(mount_point)
            .map_err(|e| format!("Failed to create mount point directory: {}", e))?;
    }

    // Create temporary credentials file (auto-deleted on drop)
    let creds_file = CredentialsFile::new(username, password)?;

    // Build mount options
    let mut mount_opts = vec![
        format!("credentials={}", creds_file.path().display()),
        format!("uid={}", options.uid.unwrap_or_else(get_current_uid)),
        format!("gid={}", options.gid.unwrap_or_else(get_current_gid)),
    ];
    mount_opts.extend(options.additional_opts);

    // Execute mount command
    let output = Command::new("mount")
        .arg("-t")
        .arg("cifs")
        .arg(remote_url)
        .arg(mount_point)
        .arg("-o")
        .arg(mount_opts.join(","))
        .output()
        .map_err(|e| format!("Failed to execute mount command: {}", e))?;

    // Check if mount succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(parse_mount_error(&stderr));
    }

    Ok(())
}

/// Unmount a CIFS/SMB share
///
/// # Arguments
/// * `mount_point` - The mount point to unmount
pub fn unmount_share(mount_point: &Path) -> Result<(), String> {
    // Check if it's actually mounted
    if !is_mounted(mount_point) {
        return Err(format!(
            "Mount point {} is not currently mounted",
            mount_point.display()
        ));
    }

    // Execute umount command
    let output = Command::new("umount")
        .arg(mount_point)
        .output()
        .map_err(|e| format!("Failed to execute umount command: {}", e))?;

    // Check if unmount succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(parse_umount_error(&stderr));
    }

    Ok(())
}

/// Validate remote URL format
fn validate_remote_url(url: &str) -> Result<(), String> {
    if !url.starts_with("//") {
        return Err("Remote URL must start with '//' (e.g., //server/share)".to_string());
    }

    if url.matches('/').count() < 3 {
        return Err("Remote URL must include server and share name (e.g., //server/share)".to_string());
    }

    // Check for potential command injection
    if url.contains(';') || url.contains('&') || url.contains('|') || url.contains('`') {
        return Err("Remote URL contains invalid characters".to_string());
    }

    Ok(())
}

/// Validate mount point path
fn validate_mount_point(path: &Path) -> Result<(), String> {
    // Must be absolute path
    if !path.is_absolute() {
        return Err("Mount point must be an absolute path".to_string());
    }

    // Check for potential command injection in path
    let path_str = path.to_string_lossy();
    if path_str.contains(';')
        || path_str.contains('&')
        || path_str.contains('|')
        || path_str.contains('`')
    {
        return Err("Mount point path contains invalid characters".to_string());
    }

    Ok(())
}

/// Parse mount command error messages into user-friendly errors
fn parse_mount_error(stderr: &str) -> String {
    let lower = stderr.to_lowercase();

    if lower.contains("permission denied") || lower.contains("access denied") {
        "Permission denied. Check your credentials or run with sudo.".to_string()
    } else if lower.contains("connection refused") || lower.contains("could not resolve") {
        "Connection refused. Server may be offline or unreachable.".to_string()
    } else if lower.contains("already mounted") || lower.contains("busy") {
        "Mount point is already in use or mounted.".to_string()
    } else if lower.contains("no such file or directory") {
        "Server or share not found. Check the remote URL.".to_string()
    } else if lower.contains("invalid argument") {
        "Invalid mount options. Check your configuration.".to_string()
    } else if lower.contains("host is down") {
        "Host is unreachable. Check network connectivity.".to_string()
    } else {
        format!("Mount failed: {}", stderr.trim())
    }
}

/// Parse unmount command error messages into user-friendly errors
fn parse_umount_error(stderr: &str) -> String {
    let lower = stderr.to_lowercase();

    if lower.contains("not mounted") {
        "The specified path is not currently mounted.".to_string()
    } else if lower.contains("busy") || lower.contains("target is busy") {
        "Mount point is busy. Close any programs using files from this share.".to_string()
    } else if lower.contains("permission denied") {
        "Permission denied. You may need to run with sudo.".to_string()
    } else {
        format!("Unmount failed: {}", stderr.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_remote_url() {
        assert!(validate_remote_url("//server/share").is_ok());
        assert!(validate_remote_url("//192.168.1.100/data").is_ok());
        assert!(validate_remote_url("server/share").is_err());
        assert!(validate_remote_url("//server").is_err());
        assert!(validate_remote_url("//server/share;rm -rf").is_err());
    }

    #[test]
    fn test_validate_mount_point() {
        assert!(validate_mount_point(Path::new("/mnt/share")).is_ok());
        assert!(validate_mount_point(Path::new("relative/path")).is_err());
        assert!(validate_mount_point(Path::new("/mnt/share;whoami")).is_err());
    }
}
