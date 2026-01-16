use std::fs;
use std::process::Command;

/// Write content to a file that requires root privileges.
/// Tries multiple methods for privilege escalation.
pub fn write_with_sudo(path: &str, content: &str) -> Result<(), String> {
    // First, try to write directly (in case we already have permissions)
    if fs::write(path, content).is_ok() {
        return Ok(());
    }

    // Create a temporary file with the content
    let temp_path = format!("/tmp/samba_share_config_{}.tmp", std::process::id());

    fs::write(&temp_path, content)
        .map_err(|e| format!("Failed to write temporary file: {}", e))?;

    // Try method 1: NixOS wrapped pkexec (if available)
    if let Ok(output) = Command::new("/run/wrappers/bin/pkexec")
        .args(["cp", &temp_path, path])
        .output()
    {
        if output.status.success() {
            let _ = fs::remove_file(&temp_path);
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("dismissed") || stderr.contains("Not authorized") {
            let _ = fs::remove_file(&temp_path);
            return Err("Authorization cancelled by user".to_string());
        }
    }

    // Try method 2: run0 (systemd's modern privilege escalation, available in systemd 256+)
    if let Ok(output) = Command::new("run0")
        .args(["cp", &temp_path, path])
        .output()
    {
        if output.status.success() {
            let _ = fs::remove_file(&temp_path);
            return Ok(());
        }
    }

    // Try method 3: Regular pkexec (might work if setuid is configured)
    if let Ok(output) = Command::new("pkexec")
        .args(["cp", &temp_path, path])
        .output()
    {
        if output.status.success() {
            let _ = fs::remove_file(&temp_path);
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("dismissed") || stderr.contains("Not authorized") {
            let _ = fs::remove_file(&temp_path);
            return Err("Authorization cancelled by user".to_string());
        }
    }

    // Try method 4: sudo (may work if user has NOPASSWD or cached credentials)
    if let Ok(output) = Command::new("sudo")
        .args(["-n", "cp", &temp_path, path])
        .output()
    {
        if output.status.success() {
            let _ = fs::remove_file(&temp_path);
            return Ok(());
        }
    }

    // Clean up temp file
    let _ = fs::remove_file(&temp_path);

    // Provide a helpful error message for NixOS users
    Err(
        "Failed to write file with elevated privileges.\n\n\
        On NixOS, you need to enable polkit in your configuration:\n\n\
        security.polkit.enable = true;\n\n\
        Then rebuild with: sudo nixos-rebuild switch\n\n\
        Alternatively, run the application with sudo or manually edit the file."
            .to_string(),
    )
}

/// Read a file (doesn't need sudo, but included for completeness)
pub fn read_file(path: &str) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path, e))
}
