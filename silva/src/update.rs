//! Automatic update: check, verify, stage, take effect on the next start.
//!
//! Nothing is ever asked of the user. That is only safe because installing and
//! activating are separated: the new binary is verified and renamed into place
//! while the current one keeps running from its now-unlinked inode, so the
//! version that started an invocation is the version that finishes it, and the
//! next invocation is the new one. An update therefore never changes anything
//! underneath a running workflow, which is what made a confirmation prompt
//! necessary in the first place.
//!
//! The guardrails that replace the prompt:
//!
//! - the download is checked against the release's published SHA-256, and is
//!   not installed without one — the trust anchor is the digest, not a script
//!   piped from a branch;
//! - only within a patch series (`0.5.11` → `0.5.12`), never across a minor or
//!   major boundary where behaviour is allowed to change;
//! - only when silva's own binary is writable, so a packaged or system-wide
//!   install is left to whoever owns it;
//! - the outgoing binary is kept next to the new one for `silva --rollback`;
//! - the check is cached, so it is roughly one request a day rather than one
//!   per invocation;
//! - `--no-update`, `SILVA_UPDATE=off`, `SILVA_NO_UPDATE_CHECK`, `NO_UPDATE` or
//!   `CI` switch it off entirely, with no outbound request at all.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::VERSION;

const GITHUB_REPO: &str = "chiral-data/silva";
const UPDATE_CHECK_TIMEOUT: Duration = Duration::from_secs(2);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(120);

/// How long a check is trusted, and how long a failed install is left alone.
///
/// The version probe is not urgent — a release found a day late costs nothing —
/// and an invocation that makes no network call at all is the point.
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

/// How long an exiting process waits for an update it started to finish.
const HANDOVER_GRACE: Duration = Duration::from_secs(3);

/// Environment variables that switch updating off entirely.
///
/// `SILVA_NO_UPDATE_CHECK` is the tool's own opt-out; `NO_UPDATE` and `CI` are
/// honoured because automation sets them already, and a runner has no business
/// making an unrequested network call in either setting.
const OPT_OUT_VARS: [&str; 3] = ["SILVA_NO_UPDATE_CHECK", "NO_UPDATE", "CI"];

/// Selects the mode directly: `auto`, `notify` or `off`.
const MODE_VAR: &str = "SILVA_UPDATE";

/// Where release assets live. Passed in rather than reached for, so the install
/// path can be exercised end to end against a local server in tests.
const RELEASE_BASE: &str = "https://github.com/chiral-data/silva/releases/download";

const INSTALL_HINT: &str =
    "  curl -fsSL https://raw.githubusercontent.com/chiral-data/silva/main/install.sh | sh";

/// What silva may do about a new release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UpdateMode {
    /// Check, verify and install in the background; the next start runs it.
    #[default]
    Auto,
    /// Check and say so, but install nothing.
    Notify,
    /// Do nothing at all — not even the version probe, so no network traffic.
    Off,
}

impl UpdateMode {
    /// Reads the mode from the flag and the environment.
    pub fn resolve(no_update_flag: bool) -> Self {
        Self::resolve_from(
            no_update_flag,
            std::env::var(MODE_VAR).ok().as_deref(),
            OPT_OUT_VARS.iter().any(|var| env_flag_set(var)),
        )
    }

    /// The decision itself, over values rather than the environment.
    ///
    /// `--no-update` wins outright. An explicit [`MODE_VAR`] is next, because
    /// somebody who set it meant it — including `SILVA_UPDATE=auto` in CI. The
    /// blanket opt-outs come last, and the default is [`UpdateMode::Auto`] for
    /// every kind of invocation: a TUI start, a headless run, `--json`, a
    /// terminal or a pipe. None of them can be disturbed by an update that only
    /// takes effect next time.
    fn resolve_from(no_update_flag: bool, mode_var: Option<&str>, opted_out: bool) -> Self {
        if no_update_flag {
            return Self::Off;
        }
        if let Some(value) = mode_var
            && let Some(mode) = Self::parse(value)
        {
            return mode;
        }
        if opted_out {
            return Self::Off;
        }
        Self::Auto
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "auto" | "install" => Some(Self::Auto),
            "notify" | "check" => Some(Self::Notify),
            "off" | "never" | "none" | "0" | "false" => Some(Self::Off),
            _ => None,
        }
    }
}

