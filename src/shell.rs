use crate::app::AppEvent;
use anyhow::{anyhow, Result};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{mpsc, Mutex};

// Match the new name in docker_setup.rs
const CONTAINER_NAME: &str = "agerus_sandbox";

// Request types for our persistent shell
pub enum ShellRequest {
    RunCommand {
        cmd: String,
        response_tx: mpsc::Sender<String>,
    },
    UserInput(String),
}

pub struct ShellSession {
    process: Child,
    stdin: Option<ChildStdin>,
    reader: Arc<Mutex<BufReader<ChildStdout>>>,
    delimiter: String,
}

impl ShellSession {
    fn new_internal() -> Result<Self> {
        // Fix: Explicitly set working directory (-w /workspace)
        // Fix: Set host current_dir to "/" to avoid OCI namespace path issues
        let mut process = Command::new("docker")
            .current_dir("/") // Critical fix for "outside of container mount namespace" error
            .args([
                "exec",
                "-i",
                "-w",
                "/workspace",
                CONTAINER_NAME,
                "bash",
                "-l",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = process
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to capture stdin"))?;
        let stdout = process
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to capture stdout"))?;

        let delimiter = "__END_OF_CMD__".to_string();

        Ok(Self {
            process,
            stdin: Some(stdin),
            reader: Arc::new(Mutex::new(BufReader::new(stdout))),
            delimiter,
        })
    }

    pub async fn run_actor(
        mut rx_request: mpsc::Receiver<ShellRequest>,
        tx_app_event: mpsc::Sender<AppEvent>,
    ) {
        let mut session = match Self::new_internal() {
            Ok(s) => s,
            Err(e) => {
                let _ = tx_app_event
                    .send(AppEvent::Error(format!("Failed to start shell: {}", e)))
                    .await;
                return;
            }
        };

        let mut current_responder: Option<mpsc::Sender<String>> = None;

        loop {
            tokio::select! {
                Some(req) = rx_request.recv() => {
                    let cmd_str = match req {
                        ShellRequest::RunCommand { cmd, response_tx } => {
                            current_responder = Some(response_tx);
                            cmd
                        },
                        ShellRequest::UserInput(input) => {
                            current_responder = None;
                            input
                        }
                    };

                    if let Some(stdin) = session.stdin.as_mut() {
                        let full_cmd = format!("{{ {}; }} 2>&1; echo {}\n", cmd_str, session.delimiter);
                        if let Err(e) = stdin.write_all(full_cmd.as_bytes()).await {
                            let _ = tx_app_event.send(AppEvent::Error(format!("Stdin error: {}", e))).await;
                        }
                        let _ = stdin.flush().await;
                    }
                }

                result = read_next_line(&session.reader) => {
                    match result {
                        Ok(Some(line)) => {
                            if line.contains(&session.delimiter) {
                                current_responder = None;
                            } else {
                                let clean_line = line.trim_end().to_string();
                                let _ = tx_app_event.send(AppEvent::TerminalLine(clean_line.clone())).await;
                                if let Some(tx) = &current_responder {
                                    let _ = tx.send(clean_line).await;
                                }
                            }
                        }
                        Ok(None) => break,
                        Err(_) => break,
                    }
                }
            }
        }
    }
}

async fn read_next_line(reader: &Arc<Mutex<BufReader<ChildStdout>>>) -> Result<Option<String>> {
    let mut reader = reader.lock().await;
    let mut line = String::new();
    let bytes = reader.read_line(&mut line).await?;
    if bytes == 0 {
        return Ok(None);
    }
    Ok(Some(line))
}
