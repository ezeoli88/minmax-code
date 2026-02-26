use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

use crate::config::settings::{save_config, AppConfig};
use crate::core::api::{AccumulatedToolCall, MiniMaxClient, QuotaInfo};
use crate::core::chat::{ChatEngine, ChatEvent};
use crate::core::commands::{handle_command, CommandResult};
use crate::core::session::SessionStore;
use crate::core::Mode;
use crate::tui::layout as tui_layout;

// ── Display message types ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub role: MessageRole,
    pub content: String,
    pub reasoning: Option<String>,
    pub tool_calls: Vec<AccumulatedToolCall>,
    pub is_streaming: bool,
    pub tool_status: Option<ToolStatus>,
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
    System,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolStatus {
    Running,
    Done,
    Error,
}

// ── Overlay state ───────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    None,
    CommandPalette { selected: usize },
    SessionList { selected: usize },
}

// ── Application state ───────────────────────────────────────────────────

pub struct App {
    pub config: AppConfig,
    pub mode: Mode,
    pub messages: Vec<DisplayMessage>,
    pub input_text: String,
    pub input_cursor: usize,
    pub scroll_offset: u16,
    pub total_tokens: u64,
    pub quota: Option<QuotaInfo>,
    pub session_name: String,
    pub overlay: Overlay,
    pub system_message: Option<String>,
    pub is_streaming: bool,
    pub should_quit: bool,

