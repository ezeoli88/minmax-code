pub mod api;
pub mod chat;
pub mod commands;
pub mod mcp;
pub mod parser;
pub mod session;
pub mod update;

/// Operating mode for the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Plan,
    Builder,
}

impl Mode {
    pub fn toggle(&self) -> Self {
        match self {
            Mode::Plan => Mode::Builder,
            Mode::Builder => Mode::Plan,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Mode::Plan => "PLAN",
            Mode::Builder => "BUILDER",
        }
    }
}
