use std::path::PathBuf;
use std::{error::Error, io};

use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use silva::run_app;

/// Silva - Terminal UI for managing Docker-based data workflows
#[derive(Parser, Debug)]
#[command(name = "silva")]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to a workflow folder to run directly (headless mode)
    ///
    /// If not provided, the TUI application will start.
    #[arg(value_name = "WORKFLOW_PATH")]
    workflow_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Check for updates on startup
    if silva::update::run_update_check().await {
        // Update was performed, exit
        return Ok(());
    }

    // Check if workflow path is provided
    if let Some(workflow_path) = args.workflow_path {
        // Headless mode: run workflow directly
        silva::headless::run_workflow(&workflow_path)
            .await
            .map_err(|e| e.into())
    } else {
        // TUI mode: start the terminal UI
        run_tui().await
    }
}

/// Runs the TUI application
async fn run_tui() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