    // Internal
    engine: Option<ChatEngine>,
    session_store: Option<Arc<SessionStore>>,
    session_id: Option<String>,
    chat_event_rx: Option<mpsc::UnboundedReceiver<ChatEvent>>,
    engine_return_rx: Option<oneshot::Receiver<ChatEngine>>,
    cancel_token: CancellationToken,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        Self {
            mode: Mode::Builder,
            messages: Vec::new(),
            input_text: String::new(),
            input_cursor: 0,
            scroll_offset: 0,
            total_tokens: 0,
            quota: None,
            session_name: "New Session".to_string(),
            overlay: Overlay::None,
            system_message: None,
            is_streaming: false,
            should_quit: false,
            engine: None,
            session_store: None,
            session_id: None,
            chat_event_rx: None,
            engine_return_rx: None,
            cancel_token: CancellationToken::new(),
            config,
        }
    }

    /// Initialize the chat engine and session store.
    pub fn initialize(&mut self) -> Result<()> {
        if self.config.api_key.is_empty() {
            self.system_message = Some(
                "No API key configured. Run /config or set MINIMAX_API_KEY env var.".to_string(),
            );
            // Try env var fallback
            if let Ok(key) = std::env::var("MINIMAX_API_KEY") {
                self.config.api_key = key;
            }
        }

        let client = MiniMaxClient::new(&self.config.api_key);
        let mut engine = ChatEngine::new(client, &self.config.model, self.mode);

        // Initialize session store
        if let Ok(store) = SessionStore::open() {
            let store = Arc::new(store);
            if let Ok(session) = store.create_session(&self.config.model) {
                self.session_id = Some(session.id.clone());
                self.session_name = session.name.clone();
                engine.set_session(session.id, store.clone());
            }
            self.session_store = Some(store);
        }

        self.engine = Some(engine);
        Ok(())
    }

    pub fn theme_name(&self) -> &str {
        &self.config.theme
    }

    /// Handle a terminal event (key, mouse, resize).
    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Mouse(mouse) => self.handle_mouse(mouse),
            Event::Resize(_, _) => {} // Layout re-calculates on next draw
            _ => {}
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        // Global: Ctrl+C to quit
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            if self.is_streaming {
                self.cancel_streaming();
            } else {
                self.should_quit = true;
            }
            return;
        }

        // Handle overlay input first
        if self.overlay != Overlay::None {
            self.handle_overlay_key(key);
            return;
        }

        // Escape: cancel streaming or clear system message
        if key.code == KeyCode::Esc {
            if self.is_streaming {
                self.cancel_streaming();
            } else if self.system_message.is_some() {
                self.system_message = None;
            }
            return;
        }

        // Tab: toggle mode
        if key.code == KeyCode::Tab {
            self.toggle_mode();
            return;
        }

        // Scrolling
        match key.code {
            KeyCode::Up => {
                if key.modifiers.contains(KeyModifiers::NONE) && self.input_text.is_empty() {
                    self.scroll_up(3);
                    return;
                }
            }
            KeyCode::Down => {
                if key.modifiers.contains(KeyModifiers::NONE) && self.input_text.is_empty() {
                    self.scroll_down(3);
                    return;
                }
            }
            _ => {}
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('u') => {
                    self.scroll_up(15);
                    return;
                }
                KeyCode::Char('d') => {
                    self.scroll_down(15);
                    return;
                }
                _ => {}
            }
        }

        // Input handling
        match key.code {
            KeyCode::Enter => {
                if !self.is_streaming {
                    self.submit_input();
                }
            }
            KeyCode::Char(c) => {
                self.input_text.insert(self.input_cursor, c);
                self.input_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                    self.input_text.remove(self.input_cursor);
                }
            }
            KeyCode::Delete => {
                if self.input_cursor < self.input_text.len() {
                    self.input_text.remove(self.input_cursor);
                }
            }
            KeyCode::Left => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if self.input_cursor < self.input_text.len() {
                    self.input_cursor += 1;
                }
            }
            KeyCode::Home => {
                self.input_cursor = 0;
            }
            KeyCode::End => {
                self.input_cursor = self.input_text.len();
            }
            _ => {}
        }
    }

    fn handle_overlay_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            self.overlay = Overlay::None;
            return;
        }

        // Extract overlay state to avoid borrow conflicts
        let overlay_clone = self.overlay.clone();
        match overlay_clone {
            Overlay::CommandPalette { mut selected } => {
                let commands = command_palette_items();
                match key.code {
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                        }
                        self.overlay = Overlay::CommandPalette { selected };
                    }
                    KeyCode::Down => {
                        if selected < commands.len().saturating_sub(1) {
                            selected += 1;
                        }
                        self.overlay = Overlay::CommandPalette { selected };
                    }
                    KeyCode::Enter => {
                        let cmd = &commands[selected];
                        let result = handle_command(cmd.0);
                        self.overlay = Overlay::None;
                        self.apply_command_result(result);
                    }
                    _ => {}
                }
            }
            Overlay::SessionList { mut selected } => {
                let sessions = self.list_sessions();
                match key.code {
                    KeyCode::Up => {
                        if selected > 0 {
                            selected -= 1;
                        }
                        self.overlay = Overlay::SessionList { selected };
                    }
                    KeyCode::Down => {
                        if selected < sessions.len().saturating_sub(1) {
                            selected += 1;
                        }
                        self.overlay = Overlay::SessionList { selected };
                    }
                    KeyCode::Enter => {
                        if let Some(session) = sessions.get(selected) {
                            self.load_session(&session.0.clone());
                        }
                        self.overlay = Overlay::None;
                    }
                    _ => {}
                }
            }
            Overlay::None => {}
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::ScrollUp => self.scroll_up(3),
            MouseEventKind::ScrollDown => self.scroll_down(3),
            _ => {}
        }
    }

    fn scroll_up(&mut self, lines: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(lines);
    }

    fn scroll_down(&mut self, lines: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    fn toggle_mode(&mut self) {
        self.mode = self.mode.toggle();
        if let Some(engine) = &mut self.engine {
            engine.set_mode(self.mode);
        }
    }

    fn cancel_streaming(&mut self) {
        self.cancel_token.cancel();
        self.is_streaming = false;
        self.cancel_token = CancellationToken::new();
    }

    fn submit_input(&mut self) {
        let text = self.input_text.trim().to_string();
        if text.is_empty() {
            return;
        }

        // Check for slash commands
        if text.starts_with('/') {
            let result = handle_command(&text);
            self.input_text.clear();
            self.input_cursor = 0;
            self.apply_command_result(result);
            return;
        }

        // Add user message to display
        self.messages.push(DisplayMessage {
            role: MessageRole::User,
            content: text.clone(),
            reasoning: None,
            tool_calls: Vec::new(),
            is_streaming: false,
            tool_status: None,
            tool_name: None,
        });

        // Reset scroll to bottom
        self.scroll_offset = 0;

        // Clear input
        self.input_text.clear();
        self.input_cursor = 0;

        // Start streaming
        self.start_streaming(text);
    }

    fn start_streaming(&mut self, user_input: String) {
        let Some(engine) = self.engine.take() else {
            return;
        };

        self.is_streaming = true;
        self.cancel_token = CancellationToken::new();

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        self.chat_event_rx = Some(event_rx);

        // Add placeholder assistant message
        self.messages.push(DisplayMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            reasoning: None,
            tool_calls: Vec::new(),
            is_streaming: true,
            tool_status: None,
            tool_name: None,
        });

        // Spawn the streaming task and return the engine via oneshot
        let (engine_tx, engine_rx) = oneshot::channel();
        self.engine_return_rx = Some(engine_rx);

        let mut engine_owned = engine;
        tokio::spawn(async move {
            let _ = engine_owned
                .send_message(&user_input, None, event_tx)
                .await;
            let _ = engine_tx.send(engine_owned);
        });
    }

    /// Poll for chat events from the streaming task.
    /// Call this in the main event loop.
    pub fn poll_chat_events(&mut self) {
        if self.chat_event_rx.is_none() {
            return;
        }

        // Drain all available events into a buffer to avoid borrow conflicts
        let mut events = Vec::new();
        let mut disconnected = false;

        if let Some(rx) = &mut self.chat_event_rx {
            loop {
                match rx.try_recv() {
                    Ok(event) => events.push(event),
                    Err(mpsc::error::TryRecvError::Empty) => break,
                    Err(mpsc::error::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }

        for event in events {
            self.process_chat_event(event);
        }

        if disconnected {
            self.is_streaming = false;
            self.chat_event_rx = None;

            // Try to recover the engine
            if let Some(mut rx) = self.engine_return_rx.take() {
                if let Ok(engine) = rx.try_recv() {
                    self.engine = Some(engine);
                }
            }
        }
    }

    fn process_chat_event(&mut self, event: ChatEvent) {
        match event {
            ChatEvent::StreamStart => {
                // Ensure we have a streaming message
                if self
                    .messages
                    .last()
                    .map(|m| m.role != MessageRole::Assistant || !m.is_streaming)
                    .unwrap_or(true)
                {
                    self.messages.push(DisplayMessage {
                        role: MessageRole::Assistant,
                        content: String::new(),
                        reasoning: None,
                        tool_calls: Vec::new(),
                        is_streaming: true,
                        tool_status: None,
                        tool_name: None,
                    });
                }
            }
            ChatEvent::ReasoningChunk(text) => {
                if let Some(msg) = self
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|m| m.role == MessageRole::Assistant && m.is_streaming)
                {
                    let r = msg.reasoning.get_or_insert_with(String::new);
                    r.push_str(&text);
                }
            }
            ChatEvent::ContentChunk(text) => {
                if let Some(msg) = self
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|m| m.role == MessageRole::Assistant && m.is_streaming)
                {
                    msg.content.push_str(&text);
                }
                // Keep scroll at bottom during streaming
                self.scroll_offset = 0;
            }
            ChatEvent::ToolCallsUpdate(tool_calls) => {
                if let Some(msg) = self
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|m| m.role == MessageRole::Assistant && m.is_streaming)
                {
                    msg.tool_calls = tool_calls;
                }
            }
            ChatEvent::StreamEnd(final_msg) => {
                if let Some(msg) = self
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|m| m.role == MessageRole::Assistant && m.is_streaming)
                {
                    msg.content = final_msg.content;
                    msg.tool_calls = final_msg.tool_calls;
                    if !final_msg.reasoning.is_empty() {
                        msg.reasoning = Some(final_msg.reasoning);
                    }
                    msg.is_streaming = false;
                }
            }
            ChatEvent::ToolExecutionStart { id: _, name } => {
                self.messages.push(DisplayMessage {
                    role: MessageRole::Tool,
                    content: String::new(),
                    reasoning: None,
                    tool_calls: Vec::new(),
                    is_streaming: false,
                    tool_status: Some(ToolStatus::Running),
                    tool_name: Some(name),
                });
            }
            ChatEvent::ToolExecutionDone {
                id: _,
                name,
                result,
            } => {
                if let Some(msg) = self
                    .messages
                    .iter_mut()
                    .rev()
                    .find(|m| {
                        m.role == MessageRole::Tool
                            && m.tool_name.as_deref() == Some(&name)
                            && m.tool_status == Some(ToolStatus::Running)
                    })
                {
                    let is_error = result.starts_with("Error:");
                    msg.content = result;
                    msg.tool_status = Some(if is_error {
                        ToolStatus::Error
                    } else {
                        ToolStatus::Done
                    });
                }
            }
            ChatEvent::TokenCount(tokens) => {
                self.total_tokens += tokens;
            }
            ChatEvent::Error(msg) => {
                self.system_message = Some(msg);
            }
        }
    }

    fn apply_command_result(&mut self, result: CommandResult) {
        match result {
            CommandResult::Message(msg) => {
                self.messages.push(DisplayMessage {
                    role: MessageRole::System,
                    content: msg,
                    reasoning: None,
                    tool_calls: Vec::new(),
                    is_streaming: false,
                    tool_status: None,
                    tool_name: None,
                });
            }
            CommandResult::NewSession => {
                self.new_session();
            }
            CommandResult::Clear => {
                self.messages.clear();
                if let Some(engine) = &mut self.engine {
                    engine.clear();
                }
            }
            CommandResult::Exit => {
                self.should_quit = true;
            }
            CommandResult::Sessions => {
                self.overlay = Overlay::SessionList { selected: 0 };
            }
            CommandResult::Config => {
                self.messages.push(DisplayMessage {
                    role: MessageRole::System,
                    content: format!(
                        "Current config:\n  API Key: {}...{}\n  Model: {}\n  Theme: {}\n\nEdit ~/.minmax-code/config.json to change settings.",
                        &self.config.api_key.chars().take(4).collect::<String>(),
                        &self.config.api_key.chars().rev().take(4).collect::<String>().chars().rev().collect::<String>(),
                        self.config.model,
                        self.config.theme,
                    ),
                    reasoning: None,
                    tool_calls: Vec::new(),
                    is_streaming: false,
                    tool_status: None,
                    tool_name: None,
                });
            }
            CommandResult::SetModel(model) => {
                self.config.model = model.clone();
                if let Some(engine) = &mut self.engine {
                    engine.set_model(&model);
                }
                let _ = save_config(&self.config);
                self.system_message = Some(format!("Model changed to {}", model));
            }
            CommandResult::SetTheme(theme) => {
                self.config.theme = theme.clone();
                let _ = save_config(&self.config);
                self.system_message = Some(format!("Theme changed to {}", theme));
            }
            CommandResult::None => {}
        }
    }

    fn new_session(&mut self) {
        self.messages.clear();
        self.total_tokens = 0;
        self.scroll_offset = 0;

        if let Some(store) = &self.session_store {
            if let Ok(session) = store.create_session(&self.config.model) {
                self.session_id = Some(session.id.clone());
                self.session_name = session.name.clone();
                if let Some(engine) = &mut self.engine {
                    engine.clear();
                    engine.set_session(session.id, store.clone());
                }
            }
        }
    }

    pub fn list_sessions(&self) -> Vec<(String, String, String)> {
        if let Some(store) = &self.session_store {
            store
                .list_sessions()
                .unwrap_or_default()
                .into_iter()
                .map(|s| (s.id, s.name, s.model))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn load_session(&mut self, session_id: &str) {
        let Some(store) = &self.session_store else {
            return;
        };
        let msgs = store.get_session_messages(session_id).unwrap_or_default();

        self.messages.clear();
        self.session_id = Some(session_id.to_string());

        for msg in &msgs {
            let role = match msg.role.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                "tool" => MessageRole::Tool,
                _ => MessageRole::System,
            };
            self.messages.push(DisplayMessage {
                role,
                content: msg.content.clone(),
                reasoning: None,
                tool_calls: Vec::new(),
                is_streaming: false,
                tool_status: if msg.role == "tool" {
                    Some(ToolStatus::Done)
                } else {
                    None
                },
                tool_name: msg.name.clone(),
            });
        }

        // Rebuild engine history
        if let Some(engine) = &mut self.engine {
            engine.clear();
            let history: Vec<serde_json::Value> = msgs
                .iter()
                .map(|m| {
                    let mut v = serde_json::json!({
                        "role": m.role,
                        "content": m.content
                    });
                    if let Some(tc) = &m.tool_calls {
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(tc) {
                            v["tool_calls"] = parsed;
                        }
                    }
                    if let Some(id) = &m.tool_call_id {
                        v["tool_call_id"] = serde_json::json!(id);
                    }
                    v
                })
                .collect();
            engine.load_history(history);
            engine.set_session(session_id.to_string(), store.clone());
        }

        self.scroll_offset = 0;
    }
}

