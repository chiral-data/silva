//! Sakura 高火力 DOK bundle preparation for `RUN_MODE=use_dok` jobs.
//!
//! `run_dok.sh` (inside a job's own script tree) consumes two presigned URLs —
//! `DOK_SCRIPT_BUNDLE_URL` and `DOK_INPUT_BUNDLE_URL` — that it has no way to
//! produce itself, since DOK's task API has no upload/mount endpoint. This
//! module produces them: tar+gzip the job's script folder (and its already-
//! merged `inputs/` folder, if non-empty — populated by the existing
//! `copy_input_files_from_dependencies` step before this runs), base64-encode
//! each, submit a tiny prep task per bundle whose command decodes and
//! extracts it directly into `$SAKURA_ARTIFACT_DIR`, then fetch a fresh
//! presigned download URL for the resulting DOK artifact.

use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::json;

const API_BASE: &str = "https://secure.sakura.ad.jp/cloud/zone/is1a/api/managed-container/1.0";
const PREP_IMAGE: &str = "python:3.12-slim";
const PREP_PLAN: &str = "v100-32gb";
const POLL_INTERVAL: Duration = Duration::from_secs(5);
const POLL_TIMEOUT: Duration = Duration::from_secs(300);

/// Decodes `BUNDLE_B64` and extracts it directly into `$SAKURA_ARTIFACT_DIR`,
/// so DOK's own auto-packaged `artifact.tar.gz` output *is* exactly the
/// original bundle content — matching what `run_dok.sh` already expects.
const DECODE_SCRIPT: &str = r#"import base64, os, tarfile, io
data = base64.b64decode(os.environ["BUNDLE_B64"])
with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as tar:
    tar.extractall(os.environ["SAKURA_ARTIFACT_DIR"])
"#;

/// Reads `RUN_MODE` from `-e/--env` CLI values, or (if listed in
/// `env_passthrough`) from silva's own host environment.
pub fn resolve_run_mode(cli_env_vars: &[String], env_passthrough: &[String]) -> Option<String> {
    for entry in cli_env_vars.iter().rev() {
        if let Some((key, value)) = entry.split_once('=')
            && key == "RUN_MODE"
        {
            return Some(value.to_string());
        }
    }
    if env_passthrough.iter().any(|k| k == "RUN_MODE") {
        return std::env::var("RUN_MODE").ok();
    }
    None
}

/// Prepares `DOK_SCRIPT_BUNDLE_URL` and (if `inputs/` is non-empty)
/// `DOK_INPUT_BUNDLE_URL` for a job about to run with `RUN_MODE=use_dok`,
/// returning them as ready-to-use `KEY=VALUE` env var strings.
pub async fn prepare_bundle_env_vars(job_path: &Path) -> Result<Vec<String>, String> {
    let token = std::env::var("SAKURA_ACCESS_TOKEN").map_err(|_| {
        "RUN_MODE=use_dok requires SAKURA_ACCESS_TOKEN in silva's own environment".to_string()
    })?;
    let secret = std::env::var("SAKURA_ACCESS_TOKEN_SECRET").map_err(|_| {
        "RUN_MODE=use_dok requires SAKURA_ACCESS_TOKEN_SECRET in silva's own environment"
            .to_string()
    })?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let mut env_vars = Vec::new();

    let script_tar = build_tar_gz(job_path, &["inputs", "outputs"])?;
    let script_url = prepare_bundle(&client, &token, &secret, script_tar).await?;
    env_vars.push(format!("DOK_SCRIPT_BUNDLE_URL={script_url}"));

    let inputs_dir = job_path.join("inputs");
    if dir_has_entries(&inputs_dir) {
        let input_tar = build_tar_gz(&inputs_dir, &[])?;
        let input_url = prepare_bundle(&client, &token, &secret, input_tar).await?;
        env_vars.push(format!("DOK_INPUT_BUNDLE_URL={input_url}"));
    }

    Ok(env_vars)
}

fn dir_has_entries(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false)
}

/// Builds an in-memory gzip tar archive of `dir`'s contents (flat — matching
/// what `run_dok.sh`'s remote `tar xz` expects), skipping any top-level
/// entries named in `exclude`.
fn build_tar_gz(dir: &Path, exclude: &[&str]) -> Result<Vec<u8>, String> {
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = tar::Builder::new(encoder);

    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("Failed to read {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let name = entry.file_name();
        if exclude.iter().any(|ex| name == std::ffi::OsStr::new(ex)) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            builder
                .append_dir_all(&name, &path)
                .map_err(|e| format!("Failed to add directory {name:?}: {e}"))?;
        } else {
            let mut file = std::fs::File::open(&path)
                .map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
            builder
                .append_file(&name, &mut file)
                .map_err(|e| format!("Failed to add file {name:?}: {e}"))?;
        }
    }

    let encoder = builder
        .into_inner()
        .map_err(|e| format!("Failed to finalize tar: {e}"))?;
    encoder
        .finish()
        .map_err(|e| format!("Failed to finalize gzip: {e}"))
}