/// True if the variable is set to anything other than empty, `0` or `false`.
fn env_flag_set(var: &str) -> bool {
    match std::env::var(var) {
        Ok(value) => {
            let value = value.trim();
            !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
        }
        Err(_) => false,
    }
}

/// What the update attempt came to, when it came to anything worth saying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    /// Installed and waiting; it becomes the running version on the next start.
    Installed { from: String, to: String },
    /// A newer release exists but was deliberately not installed.
    Available { latest: String, why: String },
}

impl UpdateOutcome {
    /// Says what happened, once, on the way out.
    ///
    /// Routed through [`crate::events`] so that a `--json` consumer gets a
    /// `note` event rather than a bare line on the stream it is parsing.
    pub fn report(&self) {
        match self {
            Self::Installed { from, to } => crate::events::note_line(format!(
                "Updated silva to v{to} (was v{from}); it takes effect the next time you run it."
            )),
            Self::Available { latest, why } => {
                crate::events::note_line(format!(
                    "New version available: v{latest} (current: v{VERSION}), not installed: {why}."
                ));
                crate::events::note_line(format!("Install it with:\n{INSTALL_HINT}"));
            }
        }
    }
}

/// An update running alongside the rest of the invocation.
///
/// Held rather than awaited: the check and download must not delay a workflow
/// starting, and their result is only interesting on the way out.
pub struct UpdateTask(Option<tokio::task::JoinHandle<Option<UpdateOutcome>>>);

impl UpdateTask {
    /// Starts the update in the background. Returns immediately.
    pub fn spawn(mode: UpdateMode) -> Self {
        if mode == UpdateMode::Off {
            return Self(None);
        }
        Self(Some(tokio::spawn(run(mode))))
    }

    /// Waits briefly for the update to land, then reports whatever it came to.
    ///
    /// A download that has not finished by the time the work has is abandoned:
    /// nothing has been renamed into place yet, so the worst it leaves behind is
    /// a staging file that the next attempt sweeps. [`Cache`] remembers the
    /// attempt either way, so an abandoned download is not retried until the
    /// next window — a short-lived invocation cannot spend its life
    /// re-downloading the same release.
    pub async fn finish(self) {
        let Some(handle) = self.0 else {
            return;
        };
        // Anything else — nothing to say, a panicked task, a download still
        // going — is not what this invocation was for, and is silent.
        if let Ok(Ok(Some(outcome))) = tokio::time::timeout(HANDOVER_GRACE, handle).await {
            outcome.report();
        }
    }
}

/// The whole flow, from cached version knowledge to an installed binary.
///
/// Every failure is silent: the caller asked to run a workflow, not to be told
/// about GitHub being unreachable.
async fn run(mode: UpdateMode) -> Option<UpdateOutcome> {
    let current = Version::parse(VERSION).ok()?;
    let mut cache = Cache::load();

    let latest = match cache.fresh_latest_version() {
        Some(known) => known,
        None => {
            let fetched = fetch_latest_version().await.ok()?;
            cache.record_check(&fetched);
            cache.save();
            fetched
        }
    };

    let latest_version = Version::parse(&latest).ok()?;
    if latest_version <= current {
        return None;
    }

    if mode == UpdateMode::Notify {
        return Some(UpdateOutcome::Available {
            latest,
            why: "SILVA_UPDATE=notify".to_string(),
        });
    }

    // A patch release is a fix to the same behaviour; a minor or major release
    // is where behaviour is allowed to change. Crossing that boundary silently
    // under a workflow that was validated against this version is not a thing
    // to do without being asked, so it is reported instead.
    if !same_patch_series(&current, &latest_version) {
        return Some(UpdateOutcome::Available {
            latest,
            why: "not a patch release".to_string(),
        });
    }

    if !cache.install_attempt_due() {
        return None;
    }
    cache.record_install_attempt();
    cache.save();

    let outcome = match locate_binary() {
        Ok(exe) => install(RELEASE_BASE, &latest, &exe).await,
        Err(why) => Err(why),
    };

    match outcome {
        Ok(()) => {
            cache.record_installed(VERSION, &latest);
            cache.save();
            Some(UpdateOutcome::Installed {
                from: VERSION.to_string(),
                to: latest,
            })
        }
        Err(why) => Some(UpdateOutcome::Available { latest, why }),
    }
}

