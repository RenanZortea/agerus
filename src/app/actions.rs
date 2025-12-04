use super::{App, AppEvent, AppMode, MessageRole};
use crate::docker_setup;
use crate::mcp::McpServer;
use crate::shell::{ShellRequest, ShellSession};
use chrono::Local;
use std::fs;
use std::path::PathBuf;
use tokio::sync::mpsc;

impl App {
    pub fn reload_sessions(&mut self) {
        if let Ok(mut list) = self.session_manager.list_sessions() {
            list.sort_by(|a, b| b.cmp(a));
            self.sessions = list;
        }
    }

    // --- Workspace ---

    pub fn change_workspace(&mut self, new_path_str: String) {
        // 1. Handle Tilde Expansion
        let new_path = if new_path_str.starts_with("~") {
            if let Some(home) = dirs::home_dir() {
                if new_path_str == "~" {
                    home
                } else if new_path_str.starts_with("~/") || new_path_str.starts_with("~\\") {
                    // Strip "~/" (first 2 chars) and join with home
                    home.join(&new_path_str[2..])
                } else {
                    // Case like "~username" (not supported simply) or just "~" in a weird place
                    PathBuf::from(&new_path_str)
                }
            } else {
                PathBuf::from(&new_path_str)
            }
        } else {
            PathBuf::from(&new_path_str)
        };

        // 2. Resolve Path (Handle relative vs absolute)
        let resolved_path = if new_path.is_absolute() {
            new_path
        } else {
            // If relative, join with the CURRENT workspace path
            self.config.workspace_path.join(new_path)
        };

        // 3. Validate existence
        if !resolved_path.exists() {
            self.add_system_message(
                format!("Path does not exist: {:?}", resolved_path),
                MessageRole::Error,
            );
            return;
        }

        // 4. Update Config with Canonical Path
        self.config.workspace_path = match fs::canonicalize(&resolved_path) {
            Ok(p) => p,
            Err(e) => {
                self.add_system_message(format!("Invalid path: {}", e), MessageRole::Error);
                return;
            }
        };

        self.add_system_message(
            format!("Switching workspace to: {:?}", self.config.workspace_path),
            MessageRole::System,
        );
        self.add_system_message(
            "Restarting Docker sandbox... (this may take a moment)".into(),
            MessageRole::Thinking,
        );

        let config_clone = self.config.clone();
        let event_tx_clone = self.event_tx.clone();

        tokio::spawn(async move {
            if let Err(e) = docker_setup::restart_docker_env(&config_clone) {
                let _ = event_tx_clone
                    .send(AppEvent::Error(format!("Failed to restart Docker: {}", e)))
                    .await;
                return;
            }

            let (tx_shell, rx_shell) = mpsc::channel::<ShellRequest>(100);
            let tx_shell_for_app = tx_shell.clone();
            let tx_shell_for_mcp = tx_shell.clone();
            let tx_app_for_shell = event_tx_clone.clone();

            tokio::spawn(async move {
                ShellSession::run_actor(rx_shell, tx_app_for_shell).await;
            });

            let tx_mcp = McpServer::start(tx_shell_for_mcp, config_clone).await;
            let _ = event_tx_clone
                .send(AppEvent::WorkspaceRestarted(tx_shell_for_app, tx_mcp))
                .await;
        });

        let _ = self.config.save();
    }

    // --- Models ---

    pub fn open_model_selector(&mut self) {
        self.last_mode = if self.mode == AppMode::ModelSelector {
            AppMode::Chat
        } else {
            self.mode.clone()
        };
        self.mode = AppMode::ModelSelector;

        let tx = self.event_tx.clone();
        let url = self.config.ollama_url.replace("/api/chat", "/api/tags");

        tokio::spawn(async move {
            match reqwest::get(&url).await {
                Ok(resp) => {
                    if let Ok(json) = resp.json::<serde_json::Value>().await {
                        if let Some(models) = json.get("models").and_then(|v| v.as_array()) {
                            let names: Vec<String> = models
                                .iter()
                                .filter_map(|m| {
                                    m.get("name")
                                        .and_then(|n| n.as_str())
                                        .map(|s| s.to_string())
                                })
                                .collect();
                            let _ = tx.send(AppEvent::ModelsLoaded(names)).await;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(AppEvent::Error(format!("Failed to fetch models: {}", e)))
                        .await;
                }
            }
        });
    }

    pub fn select_next_model(&mut self) {
        if self.available_models.is_empty() {
            return;
        }
        let i = match self.model_list_state.selected() {
            Some(i) => (i + 1) % self.available_models.len(),
            None => 0,
        };
        self.model_list_state.select(Some(i));
    }

    pub fn select_prev_model(&mut self) {
        if self.available_models.is_empty() {
            return;
        }
        let i = match self.model_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.available_models.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.model_list_state.select(Some(i));
    }

    pub fn set_default_model(&mut self) {
        if let Some(i) = self.model_list_state.selected() {
            if let Some(model) = self.available_models.get(i) {
                self.config.model = model.clone();
                if let Err(e) = self.config.save() {
                    self.add_system_message(
                        format!("Failed to save config: {}", e),
                        MessageRole::Error,
                    );
                } else {
                    self.add_system_message(
                        format!("Default model set to: {}", model),
                        MessageRole::System,
                    );
                }
            }
        }
    }

    pub fn confirm_model_selection(&mut self) {
        if let Some(i) = self.model_list_state.selected() {
            if let Some(model) = self.available_models.get(i) {
                self.config.model = model.clone();
                self.add_system_message(
                    format!("Switched to model: {}", model),
                    MessageRole::System,
                );
            }
        }
        self.mode = self.last_mode.clone();
    }

    // --- Sessions ---

    pub fn save_current_session(&mut self) {
        match self
            .session_manager
            .save_session(&self.current_session, &self.messages)
        {
            Ok(_) => {
                self.reload_sessions();
            }
            Err(e) => {
                self.add_system_message(format!("Auto-save failed: {}", e), MessageRole::Error);
            }
        }
    }

    pub fn load_session_by_name(&mut self, name: String) {
        match self.session_manager.load_session(&name) {
            Ok(msgs) => {
                self.messages = msgs;
                self.current_session = name;
                self.chat_stick_to_bottom = true;
                self.add_system_message(
                    format!("Session '{}' loaded.", self.current_session),
                    MessageRole::System,
                );
                self.reload_sessions();
            }
            Err(e) => {
                self.add_system_message(format!("Failed to load: {}", e), MessageRole::Error);
            }
        }
    }

    pub fn start_new_session(&mut self, name_opt: Option<String>) {
        let name = name_opt
            .unwrap_or_else(|| format!("chat_{}", Local::now().format("%Y-%m-%d_%H-%M-%S")));
        self.messages.clear();
        self.current_session = name;
        self.add_system_message(
            format!(
                "New Session: {}. Model: {}",
                self.current_session, self.config.model
            ),
            MessageRole::System,
        );
        self.save_current_session();
    }

    pub fn abort_agent(&mut self) {
        if let Some(task) = self.agent_task.take() {
            task.abort();
        }
        self.is_processing = false;
        self.add_system_message("🛑 Cancelled by user.".into(), MessageRole::System);
        self.save_current_session();
    }
}
