//! Location detection for local vs remote execution

use anyhow::{Context, Result};
use std::process::{Command, Output, Stdio};

const SHANNON_HOSTNAME: &str = "shannon";

/// Check if we're running on SHANNON itself
pub fn is_local() -> bool {
    hostname::get()
        .map(|h| h.to_string_lossy().to_lowercase() == SHANNON_HOSTNAME)
        .unwrap_or(false)
}

/// Execute a shell command, either locally or via SSH
pub fn execute_shell(cmd: &str) -> Result<Output> {
    if is_local() {
        Command::new("sh")
            .args(["-c", cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute command locally")
    } else {
        Command::new("ssh")
            .args([SHANNON_HOSTNAME, cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .context("Failed to execute command via SSH")
    }
}

/// Read a file, either locally or via SSH
pub fn read_file(path: &str) -> Result<String> {
    let output = execute_shell(&format!("cat {}", path))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        anyhow::bail!(
            "Failed to read {}: {}",
            path,
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

/// Write content to a file, either locally or via SSH
pub fn write_file(path: &str, content: &str) -> Result<()> {
    // Escape content for shell
    let escaped = content.replace('\'', "'\\''");
    let cmd = format!("printf '%s' '{}' > {}", escaped, path);
    let output = execute_shell(&cmd)?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "Failed to write {}: {}",
            path,
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

/// Append content to a file
pub fn append_file(path: &str, content: &str) -> Result<()> {
    let escaped = content.replace('\'', "'\\''");
    let cmd = format!("printf '%s' '{}' >> {}", escaped, path);
    let output = execute_shell(&cmd)?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "Failed to append to {}: {}",
            path,
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

/// Run systemctl command
pub fn systemctl(action: &str, service: &str) -> Result<()> {
    let output = execute_shell(&format!("systemctl {} {}", action, service))?;
    if output.status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "systemctl {} {} failed: {}",
            action,
            service,
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_local_returns_bool() {
        // Just verify it doesn't panic
        let _ = is_local();
    }
}
