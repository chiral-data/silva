//! Sakura 高火力 DOK bundle preparation for `RUN_MODE=use_dok` jobs.
//!
//! `run_dok.sh` (inside a job's own script tree) consumes a presigned
//! `DOK_BUNDLE_URL` it has no way to produce itself, since DOK's task API has
//! no upload/mount endpoint. This module produces it: tar+gzip the job's own
//! directory (script files plus its already-merged `inputs/` subdirectory,
//! populated by the existing `copy_input_files_from_dependencies` step before
//! this runs — excluding only `outputs/`), base64-encode it, submit a tiny
//! prep task whose `command` embeds the payload and decodes+extracts it
//! directly into `$SAKURA_ARTIFACT_DIR`, then fetch a fresh presigned
//! download URL for the resulting DOK artifact.
//!
//! The payload travels in `command`, not `environment` — DOK's `environment`
//! field is capped at 8192 total characters across all keys+values (confirmed
//! live), which even a small script bundle exceeds. `command` was confirmed
//! live to accept 200KB+ with no rejection.

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

/// Prepares `DOK_BUNDLE_URL` for a job about to run with `RUN_MODE=use_dok`,
/// returning it as a ready-to-use `KEY=VALUE` env var string.
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

    let bundle_tar = build_tar_gz(job_path, &["outputs"])?;
    let bundle_url = prepare_bundle(&client, &token, &secret, bundle_tar).await?;

    Ok(vec![format!("DOK_BUNDLE_URL={bundle_url}")])
}

/// Builds an in-memory gzip tar archive of `dir`'s contents (flat, including
/// any nested subdirectories like `inputs/` — matching what `run_dok.sh`'s
/// remote `tar xz` expects), skipping any top-level entries named in
/// `exclude`.
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

/// Sends a request and parses its JSON body, including the response text in
/// the error on a non-2xx status or a parse failure — `reqwest`'s own
/// `error_for_status()` discards the body, which is exactly where the DOK
/// API's actual validation detail lives.
async fn api_json(request: reqwest::RequestBuilder) -> Result<serde_json::Value, String> {
    let resp = request
        .send()
        .await
        .map_err(|e| format!("DOK API request failed: {e}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read DOK API response body: {e}"))?;
    if !status.is_success() {
        return Err(format!("DOK API request rejected ({status}): {text}"));
    }
    serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse DOK API response: {e} (body: {text})"))
}

/// Submits a prep task whose `command` embeds a base64 tar.gz payload and
/// decodes+extracts it directly into `$SAKURA_ARTIFACT_DIR`, polls it to
/// completion, and returns a fresh presigned download URL for the resulting
/// artifact.
async fn prepare_bundle(
    client: &reqwest::Client,
    token: &str,
    secret: &str,
    tar_gz: Vec<u8>,
) -> Result<String, String> {
    let payload_b64 = base64::engine::general_purpose::STANDARD.encode(&tar_gz);
    let decode_script = format!(
        "import base64, os, tarfile, io\n\
         data = base64.b64decode(\"{payload_b64}\")\n\
         with tarfile.open(fileobj=io.BytesIO(data), mode=\"r:gz\") as tar:\n    \
             tar.extractall(os.environ[\"SAKURA_ARTIFACT_DIR\"])\n"
    );

    let task_name = format!("silva-dok-bundle-{}", unique_suffix());
    let body = json!({
        "name": task_name,
        "containers": [{
            "image": PREP_IMAGE,
            "command": ["python3", "-c", decode_script],
            "plan": PREP_PLAN,
        }]
    });

    let task = api_json(
        client
            .post(format!("{API_BASE}/tasks/"))
            .basic_auth(token, Some(secret))
            .json(&body),
    )
    .await?;

    let task_id = task["id"]
        .as_str()
        .ok_or("DOK task response missing id")?
        .to_string();

    let deadline = Instant::now() + POLL_TIMEOUT;
    let final_task = loop {
        let t = api_json(
            client
                .get(format!("{API_BASE}/tasks/{task_id}/"))
                .basic_auth(token, Some(secret)),
        )
        .await?;

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

    let download = api_json(
        client
            .get(format!("{API_BASE}/artifacts/{artifact_id}/download/"))
            .basic_auth(token, Some(secret)),
    )
    .await?;

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
    fn build_tar_gz_excludes_outputs_but_keeps_nested_inputs() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("run_gpu.sh"), b"echo hi").unwrap();
        std::fs::create_dir(tmp.path().join("inputs")).unwrap();
        std::fs::write(tmp.path().join("inputs").join("upstream.txt"), b"data").unwrap();
        std::fs::create_dir(tmp.path().join("outputs")).unwrap();
        std::fs::write(tmp.path().join("outputs").join("stale.txt"), b"old").unwrap();

        let archive = build_tar_gz(tmp.path(), &["outputs"]).unwrap();

        let decoder = flate2::read::GzDecoder::new(&archive[..]);
        let mut tar = tar::Archive::new(decoder);
        let names: Vec<String> = tar
            .entries()
            .unwrap()
            .map(|e| e.unwrap().path().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(names.contains(&"run_gpu.sh".to_string()));
        assert!(names.iter().any(|n| n.starts_with("inputs")));
        assert!(!names.iter().any(|n| n.starts_with("outputs")));
    }

    #[test]
    fn build_tar_gz_handles_no_inputs_dir() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("build_cell.py"), b"pass").unwrap();

        let archive = build_tar_gz(tmp.path(), &["outputs"]).unwrap();

        let decoder = flate2::read::GzDecoder::new(&archive[..]);
        let mut tar = tar::Archive::new(decoder);
        let names: Vec<String> = tar
            .entries()
            .unwrap()
            .map(|e| e.unwrap().path().unwrap().to_string_lossy().to_string())
            .collect();

        assert_eq!(names, vec!["build_cell.py".to_string()]);
    }
}
