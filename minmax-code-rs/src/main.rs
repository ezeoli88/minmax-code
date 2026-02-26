mod config;
mod core;
mod tools;
mod tui;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "minmax-code", version, about = "AI-powered terminal coding assistant")]
struct Args {
    /// Start in Plan mode instead of Builder mode
    #[arg(long)]
    plan: bool,

    /// Override the model to use
    #[arg(long, short = 'm')]
    model: Option<String>,

    /// Override the theme
    #[arg(long)]
    theme: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut config = config::settings::load_config();

    // Apply CLI overrides
    if let Some(model) = args.model {
        config.model = model;
    }
    if let Some(theme) = args.theme {
        config.theme = theme;
    }

    // Launch TUI
    tui::app::run(config).await
}
