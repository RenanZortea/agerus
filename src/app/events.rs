use super::{App, AppEvent, ChatMessage, MessageRole};

impl App {
    pub fn handle_internal_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Tick => {
                if self.is_processing {
                    self.spinner_frame = self.spinner_frame.wrapping_add(1);
                }
            }
            AppEvent::WorkspaceRestarted(new_shell, new_mcp) => {
                self.shell_tx = new_shell;
                self.mcp_tx = new_mcp;
                self.add_system_message(
                    "Workspace changed successfully.".into(),
                    MessageRole::System,
                );
                self.terminal_lines
                    .push("--- Workspace Changed / Shell Restarted ---".into());
            }
            AppEvent::ModelsLoaded(models) => {
                self.available_models = models;
                if !self.available_models.is_empty() {
                    if let Some(pos) = self
                        .available_models
                        .iter()
                        .position(|m| m == &self.config.model)
                    {
                        self.model_list_state.select(Some(pos));
                    } else {
                        self.model_list_state.select(Some(0));
                    }
                }
            }
            AppEvent::Token(t) => self.append_message_content(t, MessageRole::Assistant),
            AppEvent::Thinking(t) => self.append_message_content(t, MessageRole::Thinking),
            AppEvent::CommandStart(c) => {
                self.add_system_message(format!("🛠️ {}", c), MessageRole::System)
            }
            AppEvent::CommandEnd(o) => {
                let s = if o.len() > 200 {
                    format!("Output ({} bytes) sent to terminal.", o.len())
                } else {
                    o
                };
                self.add_system_message(s, MessageRole::System);
            }
            AppEvent::TerminalLine(l) => {
                let was_at_bottom = self
                    .term_scroll
                    .selected()
                    .map_or(true, |s| s >= self.terminal_lines.len().saturating_sub(1));
                self.terminal_lines.push(l);
                if was_at_bottom {
                    self.term_scroll
                        .select(Some(self.terminal_lines.len().saturating_sub(1)));
                }
            }
            AppEvent::AgentFinished => {
                self.is_processing = false;
                self.agent_task = None;
                self.save_current_session();
            }
            AppEvent::Error(e) => {
                self.add_system_message(e, MessageRole::Error);
                self.is_processing = false;
                self.agent_task = None;
                self.save_current_session();
            }
        }
    }

    fn append_message_content(&mut self, content: String, role: MessageRole) {
        let start_new = if let Some(last) = self.messages.last() {
            match (&last.role, &role) {
                (MessageRole::Assistant, MessageRole::Assistant) => false,
                (MessageRole::Thinking, MessageRole::Thinking) => false,
                _ => true,
            }
        } else {
            true
        };

        if start_new {
            // Default Thinking blocks to collapsed
            let collapsed = matches!(role, MessageRole::Thinking);
            self.messages.push(ChatMessage {
                role,
                content,
                collapsed,
            });
        } else {
            if let Some(last) = self.messages.last_mut() {
                last.content.push_str(&content);
            }
        }
        self.chat_stick_to_bottom = true;
    }
}