/// The binary this process is running, with symlinks resolved.
///
/// A symlinked install must have its target replaced rather than the link.
fn locate_binary() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("cannot locate silva itself: {e}"))?;
    Ok(fs::canonicalize(&exe).unwrap_or(exe))
}

/// True when only the patch component differs, so behaviour should not.
fn same_patch_series(current: &Version, latest: &Version) -> bool {
    current.major == latest.major && current.minor == latest.minor
}

/// The newest release version already known, without touching the network.
///
/// Used to show the TUI's update badge at startup: the cache is what makes that
/// possible without a blocking request before the first frame.
pub fn cached_available_version() -> Option<String> {
    let current = Version::parse(VERSION).ok()?;
    let latest = Cache::load().latest_version?;
    if Version::parse(&latest).ok()? > current {
        Some(latest)
    } else {
        None
    }
}

/// GitHub Release API response (partial).
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Asks GitHub for the latest release tag, as a bare version.
async fn fetch_latest_version() -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");

    let response = http_client(UPDATE_CHECK_TIMEOUT)?
        .get(&url)
        .header("User-Agent", "silva-cli")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| format!("failed to fetch release info: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }

    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("failed to parse release info: {e}"))?;

    Ok(release.tag_name.trim_start_matches('v').to_string())
}

fn http_client(timeout: Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|e| format!("failed to create HTTP client: {e}"))
}

/// Downloads `version`, verifies it, and puts it where the next start finds it.
async fn install(release_base: &str, version: &str, exe: &Path) -> Result<(), String> {
    let asset = asset_name(std::env::consts::OS, std::env::consts::ARCH)
        .ok_or("no release build for this platform")?;
    let base = format!("{release_base}/v{version}");

    // The digest comes first: without one there is nothing to trust the
    // download against, and an unverified binary is not installed at all.
    let published = fetch_text(&format!("{base}/{asset}.sha256"))
        .await
        .map_err(|_| "the release publishes no checksum to verify against".to_string())?;
    let expected = parse_digest(&published).ok_or("the published checksum is unreadable")?;

    let bytes = fetch_bytes(&format!("{base}/{asset}")).await?;
    let actual = sha256_hex(&bytes);
    if actual != expected {
        return Err(format!(
            "checksum mismatch: expected {expected}, downloaded {actual}"
        ));
    }

    let staging =
        tempfile::tempdir().map_err(|e| format!("cannot create a temporary directory: {e}"))?;
    let new_binary = extract(&bytes, staging.path())?;
    activate(&new_binary, exe)
}

