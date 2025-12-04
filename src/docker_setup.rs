use crate::config::Config;
use anyhow::{anyhow, Result};
use std::fs;
use std::process::Command;

// Changed name to force a fresh container creation if the old one was stale/broken
const CONTAINER_NAME: &str = "agerus_sandbox";

pub fn ensure_docker_env(config: &Config) -> Result<()> {
    let workspace_path = &config.workspace_path;

    // 1. Create the workspace directory locally if it doesn't exist
    if !workspace_path.exists() {
        fs::create_dir_all(workspace_path)?;
        println!("Created local workspace directory at {:?}", workspace_path);
    }

    // We need the absolute path for Docker volume mounting
    let abs_workspace = fs::canonicalize(workspace_path)?;

    // 2. Check if container is already running
    let status = Command::new("docker")
        .args([
            "ps",
            "--filter",
            &format!("name={}", CONTAINER_NAME),
            "--format",
            "{{.Names}}",
        ])
        .output()?;

    let output = String::from_utf8_lossy(&status.stdout);
    let is_running = output.trim() == CONTAINER_NAME;

    if !is_running {
        // 3. Clean up any stopped container with the same name
        let _ = Command::new("docker")
            .args(["rm", "-f", CONTAINER_NAME])
            .output();

        println!(
            "Starting Docker Sandbox ({}) mapped to: {:?}",
            CONTAINER_NAME, abs_workspace
        );

        // 4. Run the container
        let status = Command::new("docker")
            .arg("run")
            .arg("-d")
            .arg("--name")
            .arg(CONTAINER_NAME)
            // Mount the absolute path of the workspace to /workspace inside the container
            .arg("-v")
            .arg(format!("{}:/workspace", abs_workspace.to_string_lossy()))
            .arg("-w")
            .arg("/workspace")
            .arg("ubuntu:latest")
            .args(["tail", "-f", "/dev/null"])
            .status()?;

        if !status.success() {
            return Err(anyhow!(
                "Failed to start Docker container. Is Docker running?"
            ));
        }
        println!("Docker Sandbox started successfully!");
    }

    // 5. Check if Rust/Cargo is installed inside Docker
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
        println!("Installing Basic Tools + Rust inside Docker... (This may take a minute)");
        let install_cmd = "apt-get update && \
                           apt-get install -y curl git vim nano wget build-essential && \
                           curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y";

        let setup = Command::new("docker")
            .args(["exec", CONTAINER_NAME, "bash", "-c", install_cmd])
            .status()?;

        if !setup.success() {
            eprintln!("Warning: Failed to install tools inside Docker.");
        } else {
            println!("Tools installed successfully.");
        }
    } else {
        println!("Docker environment is ready (Rust is installed).");
    }

    Ok(())
}