/// Items for the command palette.
pub fn command_palette_items() -> Vec<(&'static str, &'static str)> {
    vec![
        ("/new", "Start a new session"),
        ("/sessions", "Browse previous sessions"),
        ("/model", "Change model"),
        ("/theme", "Change theme"),
        ("/config", "Show configuration"),
        ("/init", "Create agent.md template"),
        ("/clear", "Clear current chat"),
        ("/exit", "Exit the terminal"),
    ]
}

/// The main run loop. Sets up the terminal and runs the event loop.
pub async fn run(config: AppConfig) -> Result<()> {
    // Setup terminal
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture,
    )?;

    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config);
    app.initialize()?;

    let result = event_loop(&mut terminal, &mut app).await;

    // Restore terminal
    crossterm::terminal::disable_raw_mode()?;
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::LeaveAlternateScreen,
        crossterm::event::DisableMouseCapture,
    )?;
    terminal.show_cursor()?;

    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    loop {
        // Draw
        terminal.draw(|frame| {
            tui_layout::draw(frame, app);
        })?;

        if app.should_quit {
            break;
        }

        // Poll chat events from streaming
        app.poll_chat_events();

        // Poll terminal events with a short timeout to keep UI responsive
        if crossterm::event::poll(Duration::from_millis(16))? {
            let event = event::read()?;
            app.handle_event(event);
        }
    }
    Ok(())
}
