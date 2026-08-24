//! Auto-update functionality for Silva CLI.
//!
//! This module provides version checking against GitHub Releases API
//! and interactive update prompts.
//!
//! What an invocation is allowed to do about updates is decided up front by
//! [`UpdatePolicy`], not by the check itself: a batch invocation must never stop
//! on a prompt, a running workflow must never replace the binary executing it,
//! and a caller that opted out must not see an outbound request at all.

use std::io::{self, IsTerminal, Write};
use std::process::Command;
use std::time::Duration;

use semver::Version;
use serde::Deserialize;

const GITHUB_REPO: &str = "chiral-data/silva";
const UPDATE_CHECK_TIMEOUT: Duration = Duration::from_secs(2);

/// Environment variables that switch the update check off entirely.
///
/// `SILVA_NO_UPDATE_CHECK` is the tool's own opt-out; `NO_UPDATE` and `CI` are
/// honoured because automation sets them already and a runner has no business
/// making an unrequested network call in either setting.
const OPT_OUT_VARS: [&str; 3] = ["SILVA_NO_UPDATE_CHECK", "NO_UPDATE", "CI"];

/// What this invocation may do about an available update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdatePolicy {
    /// Do nothing at all — not even the version probe, so no network traffic.
    Disabled,
    /// Probe and report an available update, but never prompt and never install.
    NotifyOnly,
    /// Probe, prompt, and install if the user agrees.
    Interactive,
}

/// The facts about one invocation that decide its [`UpdatePolicy`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateContext {
    /// `--no-update` was passed.
    pub no_update_flag: bool,
    /// One of [`OPT_OUT_VARS`] is set to something other than `0`/empty.
    pub opted_out_by_env: bool,
    /// Output is machine-readable, so human text would corrupt the stream.
    pub json: bool,
    /// Stdin is a terminal, so there is somebody to answer a prompt.
    pub stdin_is_tty: bool,
    /// This invocation is about to run a workflow.
    pub is_workflow_run: bool,
}

impl UpdateContext {
    /// Reads the parts of the context that come from the environment.
    pub fn new(no_update_flag: bool, json: bool, is_workflow_run: bool) -> Self {
        Self {
            no_update_flag,
            opted_out_by_env: env_opt_out(),
            json,
            stdin_is_tty: io::stdin().is_terminal(),
            is_workflow_run,
        }
    }
}

/// True if any opt-out variable is set to a value other than empty or `0`.
fn env_opt_out() -> bool {
    OPT_OUT_VARS.iter().any(|var| match std::env::var(var) {
        Ok(value) => {
            let value = value.trim();
            !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
        }
        Err(_) => false,
    })
}

impl UpdatePolicy {
    /// Decides what an invocation may do about updates.
    ///
    /// Anything that cannot answer a prompt gets [`UpdatePolicy::Disabled`]: a
    /// non-TTY stdin is not an interlocutor, and reading a piped stdin as
    /// consent is how a scripted run ended up trying to overwrite itself
    /// mid-workflow. A workflow run that *could* prompt still only gets
    /// [`UpdatePolicy::NotifyOnly`] — the version that starts a workflow should
    /// be the version that finishes it.
    pub fn resolve(ctx: UpdateContext) -> Self {
        if ctx.no_update_flag || ctx.opted_out_by_env || ctx.json || !ctx.stdin_is_tty {
            Self::Disabled
        } else if ctx.is_workflow_run {
            Self::NotifyOnly
        } else {
            Self::Interactive
        }
    }
}

/// Information about an available update.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub latest_version: String,
    pub current_version: String,
    pub release_url: String,
}

/// GitHub Release API response (partial).
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

/// Check for available updates from GitHub Releases.
///
/// Returns `Ok(Some(UpdateInfo))` if an update is available,
/// `Ok(None)` if already on latest version,
/// `Err` if the check failed (network issues, etc.).
pub async fn check_for_updates() -> Result<Option<UpdateInfo>, String> {
    let current_version = env!("CARGO_PKG_VERSION");
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );

    let client = reqwest::Client::builder()
        .timeout(UPDATE_CHECK_TIMEOUT)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let response = client
        .get(&url)
        .header("User-Agent", "silva-cli")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch release info: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {e}"))?;

    let latest_version = release.tag_name.trim_start_matches('v').to_string();

    // Compare versions using semver
    let current =
        Version::parse(current_version).map_err(|e| format!("Invalid current version: {e}"))?;
    let latest =
        Version::parse(&latest_version).map_err(|e| format!("Invalid latest version: {e}"))?;

    if latest > current {
        Ok(Some(UpdateInfo {
            latest_version,
            current_version: current_version.to_string(),
            release_url: release.html_url,
        }))
    } else {
        Ok(None)
    }
}

/// Prompt the user to update interactively.
///
/// Returns `true` if user wants to update, `false` otherwise.
pub fn prompt_update(info: &UpdateInfo) -> bool {
    println!(
        "New version available: v{} (current: v{})",
        info.latest_version, info.current_version
    );
    print!("Update now? [Y/n]: ");
    io::stdout().flush().ok();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        return false;
    }

    let input = input.trim().to_lowercase();
    input.is_empty() || input == "y" || input == "yes"
}