async fn fetch_text(url: &str) -> Result<String, String> {
    let response = http_client(UPDATE_CHECK_TIMEOUT)?
        .get(url)
        .header("User-Agent", "silva-cli")
        .send()
        .await
        .map_err(|e| format!("failed to fetch {url}: {e}"))?
        .error_for_status()
        .map_err(|e| format!("failed to fetch {url}: {e}"))?;
    response
        .text()
        .await
        .map_err(|e| format!("failed to read {url}: {e}"))
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>, String> {
    let response = http_client(DOWNLOAD_TIMEOUT)?
        .get(url)
        .header("User-Agent", "silva-cli")
        .send()
        .await
        .map_err(|e| format!("failed to download the release: {e}"))?
        .error_for_status()
        .map_err(|e| format!("failed to download the release: {e}"))?;
    Ok(response
        .bytes()
        .await
        .map_err(|e| format!("failed to read the download: {e}"))?
        .to_vec())
}

/// The release asset for a platform, named as `release.yml` uploads it.
fn asset_name(os: &str, arch: &str) -> Option<String> {
    let arch = match arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return None,
    };
    match os {
        "linux" => Some(format!("silva-linux-{arch}.tar.gz")),
        "macos" => Some(format!("silva-macos-{arch}.tar.gz")),
        "windows" => Some(format!("silva-windows-{arch}.exe.zip")),
        _ => None,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Reads a digest out of a `.sha256` file, in either `shasum` or bare form.
fn parse_digest(published: &str) -> Option<String> {
    let token = published.split_whitespace().next()?.to_ascii_lowercase();
    let looks_like_a_digest = token.len() == 64 && token.chars().all(|c| c.is_ascii_hexdigit());
    looks_like_a_digest.then_some(token)
}

#[cfg(unix)]
fn extract(bytes: &[u8], into: &Path) -> Result<PathBuf, String> {
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes));
    archive
        .unpack(into)
        .map_err(|e| format!("cannot extract the release archive: {e}"))?;

    let binary = into.join("silva");
    if !binary.is_file() {
        return Err("the release archive contains no silva binary".to_string());
    }
    Ok(binary)
}

#[cfg(windows)]
fn extract(bytes: &[u8], into: &Path) -> Result<PathBuf, String> {
    let archive = into.join("silva.zip");
    fs::write(&archive, bytes).map_err(|e| format!("cannot write the download: {e}"))?;

    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"])
        .arg(format!(
            "Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
            archive.display(),
            into.display()
        ))
        .status()
        .map_err(|e| format!("cannot run Expand-Archive: {e}"))?;
    if !status.success() {
        return Err("Expand-Archive could not read the release archive".to_string());
    }

    let binary = into.join("silva.exe");
    if !binary.is_file() {
        return Err("the release archive contains no silva.exe".to_string());
    }
    Ok(binary)
}

/// The install directory and binary name silva is running from.
fn install_location(exe: &Path) -> Result<(&Path, &str), String> {
    let dir = exe
        .parent()
        .ok_or("cannot locate the directory silva is installed in")?;
    let name = exe
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("cannot read silva's own file name")?;
    Ok((dir, name))
}

/// Puts the new binary in place of the running one.
///
/// Staging inside the destination directory and renaming is what makes this
/// safe to do unattended: `rename(2)` is atomic, so no invocation ever sees a
/// half-written binary, and the process running right now keeps executing the
/// inode it started from. A directory that cannot be written is reported rather
/// than forced — a system-wide or packaged install belongs to whoever installed
/// it.
#[cfg(unix)]
fn activate(new_binary: &Path, exe: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let (dir, name) = install_location(exe)?;
    sweep_stale_staging(dir, name);

    let staged = dir.join(format!(".{name}.new.{}", std::process::id()));
    fs::copy(new_binary, &staged).map_err(|e| format!("cannot write to {}: {e}", dir.display()))?;
    if let Err(e) = fs::set_permissions(&staged, fs::Permissions::from_mode(0o755)) {
        let _ = fs::remove_file(&staged);
        return Err(format!("cannot make the new binary executable: {e}"));
    }

    keep_previous(exe, dir, name);

    fs::rename(&staged, exe).map_err(|e| {
        let _ = fs::remove_file(&staged);
        format!("cannot install to {}: {e}", exe.display())
    })
}

/// Windows will not overwrite a running image, but it will let it be renamed,
/// so the outgoing binary is moved aside and put back if the copy fails.
#[cfg(windows)]
fn activate(new_binary: &Path, exe: &Path) -> Result<(), String> {
    let (dir, name) = install_location(exe)?;
    sweep_stale_staging(dir, name);

    let previous = dir.join(format!("{name}.old"));
    let _ = fs::remove_file(&previous);
    let moved_aside = fs::rename(exe, &previous).is_ok();

    match fs::copy(new_binary, exe) {
        Ok(_) => Ok(()),
        Err(e) => {
            if moved_aside {
                let _ = fs::rename(&previous, exe);
            }
            Err(format!("cannot install to {}: {e}", exe.display()))
        }
    }
}

