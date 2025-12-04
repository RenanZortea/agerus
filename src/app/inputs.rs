use super::{App, AppEvent, AppMode, ChatMessage, MessageRole};
use crate::agent::run_agent_loop;
use crate::shell::ShellRequest;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use std::fs;

impl App {
    pub fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        if self.mode == AppMode::ModelSelector {
            return;
        }
        match mouse.kind {
            MouseEventKind::ScrollDown => self.scroll_down(),
            MouseEventKind::ScrollUp => self.scroll_up(),
            _ => {}
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('p') => {
                    self.open_model_selector();
                    return;
                }
                KeyCode::Char('t') => {
                    // Toggle latest thinking message
                    if let Some(msg) = self
                        .messages
                        .iter_mut()
                        .rev()
                        .find(|m| matches!(m.role, MessageRole::Thinking))
                    {
                        msg.collapsed = !msg.collapsed;
                    }
                    return;
                }
                _ => {}
            }
        }

        match self.mode {
            AppMode::ModelSelector => match key.code {
                KeyCode::Esc => self.mode = self.last_mode.clone(),
                KeyCode::Up => self.select_prev_model(),
                KeyCode::Down => self.select_next_model(),
                KeyCode::Char(' ') => self.set_default_model(),
                KeyCode::Enter => self.confirm_model_selection(),
                _ => {}
            },
            AppMode::Chat | AppMode::Terminal => match key.code {
                KeyCode::Tab => {
                    self.mode = match self.mode {
                        AppMode::Chat => AppMode::Terminal,
                        AppMode::Terminal => AppMode::Chat,
                        _ => AppMode::Chat,
                    };
                }
                KeyCode::Esc if self.is_processing => self.abort_agent(),
                KeyCode::Up => self.scroll_up(),
                KeyCode::Down => self.scroll_down(),
                KeyCode::PageUp => self.scroll_page(-10),
                KeyCode::PageDown => self.scroll_page(10),
                KeyCode::Char(c) if !self.is_processing => self.input_buffer.push(c),
                KeyCode::Backspace if !self.is_processing => {
                    self.input_buffer.pop();
                }
                KeyCode::Enter if !self.is_processing => {
                    if key.modifiers.contains(KeyModifiers::ALT) {
                        self.input_buffer.push('\n');
                    } else {
                        self.submit_message();
                    }
                }
                _ => {}
            },
        }
    }

    pub fn submit_message(&mut self) {
        if self.input_buffer.trim().is_empty() {
            return;
        }
        let text = self.input_buffer.clone();
        self.input_buffer.clear();

        if text.starts_with('/') {
            let parts: Vec<&str> = text.split_whitespace().collect();
            match parts[0] {
                "/new" => {
                    self.start_new_session(parts.get(1).map(|&s| s.to_string()));
                    return;
                }
                "/save" => {
                    self.save_current_session();
                    self.add_system_message(
                        format!("Saved '{}'", self.current_session),
                        MessageRole::System,
                    );
                    return;
                }
                "/load" => {
                    if let Some(name) = parts.get(1) {
                        self.load_session_by_name(name.to_string());
                    } else {
                        self.add_system_message(
                            "Usage: /load <session_name>".into(),
                            MessageRole::Error,
                        );
                    }
                    return;
                }
                "/list" => {
                    self.reload_sessions();
                    let content = format!(
                        "Check sidebar for sessions. ({} total)",
                        self.sessions.len()
                    );
                    self.add_system_message(content, MessageRole::System);
                    return;
                }
                "/model" => {
                    self.open_model_selector();
                    return;
                }
                "/reset" => {
                    self.messages.clear();
                    self.add_system_message("Context reset.".into(), MessageRole::System);
                    return;
                }
                "/add" => {
                    if let Some(filename) = parts.get(1) {
                        let path = self.config.workspace_path.join(filename);
                        match fs::read_to_string(&path) {
                            Ok(content) => {
                                let context_msg = format!(
                                    "File context loaded: {}\n\n```\n{}\n```",
                                    filename, content
                                );
                                self.add_system_message(context_msg, MessageRole::System);
                                self.add_system_message(
                                    format!("✅ Added {} to context.", filename),
                                    MessageRole::System,
                                );
                            }
                            Err(e) => {
                                self.add_system_message(
                                    format!("Failed to read file '{}': {}", filename, e),
                                    MessageRole::Error,
                                );
                            }
                        }
                    } else {
                        self.add_system_message(
                            "Usage: /add <filename>".into(),
                            MessageRole::Error,
                        );
                    }
                    return;
                }
                "/cd" => {
                    if let Some(path) = parts.get(1) {
                        self.change_workspace(path.to_string());
                    } else {
                        self.add_system_message("Usage: /cd <path>".into(), MessageRole::Error);
                    }
                    return;
                }
                _ => {}
            }
        }

        match self.mode {
            AppMode::Chat => {
                self.is_processing = true;
                self.add_system_message(text.clone(), MessageRole::User);
                self.save_current_session();

                let tx = self.event_tx.clone();
                let mcp = self.mcp_tx.clone();
                let history = self.messages.clone();
                let config = self.config.clone();

                let handle = tokio::spawn(async move {
                    if let Err(e) = run_agent_loop(config, history, tx.clone(), mcp).await {
                        let _ = tx.send(AppEvent::Error(e.to_string())).await;
                    }
                    let _ = tx.send(AppEvent::AgentFinished).await;
                });
                self.agent_task = Some(handle);
            }
            AppMode::Terminal => {
                let shell = self.shell_tx.clone();
                tokio::spawn(async move {
                    let _ = shell.send(ShellRequest::UserInput(text)).await;
                });
            }
            _ => {}
        }
    }

    pub fn scroll_up(&mut self) {
        if let AppMode::Chat = self.mode {
            self.chat_stick_to_bottom = false;
            self.chat_scroll = self.chat_scroll.saturating_sub(1);
        } else {
            let i = self.term_scroll.selected().unwrap_or(0) as i32;
            self.term_scroll.select(Some((i - 1).max(0) as usize));
        }
    }
    pub fn scroll_down(&mut self) {
        if let AppMode::Chat = self.mode {
            self.chat_stick_to_bottom = false;
            self.chat_scroll = self.chat_scroll.saturating_add(1);
        } else {
            let i = self.term_scroll.selected().unwrap_or(0) as i32;
            self.term_scroll.select(Some((i + 1).max(0) as usize));
        }
    }
    pub fn scroll_page(&mut self, amt: i16) {
        if let AppMode::Chat = self.mode {
            self.chat_stick_to_bottom = false;
            if amt < 0 {
                self.chat_scroll = self.chat_scroll.saturating_sub(amt.abs() as u16);
            } else {
                self.chat_scroll = self.chat_scroll.saturating_add(amt.abs() as u16);
            }
        } else {
            let i = self.term_scroll.selected().unwrap_or(0) as i32;
            self.term_scroll
                .select(Some((i + amt as i32).max(0) as usize));
        }
    }

    pub fn term_scroll_delta(&mut self, delta: i32) {
        let i = self.term_scroll.selected().unwrap_or(0) as i32;
        self.term_scroll.select(Some((i + delta).max(0) as usize));
    }
}
