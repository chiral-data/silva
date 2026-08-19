use std::path::PathBuf;
use std::{error::Error, io};

use clap::{Parser, Subcommand};
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
    #[command(subcommand)]
    command: Option<Command>,

    /// Path to a workflow folder to run directly (headless mode)
    ///
    /// Deprecated: use `silva run <WORKFLOW_PATH>` instead. If neither this nor
    /// a subcommand is given, the TUI application starts.
    #[arg(value_name = "WORKFLOW_PATH")]
    workflow_path: Option<PathBuf>,

    /// Emit the run as newline-delimited JSON events instead of human output
    ///
    /// Headless mode only. Every line on stdout is one JSON object: workflow
    /// start and end, each job's state changes, and every log line attributed
    /// to the job and stream that produced it.
    #[arg(long)]
    json: bool,

    /// Set an environment variable in every job's container (headless mode only)
    ///
    /// Repeatable, format KEY=VALUE (e.g. `-e RUN_MODE=use_gpu`). Injected as-is,
    /// unprefixed, into every job's container exec environment for this run —
    /// independent of workflow.toml's `env_passthrough` allowlist.
    #[arg(short = 'e', long = "env", value_name = "KEY=VALUE")]
    env: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run a workflow folder headlessly (no TUI)
    Run {
        /// Path to the workflow folder
        #[arg(value_name = "WORKFLOW_PATH")]
        path: PathBuf,

        /// Emit the run as newline-delimited JSON events instead of human output
        ///
        /// Every line on stdout is one JSON object: workflow start and end, each
        /// job's state changes, silva's own diagnostics, and every log line
        /// attributed to the job and stream that produced it.
        #[arg(long)]
        json: bool,

        /// Set an environment variable in every job's container
        ///
        /// Repeatable, format KEY=VALUE (e.g. `-e RUN_MODE=use_gpu`). Injected as-is,
        /// unprefixed, into every job's container exec environment for this run —
        /// independent of workflow.toml's `env_passthrough` allowlist.
        #[arg(short = 'e', long = "env", value_name = "KEY=VALUE")]
        env: Vec<String>,
    },

    /// Check a workflow folder without running it
    ///
    /// Parses workflow.toml and every job.toml, checks the dependency graph and
    /// parameter files, and applies the same conventions a headless run applies.
    /// Needs no Docker and starts no containers.
    Validate {
        /// Path to the workflow folder
        #[arg(value_name = "WORKFLOW_PATH")]
        path: PathBuf,

        /// Emit the report as JSON
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Validation is a local, offline check: no update probe, no network, no
    // Docker — so it stays usable in CI and inside another tool.
    if let Some(Command::Validate { path, json }) = args.command {
        let report = silva::validate::validate_workflow(&path);
        if json {
            println!("{}", report.to_json());
        } else if report.is_valid() {
            print!("{}", report.render());
        } else {
            eprint!("{}", report.render());
        }
        std::process::exit(if report.is_valid() { 0 } else { 1 });
    }

    // Whether this invocation wants machine-readable output, in either form:
    // `silva run <path> --json` or the deprecated `silva <path> --json`.
    let json = args.json || matches!(&args.command, Some(Command::Run { json: true, .. }));

    silva::events::set_json_mode(json);

    // Check for updates on startup. Skipped for --json: it prints human text
    // onto the stream a caller is parsing, prompts for input, and reaches the
    // network besides.
    let update_result = if json {
        silva::update::UpdateCheckResult {
            should_exit: false,
            deferred_update: None,
        }
    } else {
        silva::update::run_update_check().await
    };
    if update_result.should_exit {
        // Update was performed, exit
        return Ok(());
    }

    // `silva run <path>` is the explicit form; a bare `silva <path>` is the
    // deprecated alias kept for existing scripts.
    let (workflow_path, env) = match args.command {
        Some(Command::Run { path, env, .. }) => (Some(path), env),
        Some(Command::Validate { .. }) => unreachable!("handled above"),
        None => {
            if args.workflow_path.is_some() {
                silva::events::warn_line(
                    "warning: `silva <WORKFLOW_PATH>` is deprecated, use `silva run <WORKFLOW_PATH>` instead"
                        .to_string(),
                );
            }
            (args.workflow_path, args.env)
        }
    };

    if let Some(workflow_path) = workflow_path {
        // Validate and parse -e/--env KEY=VALUE entries before running anything
        let cli_env_vars = match parse_cli_env_vars(&env) {
            Ok(vars) => vars,
            Err(e) => {
                eprintln!("{e}");
                std::process::exit(1);
            }
        };

        // Headless mode: run workflow directly
        let output = if json {
            silva::events::OutputFormat::Json
        } else {
            silva::events::OutputFormat::Human
        };
        if let Err(e) = silva::headless::run_workflow(&workflow_path, &cli_env_vars, output).await {
            // In JSON mode the terminal workflow event already carries this,
            // and stdout must stay parseable.
            if !json {
                eprintln!("{e}");
            }
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
