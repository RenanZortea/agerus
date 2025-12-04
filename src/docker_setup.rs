use crate::config::Config;
use anyhow::{anyhow, Result};
use std::fs;
use std::process::Command;

const CONTAINER_NAME: &str = "agerus_sandbox";

pub fn ensure_docker_env(config: &Config) -> Result<()> {
    // Standard startup check
    setup_container(config, false)
}

pub fn restart_docker_env(config: &Config) -> Result<()> {
    // Force restart
    setup_container(config, true)
}

fn setup_container(config: &Config, force_restart: bool) -> Result<()> {
    let workspace_path = &config.workspace_path;

    if !workspace_path.exists() {
        fs::create_dir_all(workspace_path)?;
    }

    let abs_workspace = fs::canonicalize(workspace_path)?;

    // Check status
    let output = Command::new("docker")
        .args([
            "ps",
            "--filter",
            &format!("name={}", CONTAINER_NAME),
            "--format",
            "{{.Names}}",
        ])
        .output()?; // Use output() to capture stdout

    let output_str = String::from_utf8_lossy(&output.stdout);
    let is_running = output_str.trim() == CONTAINER_NAME;

    if !is_running || force_restart {
        // Kill existing
        let _ = Command::new("docker")
            .args(["rm", "-f", CONTAINER_NAME])
            .output();

        // Start new
        // CHANGED: .status()? -> .output()? to prevent printing Container ID to TUI
        let output = Command::new("docker")
            .arg("run")
            .arg("-d")
            .arg("--name")
            .arg(CONTAINER_NAME)
            .arg("-v")
            .arg(format!("{}:/workspace", abs_workspace.to_string_lossy()))
            .arg("-w")
            .arg("/workspace")
            .arg("ubuntu:latest")
            .args(["tail", "-f", "/dev/null"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to start Docker container."));
        }

        // Only verify/install rust if we actually restarted/created the container
        check_and_install_tools()?;
    }

    Ok(())
}

fn check_and_install_tools() -> Result<()> {
    let cargo_check = Command::new("docker")
        .args([
            "exec",
            CONTAINER_NAME,
            "bash",
            "-l",
            "-c",
            "cargo --version",
        ])
        .output();

    let needs_install = match cargo_check {
        Ok(out) => !out.status.success(),
        Err(_) => true,
    };

    if needs_install {
        // Silent install if possible
        let install_cmd = "apt-get update && \
                           apt-get install -y curl git vim nano wget build-essential && \
                           curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y";

        // CHANGED: .status()? -> .output()? to prevent printing apt-get logs to TUI
        Command::new("docker")
            .args(["exec", CONTAINER_NAME, "bash", "-c", install_cmd])
            .output()?;
    }
    Ok(())
}