/// Perform the update by running the install script.
///
/// Returns `Ok(())` if update was successful, `Err` with message otherwise.
pub fn perform_update() -> Result<(), String> {
    println!("Downloading and installing update...");

    #[cfg(unix)]
    {
        let status = Command::new("sh")
            .arg("-c")
            .arg("curl -fsSL https://raw.githubusercontent.com/chiral-data/silva/main/install.sh | sh")
            .status()
            .map_err(|e| format!("Failed to run install script: {e}"))?;

        if status.success() {
            println!("Update complete! Please restart silva.");
            Ok(())
        } else {
            Err(format!(
                "Install script failed with exit code: {:?}",
                status.code()
            ))
        }
    }

    #[cfg(windows)]
    {
        let status = Command::new("powershell")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg("iwr -useb https://raw.githubusercontent.com/chiral-data/silva/main/install.ps1 | iex")
            .status()
            .map_err(|e| format!("Failed to run install script: {e}"))?;

        if status.success() {
            println!("Update complete! Please restart silva.");
            Ok(())
        } else {
            Err(format!(
                "Install script failed with exit code: {:?}",
                status.code()
            ))
        }
    }
}

/// Result of the update check flow.
pub struct UpdateCheckResult {
    /// Whether the application should exit (update was performed)
    pub should_exit: bool,
    /// Available update version if user declined to update (for TUI notification)
    pub deferred_update: Option<String>,
}

impl UpdateCheckResult {
    /// The "nothing happened" result: carry on, on this version.
    fn inert() -> Self {
        Self {
            should_exit: false,
            deferred_update: None,
        }
    }
}

/// Run the update flow permitted by `policy`.
///
/// This is the main entry point for the update feature.
/// Returns UpdateCheckResult indicating whether to exit and any deferred update info.
pub async fn run_update_check(policy: UpdatePolicy) -> UpdateCheckResult {
    match policy {
        UpdatePolicy::Disabled => UpdateCheckResult::inert(),
        UpdatePolicy::NotifyOnly => run_notify_only().await,
        UpdatePolicy::Interactive => run_interactive().await,
    }
}

/// Report an available update and carry on, without prompting or installing.
///
/// Stays silent unless there is something to report: this runs alongside a
/// workflow's own output, and "already on latest version" is noise there. A
/// failed check is silent for the same reason — nothing was asked of the user.
async fn run_notify_only() -> UpdateCheckResult {
    if let Ok(Some(info)) = check_for_updates().await {
        println!(
            "New version available: v{} (current: v{}). Continuing on v{} for this run.",
            info.latest_version, info.current_version, info.current_version
        );
        io::stdout().flush().ok();
    }
    UpdateCheckResult::inert()
}

/// Check, prompt, and install on confirmation.
async fn run_interactive() -> UpdateCheckResult {
    print!("Checking for updates... ");
    io::stdout().flush().ok();

    match check_for_updates().await {
        Ok(Some(info)) => {
            println!();
            let version = info.latest_version.clone();
            if prompt_update(&info) {
                match perform_update() {
                    Ok(()) => {
                        // Exit after successful update
                        return UpdateCheckResult {
                            should_exit: true,
                            deferred_update: None,
                        };
                    }
                    Err(e) => {
                        eprintln!("Update failed: {e}");
                        eprintln!("You can manually update by running:");
                        eprintln!(
                            "  curl -fsSL https://raw.githubusercontent.com/chiral-data/silva/main/install.sh | sh"
                        );
                    }
                }
            } else {
                println!("Update skipped. You can update later with the install script.");
                // Return the deferred update version for TUI notification
                return UpdateCheckResult {
                    should_exit: false,
                    deferred_update: Some(version),
                };
            }
        }
        Ok(None) => {
            println!("already on latest version.");
        }
        Err(e) => {
            println!("skipped ({e})");
        }
    }
    UpdateCheckResult::inert()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("0.3.8").unwrap();
        let v2 = Version::parse("0.4.0").unwrap();
        let v3 = Version::parse("0.3.8").unwrap();

        assert!(v2 > v1);
        assert!(v1 < v2);
        assert!(v1 == v3);
    }

    /// An interactive TUI start on a terminal: the only case that may install.
    fn tui_on_a_terminal() -> UpdateContext {
        UpdateContext {
            no_update_flag: false,
            opted_out_by_env: false,
            json: false,
            stdin_is_tty: true,
            is_workflow_run: false,
        }
    }

    #[test]
    fn interactive_only_for_a_tui_start_on_a_terminal() {
        assert_eq!(
            UpdatePolicy::resolve(tui_on_a_terminal()),
            UpdatePolicy::Interactive
        );
    }

    #[test]
    fn a_workflow_run_never_installs_even_on_a_terminal() {
        let ctx = UpdateContext {
            is_workflow_run: true,
            ..tui_on_a_terminal()
        };
        assert_eq!(UpdatePolicy::resolve(ctx), UpdatePolicy::NotifyOnly);
    }

    /// The reported bug: a scripted run with stdin piped must not reach a prompt.
    #[test]
    fn a_piped_run_is_disabled_not_merely_unprompted() {
        let ctx = UpdateContext {
            stdin_is_tty: false,
            is_workflow_run: true,
            ..tui_on_a_terminal()
        };
        assert_eq!(UpdatePolicy::resolve(ctx), UpdatePolicy::Disabled);
    }

    #[test]
    fn each_opt_out_disables_the_check_on_its_own() {
        for ctx in [
            UpdateContext {
                no_update_flag: true,
                ..tui_on_a_terminal()
            },
            UpdateContext {
                opted_out_by_env: true,
                ..tui_on_a_terminal()
            },
            UpdateContext {
                json: true,
                ..tui_on_a_terminal()
            },
            UpdateContext {
                stdin_is_tty: false,
                ..tui_on_a_terminal()
            },
        ] {
            assert_eq!(
                UpdatePolicy::resolve(ctx),
                UpdatePolicy::Disabled,
                "expected Disabled for {ctx:?}"
            );
        }
    }
}