/// Keeps the outgoing binary for `--rollback`.
///
/// A hard link costs nothing and leaves the running process's inode alone; a
/// copy is the fallback where links are unavailable. Failing to keep it does not
/// fail the update — it only costs the ability to roll back.
#[cfg(unix)]
fn keep_previous(exe: &Path, dir: &Path, name: &str) {
    let previous = dir.join(format!("{name}.old"));
    let _ = fs::remove_file(&previous);
    if fs::hard_link(exe, &previous).is_err() {
        let _ = fs::copy(exe, &previous);
    }
}

/// Removes staging files left by an update that was abandoned mid-download.
fn sweep_stale_staging(dir: &Path, name: &str) {
    let prefix = format!(".{name}.new.");
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !file_name.starts_with(&prefix) {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .map(|modified| modified.elapsed().unwrap_or_default() > CACHE_TTL)
            .unwrap_or(false);
        if stale {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Restores the binary kept by the last update.
pub fn rollback() -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| format!("cannot locate silva itself: {e}"))?;
    let exe = fs::canonicalize(&exe).unwrap_or(exe);
    let (dir, name) = install_location(&exe)?;

    restore_previous(dir, name, &exe)?;

    let mut cache = Cache::load();
    let restored = cache.previous_version.take();
    // The restored binary is older than whatever the cache last learned, so the
    // next start must be free to find and install an update again — and must not
    // announce the update that was just undone.
    cache.installed_at = None;
    cache.pending_announcement = None;
    cache.save();

    Ok(match restored {
        Some(version) => format!("Rolled back to v{version}."),
        None => format!("Restored the previous binary at {}.", exe.display()),
    })
}

/// Moves the kept binary back over the current one.
///
/// Split out from [`rollback`] so it can be exercised against a directory that
/// is not the one this test binary is running from.
fn restore_previous(dir: &Path, name: &str, exe: &Path) -> Result<(), String> {
    let previous = dir.join(format!("{name}.old"));
    if !previous.is_file() {
        return Err(format!(
            "nothing to roll back to: no {} alongside the current binary",
            previous.display()
        ));
    }
    fs::rename(&previous, exe).map_err(|e| format!("cannot restore {}: {e}", exe.display()))
}

/// Says so, once, when this invocation is the first to run an installed update.
///
/// Costs one small file read, and no network request: this is what keeps an
/// automatic update from being an unannounced one.
pub fn announce_completed_update() {
    let mut cache = Cache::load();
    let Some(installed) = cache.pending_announcement.take() else {
        return;
    };
    // Only claim it if this really is the version that was installed — a
    // rollback, or a manual install of something else, makes the note obsolete
    // rather than true.
    if installed == VERSION {
        let from = cache
            .previous_version
            .as_deref()
            .unwrap_or("an earlier version");
        crate::events::note_line(format!(
            "Now running silva v{VERSION}, updated from v{from}."
        ));
    }
    cache.save();
}

/// What silva remembers between invocations about updating.
///
/// Persisted so that the common case — no new release, or one already
/// installed — costs no network request at all.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Cache {
    /// When GitHub was last asked, as seconds since the epoch.
    #[serde(default)]
    checked_at: Option<u64>,
    /// The newest release seen at that point.
    #[serde(default)]
    latest_version: Option<String>,
    /// When an install was last attempted, successfully or not.
    #[serde(default)]
    install_attempted_at: Option<u64>,
    /// When an install last succeeded.
    #[serde(default)]
    installed_at: Option<u64>,
    /// The version replaced by that install, for `--rollback` to name.
    #[serde(default)]
    previous_version: Option<String>,
    /// A version installed but not yet announced to the user.
    ///
    /// The install happens under the *old* binary, so the only invocation that
    /// can honestly say "you are now running this" is the next one. Kept here so
    /// that an update is never silent, even when the invocation that installed it
    /// exited before it could say anything.
    #[serde(default)]
    pending_announcement: Option<String>,
}