/// Submits a prep task that decodes+extracts a base64 tar.gz payload directly
/// into `$SAKURA_ARTIFACT_DIR`, polls it to completion, and returns a fresh
/// presigned download URL for the resulting artifact.
async fn prepare_bundle(
    client: &reqwest::Client,
    token: &str,
    secret: &str,
    tar_gz: Vec<u8>,
) -> Result<String, String> {
    let payload_b64 = base64::engine::general_purpose::STANDARD.encode(&tar_gz);

    let task_name = format!("silva-dok-bundle-{}", unique_suffix());
    let body = json!({
        "name": task_name,
        "containers": [{
            "image": PREP_IMAGE,
            "command": ["python3", "-c", DECODE_SCRIPT],
            "environment": {"BUNDLE_B64": payload_b64},
            "plan": PREP_PLAN,
        }]
    });

    let task: serde_json::Value = client
        .post(format!("{API_BASE}/tasks/"))
        .basic_auth(token, Some(secret))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to submit DOK prep task: {e}"))?
        .error_for_status()
        .map_err(|e| format!("DOK prep task submission rejected: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse DOK task response: {e}"))?;

    let task_id = task["id"]
        .as_str()
        .ok_or("DOK task response missing id")?
        .to_string();

    let deadline = Instant::now() + POLL_TIMEOUT;
    let final_task = loop {
        let t: serde_json::Value = client
            .get(format!("{API_BASE}/tasks/{task_id}/"))
            .basic_auth(token, Some(secret))
            .send()
            .await
            .map_err(|e| format!("Failed to poll DOK task {task_id}: {e}"))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse DOK task poll response: {e}"))?;

        let status = t["status"].as_str().unwrap_or("");
        if matches!(status, "done" | "error" | "aborted" | "canceled") {
            break t;
        }
        if Instant::now() > deadline {
            return Err(format!("Timed out waiting for DOK prep task {task_id}"));
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    };

    let status = final_task["status"].as_str().unwrap_or("");
    if status != "done" {
        return Err(format!(
            "DOK prep task {task_id} finished with status={status}: {}",
            final_task["error_message"].as_str().unwrap_or("")
        ));
    }

    let artifact_id = final_task["artifact"]["id"]
        .as_str()
        .ok_or_else(|| format!("DOK prep task {task_id} has no artifact"))?;

    let download: serde_json::Value = client
        .get(format!("{API_BASE}/artifacts/{artifact_id}/download/"))
        .basic_auth(token, Some(secret))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch DOK artifact download URL: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse DOK artifact download response: {e}"))?;

    download["url"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "DOK artifact download response missing url".to_string())
}

fn unique_suffix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn resolve_run_mode_from_cli_env() {
        let cli = vec!["FOO=bar".to_string(), "RUN_MODE=use_dok".to_string()];
        assert_eq!(resolve_run_mode(&cli, &[]), Some("use_dok".to_string()));
    }

    #[test]
    fn resolve_run_mode_last_cli_entry_wins() {
        let cli = vec!["RUN_MODE=mock".to_string(), "RUN_MODE=use_gpu".to_string()];
        assert_eq!(resolve_run_mode(&cli, &[]), Some("use_gpu".to_string()));
    }

    #[test]
    fn resolve_run_mode_absent_returns_none() {
        let cli = vec!["FOO=bar".to_string()];
        assert_eq!(resolve_run_mode(&cli, &[]), None);
    }

    #[test]
    #[serial]
    fn resolve_run_mode_falls_back_to_env_passthrough() {
        unsafe { std::env::set_var("RUN_MODE", "use_dok") };
        let result = resolve_run_mode(&[], &["RUN_MODE".to_string()]);
        unsafe { std::env::remove_var("RUN_MODE") };
        assert_eq!(result, Some("use_dok".to_string()));
    }

    #[test]
    #[serial]
    fn resolve_run_mode_cli_takes_precedence_over_env_passthrough() {
        unsafe { std::env::set_var("RUN_MODE", "mock") };
        let cli = vec!["RUN_MODE=use_dok".to_string()];
        let result = resolve_run_mode(&cli, &["RUN_MODE".to_string()]);
        unsafe { std::env::remove_var("RUN_MODE") };
        assert_eq!(result, Some("use_dok".to_string()));
    }

    #[test]
    fn build_tar_gz_excludes_named_entries_and_preserves_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("run_gpu.sh"), b"echo hi").unwrap();
        std::fs::create_dir(tmp.path().join("inputs")).unwrap();
        std::fs::write(tmp.path().join("inputs").join("upstream.txt"), b"data").unwrap();
        std::fs::create_dir(tmp.path().join("outputs")).unwrap();

        let archive = build_tar_gz(tmp.path(), &["inputs", "outputs"]).unwrap();

        let decoder = flate2::read::GzDecoder::new(&archive[..]);
        let mut tar = tar::Archive::new(decoder);
        let names: Vec<String> = tar
            .entries()
            .unwrap()
            .map(|e| e.unwrap().path().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(names.contains(&"run_gpu.sh".to_string()));
        assert!(!names.iter().any(|n| n.starts_with("inputs")));
        assert!(!names.iter().any(|n| n.starts_with("outputs")));
    }

    #[test]
    fn dir_has_entries_detects_empty_vs_populated() {
        let tmp = tempfile::tempdir().unwrap();
        let empty = tmp.path().join("empty");
        std::fs::create_dir(&empty).unwrap();
        assert!(!dir_has_entries(&empty));

        std::fs::write(empty.join("file.txt"), b"x").unwrap();
        assert!(dir_has_entries(&empty));

        assert!(!dir_has_entries(&tmp.path().join("does-not-exist")));
    }
}
