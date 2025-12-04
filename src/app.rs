use crate::config::Config;
use crate::mcp::McpRequest;
use crate::session::SessionManager;
use crate::shell::ShellRequest;
use chrono::Local;
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

// Declare the sub-modules so Rust knows to compile them
pub mod actions;
pub mod events;
pub mod inputs;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum AppMode {
    Chat,
    Terminal,
    ModelSelector,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Error,
    Thinking,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    #[serde(default)]
    pub collapsed: bool, // Track expanded/collapsed state
}

pub enum AppEvent {
    Token(String),
    Thinking(String),
    AgentFinished,
    CommandStart(String),
    CommandEnd(String),
    TerminalLine(String),
    Error(String),
    Tick,
    ModelsLoaded(Vec<String>),
    WorkspaceRestarted(mpsc::Sender<ShellRequest>, mpsc::Sender<McpRequest>),
}

pub struct App {
    pub mode: AppMode,
    pub last_mode: AppMode,
    pub input_buffer: String,
    pub messages: Vec<ChatMessage>,

    // Session State
    pub current_session: String,
    pub session_manager: SessionManager,
    pub sessions: Vec<String>,

    // UI State
    pub chat_scroll: u16,
    pub chat_stick_to_bottom: bool,
    pub terminal_lines: Vec<String>,
    pub term_scroll: ListState,
    pub spinner_frame: usize,

    // Model Selector State
    pub available_models: Vec<String>,
    pub model_list_state: ListState,

    // Async State
    pub is_processing: bool,
    pub agent_task: Option<JoinHandle<()>>,

    // Channels
    pub event_tx: mpsc::Sender<AppEvent>,
    pub shell_tx: mpsc::Sender<ShellRequest>,
    pub mcp_tx: mpsc::Sender<McpRequest>,
    pub config: Config,
}

impl App {
    pub fn new(
        event_tx: mpsc::Sender<AppEvent>,
        shell_tx: mpsc::Sender<ShellRequest>,
        mcp_tx: mpsc::Sender<McpRequest>,
        config: Config,
    ) -> Self {
        let session_manager = SessionManager::new();
        let current_session = format!("chat_{}", Local::now().format("%Y-%m-%d_%H-%M"));

        let mut sessions = session_manager.list_sessions().unwrap_or_default();
        sessions.sort_by(|a, b| b.cmp(a));

        Self {
            mode: AppMode::Chat,
            last_mode: AppMode::Chat,
            input_buffer: String::new(),
            messages: vec![ChatMessage {
                role: MessageRole::System,
                content: format!("Ready. Model: {}", config.model),
                collapsed: false,
            }],
            current_session,
            session_manager,
            sessions,

            chat_scroll: 0,
            chat_stick_to_bottom: true,

            terminal_lines: vec![String::from("--- Shell Connected ---")],
            term_scroll: ListState::default(),

            available_models: vec![],
            model_list_state: ListState::default(),

            is_processing: false,
            agent_task: None,
            spinner_frame: 0,

            event_tx,
            shell_tx,
            mcp_tx,
            config,
        }
    }

    // Helper used by all sub-modules
    pub fn add_system_message(&mut self, content: String, role: MessageRole) {
        self.messages.push(ChatMessage {
            role,
            content,
            collapsed: false,
        });
        self.chat_stick_to_bottom = true;
    }
}