impl Cache {
    fn load() -> Self {
        let Some(path) = cache_path() else {
            return Self::default();
        };
        fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    fn save(&self) {
        let Some(path) = cache_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(encoded) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, encoded);
        }
    }

    /// The known latest version, if it was learned recently enough to trust.
    fn fresh_latest_version(&self) -> Option<String> {
        fresh(self.checked_at, now_secs()).then_some(())?;
        self.latest_version.clone()
    }

    /// Whether enough time has passed to try installing again.
    fn install_attempt_due(&self) -> bool {
        !fresh(self.install_attempted_at, now_secs())
    }

    fn record_check(&mut self, latest_version: &str) {
        self.checked_at = Some(now_secs());
        self.latest_version = Some(latest_version.to_string());
    }

    fn record_install_attempt(&mut self) {
        self.install_attempted_at = Some(now_secs());
    }

    fn record_installed(&mut self, replaced_version: &str, installed_version: &str) {
        self.installed_at = Some(now_secs());
        self.previous_version = Some(replaced_version.to_string());
        self.pending_announcement = Some(installed_version.to_string());
    }
}

/// True if `at` is within [`CACHE_TTL`] of `now`.
///
/// A timestamp in the future — a clock that moved backwards — counts as stale,
/// so a bad clock cannot switch updating off indefinitely.
fn fresh(at: Option<u64>, now: u64) -> bool {
    match at {
        Some(at) => now >= at && now - at < CACHE_TTL.as_secs(),
        None => false,
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

/// Where the update cache lives, following the platform's cache convention.
fn cache_path() -> Option<PathBuf> {
    let dir = if cfg!(windows) {
        PathBuf::from(std::env::var("LOCALAPPDATA").ok()?)
    } else if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(xdg)
    } else {
        PathBuf::from(std::env::var("HOME").ok()?).join(".cache")
    };
    Some(dir.join("silva").join("update-check.json"))
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

    #[test]
    fn auto_is_the_default_for_every_kind_of_invocation() {
        assert_eq!(
            UpdateMode::resolve_from(false, None, false),
            UpdateMode::Auto
        );
    }

    #[test]
    fn the_flag_beats_everything() {
        assert_eq!(
            UpdateMode::resolve_from(true, Some("auto"), false),
            UpdateMode::Off
        );
    }

    #[test]
    fn an_explicit_mode_beats_a_blanket_opt_out() {
        // Somebody who sets SILVA_UPDATE=auto in CI meant it.
        assert_eq!(
            UpdateMode::resolve_from(false, Some("auto"), true),
            UpdateMode::Auto
        );
        assert_eq!(
            UpdateMode::resolve_from(false, Some("notify"), false),
            UpdateMode::Notify
        );
        assert_eq!(
            UpdateMode::resolve_from(false, Some("off"), false),
            UpdateMode::Off
        );
    }

    #[test]
    fn a_blanket_opt_out_disables_updating() {
        assert_eq!(UpdateMode::resolve_from(false, None, true), UpdateMode::Off);
    }

    #[test]
    fn an_unparseable_mode_falls_through_to_the_other_rules() {
        assert_eq!(
            UpdateMode::resolve_from(false, Some("sometimes"), true),
            UpdateMode::Off
        );
        assert_eq!(
            UpdateMode::resolve_from(false, Some("sometimes"), false),
            UpdateMode::Auto
        );
    }

    #[test]
    fn only_a_patch_release_installs_itself() {
        let current = Version::parse("0.5.11").unwrap();
        assert!(same_patch_series(
            &current,
            &Version::parse("0.5.12").unwrap()
        ));
        assert!(!same_patch_series(
            &current,
            &Version::parse("0.6.0").unwrap()
        ));
        assert!(!same_patch_series(
            &current,
            &Version::parse("1.0.0").unwrap()
        ));
    }

    #[test]
    fn asset_names_match_the_release_workflow() {
        assert_eq!(
            asset_name("linux", "x86_64").unwrap(),
            "silva-linux-x86_64.tar.gz"
        );
        assert_eq!(
            asset_name("macos", "aarch64").unwrap(),
            "silva-macos-aarch64.tar.gz"
        );
        assert_eq!(
            asset_name("windows", "x86_64").unwrap(),
            "silva-windows-x86_64.exe.zip"
        );
        assert!(asset_name("freebsd", "x86_64").is_none());
        assert!(asset_name("linux", "riscv64").is_none());
    }

    #[test]
    fn digests_are_read_from_either_shasum_or_bare_form() {
        let digest = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert_eq!(parse_digest(digest).unwrap(), digest);
        assert_eq!(
            parse_digest(&format!("{digest}  silva-linux-x86_64.tar.gz\n")).unwrap(),
            digest
        );
        assert_eq!(
            parse_digest(&digest.to_ascii_uppercase()).unwrap(),
            digest,
            "a digest is compared lowercase"
        );
    }

    #[test]
    fn anything_that_is_not_a_digest_is_refused() {
        assert!(parse_digest("").is_none());
        assert!(parse_digest("Not Found").is_none());
        assert!(parse_digest("deadbeef").is_none(), "too short");
        assert!(
            parse_digest(&"z".repeat(64)).is_none(),
            "right length, not hex"
        );
    }

    #[test]
    fn sha256_matches_a_known_value() {
        // The canonical SHA-256 of the empty input.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"silva"),
            "d24e913a4107af875dc2ac3d419798f3794d00434e5059fbb68ac8d33626eaee"
        );
    }

    #[test]
    fn freshness_expires_and_distrusts_a_clock_that_went_backwards() {
        let ttl = CACHE_TTL.as_secs();
        assert!(fresh(Some(1_000_000), 1_000_000), "just checked");
        assert!(fresh(Some(1_000_000), 1_000_000 + ttl - 1));
        assert!(!fresh(Some(1_000_000), 1_000_000 + ttl), "expired");
        assert!(!fresh(Some(1_000_000), 999_999), "timestamp in the future");
        assert!(!fresh(None, 1_000_000), "never checked");
    }

    /// A release archive in the shape `release.yml` produces.
    #[cfg(unix)]
    fn tarball(entry: &str, contents: &[u8]) -> Vec<u8> {
        let mut header = tar::Header::new_gnu();
        header.set_path(entry).unwrap();
        header.set_size(contents.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();

        let mut builder = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        builder.append(&header, contents).unwrap();
        builder.into_inner().unwrap().finish().unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn extract_takes_the_binary_out_of_a_release_archive() {
        let into = tempfile::tempdir().unwrap();
        let binary = extract(&tarball("silva", b"new binary"), into.path()).unwrap();

        assert_eq!(binary, into.path().join("silva"));
        assert_eq!(fs::read(&binary).unwrap(), b"new binary");
    }

    #[cfg(unix)]
    #[test]
    fn an_archive_without_a_silva_binary_is_refused() {
        let into = tempfile::tempdir().unwrap();
        assert!(extract(&tarball("README", b"not a binary"), into.path()).is_err());
    }

    /// The whole point of the design: an unattended install has to be able to
    /// replace a binary that is executing at that moment, which a write to the
    /// same path cannot do.
    #[cfg(unix)]
    #[test]
    fn activate_replaces_a_binary_that_is_running_right_now() {
        let Some(long_lived) = ["/bin/sleep", "/usr/bin/sleep"]
            .into_iter()
            .map(Path::new)
            .find(|candidate| candidate.is_file())
        else {
            return;
        };

        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("silva");
        fs::copy(long_lived, &exe).unwrap();

        let mut running = std::process::Command::new(&exe).arg("30").spawn().unwrap();

        let incoming = dir.path().join("incoming");
        fs::write(&incoming, b"the new binary").unwrap();

        // Writing to the path in place is what fails while it is executing.
        let mut refused = false;
        for _ in 0..50 {
            match fs::copy(&incoming, &exe) {
                Err(_) => {
                    refused = true;
                    break;
                }
                // Lost the race with exec: put the executable back and retry.
                Ok(_) => {
                    fs::copy(long_lived, &exe).unwrap();
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        }
        assert!(refused, "expected ETXTBSY while the binary is executing");

        activate(&incoming, &exe).expect("staging and renaming works on a live binary");

        let still_running = running.try_wait().unwrap().is_none();
        running.kill().ok();
        running.wait().ok();

        assert!(still_running, "the running process was not disturbed");
        assert_eq!(fs::read(&exe).unwrap(), b"the new binary");
        assert!(
            dir.path().join("silva.old").is_file(),
            "the outgoing binary is kept for --rollback"
        );
        assert!(
            !fs::read_dir(dir.path()).unwrap().any(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".silva.new.")
            }),
            "no staging file is left behind"
        );
    }

    /// Serves one release: the digest first, then the archive.
    ///
    /// Enough of an HTTP server to exercise [`install`] for real — the download,
    /// the verification and the replacement — without reaching GitHub.
    #[cfg(unix)]
    async fn serve_release(digest_body: String, archive: Vec<u8>) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());

        tokio::spawn(async move {
            for _ in 0..2 {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                let mut request = vec![0u8; 4096];
                let read = socket.read(&mut request).await.unwrap_or(0);
                let asked_for_digest =
                    String::from_utf8_lossy(&request[..read]).contains(".sha256");

                let body = if asked_for_digest {
                    digest_body.clone().into_bytes()
                } else {
                    archive.clone()
                };
                // Closing rather than keeping alive, so the next request is a
                // fresh accept rather than a reuse this server cannot handle.
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                socket.write_all(head.as_bytes()).await.ok();
                socket.write_all(&body).await.ok();
                socket.flush().await.ok();
            }
        });

        base
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_verified_download_is_downloaded_verified_and_installed() {
        let archive = tarball("silva", b"downloaded binary");
        let asset = asset_name(std::env::consts::OS, std::env::consts::ARCH).unwrap();
        let base = serve_release(format!("{}  {asset}\n", sha256_hex(&archive)), archive).await;

        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("silva");
        fs::write(&exe, b"old binary").unwrap();

        install(&base, "0.5.12", &exe).await.unwrap();

        assert_eq!(fs::read(&exe).unwrap(), b"downloaded binary");
        assert!(dir.path().join("silva.old").is_file());
    }

    /// The guardrail that replaces the confirmation prompt: a download that does
    /// not match its published digest is not installed at all.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_download_that_fails_verification_is_not_installed() {
        let asset = asset_name(std::env::consts::OS, std::env::consts::ARCH).unwrap();
        let wrong = "0".repeat(64);
        let base = serve_release(
            format!("{wrong}  {asset}\n"),
            tarball("silva", b"tampered binary"),
        )
        .await;

        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("silva");
        fs::write(&exe, b"old binary").unwrap();

        let refused = install(&base, "0.5.12", &exe).await.unwrap_err();

        assert!(refused.contains("checksum mismatch"), "got: {refused}");
        assert_eq!(
            fs::read(&exe).unwrap(),
            b"old binary",
            "the binary in place was left alone"
        );
        assert!(!dir.path().join("silva.old").exists());
    }

    #[cfg(unix)]
    #[test]
    fn rollback_puts_the_kept_binary_back() {
        let dir = tempfile::tempdir().unwrap();
        let exe = dir.path().join("silva");
        fs::write(&exe, b"current").unwrap();
        fs::write(dir.path().join("silva.old"), b"previous").unwrap();

        restore_previous(dir.path(), "silva", &exe).unwrap();

        assert_eq!(fs::read(&exe).unwrap(), b"previous");
        assert!(
            !dir.path().join("silva.old").exists(),
            "the kept binary is consumed, so a second rollback has nothing to undo"
        );
        assert!(restore_previous(dir.path(), "silva", &exe).is_err());
    }

    #[test]
    fn a_cache_without_a_recent_check_forces_a_probe() {
        let mut cache = Cache::default();
        assert!(cache.fresh_latest_version().is_none());
        assert!(
            cache.install_attempt_due(),
            "a fresh cache has attempted nothing"
        );

        cache.record_check("0.9.9");
        assert_eq!(cache.fresh_latest_version().unwrap(), "0.9.9");

        cache.record_install_attempt();
        assert!(
            !cache.install_attempt_due(),
            "an attempt just made is not retried"
        );
    }
}
