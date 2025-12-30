//! Auto-update functionality for Silva CLI.
//!
//! This module provides version checking against GitHub Releases API
//! and interactive update prompts.

use std::io::{self, Write};
use std::process::Command;
use std::time::Duration;

use semver::Version;
use serde::Deserialize;

const GITHUB_REPO: &str = "chiral-data/silva";
const UPDATE_CHECK_TIMEOUT: Duration = Duration::from_secs(2);

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
        return Err(format!(
            "GitHub API returned status: {}",
            response.status()
        ));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse release info: {e}"))?;

    let latest_version = release.tag_name.trim_start_matches('v').to_string();

    // Compare versions using semver
    let current = Version::parse(current_version)
        .map_err(|e| format!("Invalid current version: {e}"))?;
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

/// Run the complete update check and prompt flow.
///
/// This is the main entry point for the update feature.
/// Returns `true` if the application should exit (update was performed).
pub async fn run_update_check() -> bool {
    print!("Checking for updates... ");
    io::stdout().flush().ok();

    match check_for_updates().await {
        Ok(Some(info)) => {
            println!();
            if prompt_update(&info) {
                match perform_update() {
                    Ok(()) => {
                        // Exit after successful update
                        return true;
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
            }
        }
        Ok(None) => {
            println!("already on latest version.");
        }
        Err(e) => {
            println!("skipped ({e})");
        }
    }
    false
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
}
