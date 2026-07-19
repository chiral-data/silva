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

    /// Set an environment variable in every job's container (headless mode only)
    ///
    /// Repeatable, format KEY=VALUE (e.g. `-e RUN_MODE=use_gpu`). Injected as-is,
    /// unprefixed, into every job's container exec environment for this run —
    /// independent of workflow.toml's `env_passthrough` allowlist.
    #[arg(short = 'e', long = "env", value_name = "KEY=VALUE")]
    env: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Check for updates on startup
    let update_result = silva::update::run_update_check().await;
    if update_result.should_exit {
        // Update was performed, exit
        return Ok(());
    }

    // Check if workflow path is provided
    if let Some(workflow_path) = args.workflow_path {
        // Validate and parse -e/--env KEY=VALUE entries before running anything
        let cli_env_vars = match parse_cli_env_vars(&args.env) {
            Ok(vars) => vars,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        };

        // Headless mode: run workflow directly
        if let Err(e) = silva::headless::run_workflow(&workflow_path, &cli_env_vars).await {
            eprintln!("{e}");
            std::process::exit(1);
        }
        Ok(())
    } else {
        // TUI mode: start the terminal UI with update info
        run_tui(update_result.deferred_update).await
    }
}

/// Validates `-e/--env` entries and returns them unchanged as `KEY=VALUE` strings.
///
/// Rejects entries missing a `=` or with an empty key, so malformed flags fail
/// before any container runs rather than producing a confusing env var later.
fn parse_cli_env_vars(entries: &[String]) -> Result<Vec<String>, String> {
    for entry in entries {
        match entry.split_once('=') {
            Some((key, _)) if !key.is_empty() => {}
            _ => {
                return Err(format!(
                    "Invalid -e/--env value '{entry}': expected KEY=VALUE"
                ));
            }
        }
    }
    Ok(entries.to_vec())
}

/// Runs the TUI application
async fn run_tui(update_available: Option<String>) -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, update_available).await;

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
