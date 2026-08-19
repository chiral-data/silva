//! Machine-readable output for headless runs.
//!
//! A headless run tells a person plenty and a program almost nothing: without
//! `--json` the only reliable signal a caller gets is the process exit code, so
//! learning *which* job failed means scraping prose that was never meant to be
//! an interface.
//!
//! [`Emitter`] is the one place run output is produced. In [`OutputFormat::Human`]
//! it prints exactly what it always printed; in [`OutputFormat::Json`] it writes
//! newline-delimited JSON to stdout, one object per line, flushed per event so a
//! consumer reading incrementally sees progress as it happens.

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};

use crate::components::docker::{job::JobStatus, logs::LogSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Human,
    Json,
}

/// Set once, when the emitter is created. The run's helper functions produce
/// diagnostics from several places and threading an emitter through all of them
/// would be worse than a flag they can consult.
static JSON_MODE: AtomicBool = AtomicBool::new(false);

pub fn json_mode() -> bool {
    JSON_MODE.load(Ordering::Relaxed)
}

/// Set before anything is printed. [`Emitter::new`] does this too, but the
/// deprecation notice for `silva <WORKFLOW_PATH>` is emitted before a run
/// begins, and it must not land on stdout as bare text.
pub fn set_json_mode(on: bool) {
    JSON_MODE.store(on, Ordering::Relaxed);
}

/// A diagnostic line from the run itself, rather than from a container. In JSON
/// mode it becomes a `note` event: dropping it would lose information, and
/// printing it raw would put a non-JSON line on a stream a caller is parsing.
pub fn note_line(text: String) {
    if json_mode() {
        emit(
            serde_json::json!({ "event": "note", "level": "info", "text": text, "at": Utc::now().to_rfc3339() }),
        );
    } else {
        println!("{text}");
    }
}

pub fn warn_line(text: String) {
    if json_mode() {
        emit(
            serde_json::json!({ "event": "note", "level": "warning", "text": text, "at": Utc::now().to_rfc3339() }),
        );
    } else {
        eprintln!("{text}");
    }
}

/// One JSON object per line on stdout, flushed so a consumer reading
/// incrementally sees progress as it happens.
fn emit(value: serde_json::Value) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{value}");
    let _ = stdout.flush();
}

/// `println!`, except it becomes a `note` event under `--json`.
#[macro_export]
macro_rules! note {
    ($($arg:tt)*) => { $crate::events::note_line(format!($($arg)*)) };
}

/// `eprintln!`, except it becomes a warning-level `note` event under `--json`.
#[macro_export]
macro_rules! warn_note {
    ($($arg:tt)*) => { $crate::events::warn_line(format!($($arg)*)) };
}

/// Job lifecycle as a consumer sees it. Derived from [`JobStatus`], which
/// carries container detail a caller has no use for.
fn status_name(status: &JobStatus) -> &'static str {
    match status {
        JobStatus::Idle => "idle",
        JobStatus::Pending => "pending",
        JobStatus::PullingImage => "pulling_image",
        JobStatus::BuildingImage => "building_image",
        JobStatus::CreatingContainer => "creating_container",
        JobStatus::ContainerRunning(_) | JobStatus::Running => "running",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
    }
}

pub struct Emitter {
    format: OutputFormat,
    /// Human mode prints a header when the run moves to another job.
    current_job: Option<String>,
    /// JSON mode emits a job event only when its phase actually changes: the
    /// executor reports `Completed` once per script, which is progress rather
    /// than a job finishing.
    last_status: Option<(usize, &'static str)>,
}

impl Emitter {
    pub fn new(format: OutputFormat) -> Self {
        JSON_MODE.store(format == OutputFormat::Json, Ordering::Relaxed);
        Self {
            format,
            current_job: None,
            last_status: None,
        }
    }

    pub fn is_json(&self) -> bool {
        self.format == OutputFormat::Json
    }

    fn write(&self, value: serde_json::Value) {
        emit(value);
    }

    fn now() -> String {
        Utc::now().to_rfc3339()
    }

    fn stamp(at: DateTime<Utc>) -> String {
        at.to_rfc3339()
    }

    /// Environment facts, before the run proper. Human only — a consumer that
    /// wants the temp folder gets it on the terminal `workflow` event.
    pub fn preamble(&self, docker_socket: &str, workflow_name: &str, temp_folder: &Path) {
        if self.is_json() {
            return;
        }
        println!("Docker socket: {docker_socket}");
        println!("Running workflow: {workflow_name}");
        println!("Temp folder: {}", temp_folder.display());
    }

    pub fn jobs_found(&self, count: usize) {
        if self.is_json() {
            return;
        }
        println!("Found {count} job(s)");
    }

    pub fn global_params_loaded(&self, count: usize) {
        if self.is_json() || count == 0 {
            return;
        }
        println!("Loaded {count} global workflow parameter(s)");
    }

    /// Printed as soon as the graph is sorted, which is where it has always
    /// appeared. The machine-readable counterpart is [`Emitter::workflow_started`],
    /// which waits until the run is actually about to begin.
    pub fn execution_order(&self, execution_order: &[String]) {
        if self.is_json() {
            return;
        }
        println!("Execution order: {}", execution_order.join(" -> "));
    }

    /// The run is about to begin: prechecks have passed and inputs are staged.
    /// A workflow rejected before this point never reports itself as started.
    pub fn workflow_started(&self, workflow_name: &str, execution_order: &[String]) {
        if self.is_json() {
            self.write(serde_json::json!({
                "event": "workflow",
                "status": "started",
                "workflow": workflow_name,
                "jobs": execution_order,
                "at": Self::now(),
            }));
            return;
        }
        println!();
    }

    /// A job's phase changed. Repeats of the same phase are dropped in JSON
    /// mode; in human mode this prints only the header shown when the run moves
    /// to another job — the status line follows the log line, as it always has,
    /// and is printed by [`Emitter::job_status_line`].
    /// Whether this message is a phase transition worth reporting. `Completed`
    /// and `Failed` never are: the executor sends `Completed` once per script,
    /// and a job's terminal state is decided by the run moving on — see
    /// [`Emitter::job_finished`].
    fn should_report_phase(&mut self, index: usize, status: &JobStatus) -> bool {
        if matches!(status, JobStatus::Completed | JobStatus::Failed) {
            return false;
        }
        let name = status_name(status);
        if self.last_status == Some((index, name)) {
            return false;
        }
        self.last_status = Some((index, name));
        true
    }

    pub fn job_phase(&mut self, job: &str, index: usize, status: &JobStatus, at: DateTime<Utc>) {
        if self.is_json() {
            let name = status_name(status);
            if !self.should_report_phase(index, status) {
                return;
            }
            self.write(serde_json::json!({
                "event": "job",
                "job": job,
                "index": index,
                "status": name,
                "at": Self::stamp(at),
            }));
            return;
        }

        if self.current_job.as_deref() != Some(job) {
            if self.current_job.is_some() {
                println!();
            }
            println!("=== Job: {job} ===");
            self.current_job = Some(job.to_string());
        }
    }

    /// Human mode's per-message status line, printed after the log line exactly
    /// as it was before events existed. JSON mode reports terminal state once
    /// per job instead — see [`Emitter::job_finished`].
    pub fn job_status_line(&self, job: &str, status: &JobStatus) {
        if self.is_json() {
            return;
        }
        match status {
            JobStatus::Completed => println!("[{job}] Completed"),
            JobStatus::Failed => eprintln!("[{job}] Failed"),
            _ => {}
        }
    }

    pub fn log(
        &mut self,
        job: &str,
        index: usize,
        source: LogSource,
        line: &str,
        at: DateTime<Utc>,
    ) {
        if line.is_empty() {
            return;
        }
        if self.is_json() {
            self.write(serde_json::json!({
                "event": "log",
                "job": job,
                "index": index,
                "stream": match source {
                    LogSource::Stdout => "stdout",
                    LogSource::Stderr => "stderr",
                },
                "text": line,
                "at": Self::stamp(at),
            }));
            return;
        }
        match source {
            LogSource::Stdout => println!("{line}"),
            LogSource::Stderr => eprintln!("{line}"),
        }
    }

    /// One terminal event per job, emitted when the run moves on or ends.
    /// `error` carries silva's own failure message rather than leaving a caller
    /// to read it out of the logs.
    pub fn job_finished(&mut self, job: &str, index: usize, failed: bool, error: Option<&str>) {
        if !self.is_json() {
            return;
        }
        self.write(serde_json::json!({
            "event": "job",
            "job": job,
            "index": index,
            "status": if failed { "failed" } else { "completed" },
            "error": error,
            "at": Self::now(),
        }));
    }

    /// Jobs the run never reached. Without this they are simply absent, and a
    /// caller cannot tell "never ran" from "not part of this workflow".
    pub fn job_skipped(&self, job: &str, index: usize, reason: &str) {
        if !self.is_json() {
            return;
        }
        self.write(serde_json::json!({
            "event": "job",
            "job": job,
            "index": index,
            "status": "skipped",
            "reason": reason,
            "at": Self::now(),
        }));
    }

    /// The run was interrupted (Ctrl-C in headless mode) rather than completing
    /// or failing on its own. Kept distinct from [`Emitter::workflow_finished`]
    /// so a consumer reading the event stream can tell a deliberate interrupt
    /// apart from a crash instead of just seeing the stream stop mid-job.
    pub fn workflow_cancelled(&self, output_dir: &Path) {
        if self.is_json() {
            self.write(serde_json::json!({
                "event": "workflow",
                "status": "cancelled",
                "error": serde_json::Value::Null,
                "outputDir": output_dir.display().to_string(),
                "at": Self::now(),
            }));
            return;
        }

        println!();
        eprintln!("Workflow cancelled");
        println!();
        println!("Working folder: {}", output_dir.display());
        println!("  (You can inspect this folder to debug the issue)");
    }

    pub fn workflow_finished(&self, result: &Result<(), String>, output_dir: &Path) {
        if self.is_json() {
            self.write(serde_json::json!({
                "event": "workflow",
                "status": if result.is_ok() { "completed" } else { "failed" },
                "error": result.as_ref().err(),
                "outputDir": output_dir.display().to_string(),
                "at": Self::now(),
            }));
            return;
        }

        println!();
        match result {
            Ok(()) => {
                println!("Workflow completed successfully");
                println!();
                println!("Output folder: {}", output_dir.display());
                println!("  (This folder will persist until you delete it manually)");
            }
            Err(e) => {
                eprintln!("Workflow failed: {e}");
                println!();
                println!("Working folder: {}", output_dir.display());
                println!("  (You can inspect this folder to debug the issue)");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn container_detail_is_not_part_of_the_reported_status() {
        assert_eq!(
            status_name(&JobStatus::ContainerRunning("abc123".to_string())),
            status_name(&JobStatus::Running)
        );
    }

    /// `--json` promises that every line on stdout parses. These cover the
    /// decisions that promise rests on; the stream itself is exercised by
    /// running a real workflow, which needs Docker.
    #[test]
    #[serial]
    fn json_mode_is_off_for_a_human_run() {
        let _e = Emitter::new(OutputFormat::Human);
        assert!(!json_mode());
    }

    #[test]
    #[serial]
    fn creating_a_json_emitter_switches_diagnostics_to_events() {
        let _e = Emitter::new(OutputFormat::Json);
        assert!(json_mode());
        set_json_mode(false);
    }

    #[test]
    #[serial]
    fn a_repeated_phase_is_reported_once() {
        let mut emitter = Emitter::new(OutputFormat::Json);
        // `Running` arrives many times per job; a consumer wants the transition.
        assert!(emitter.should_report_phase(0, &JobStatus::Running));
        assert!(!emitter.should_report_phase(0, &JobStatus::Running));
        assert!(emitter.should_report_phase(0, &JobStatus::PullingImage));
        // A different job at the same phase is its own transition.
        assert!(emitter.should_report_phase(1, &JobStatus::PullingImage));
        set_json_mode(false);
    }

    #[test]
    #[serial]
    fn terminal_statuses_are_not_reported_as_phases() {
        let mut emitter = Emitter::new(OutputFormat::Json);
        // The executor sends `Completed` once per script — that is progress,
        // not the job finishing, so it is never a phase event.
        assert!(!emitter.should_report_phase(0, &JobStatus::Completed));
        assert!(!emitter.should_report_phase(0, &JobStatus::Failed));
        set_json_mode(false);
    }

    #[test]
    fn every_status_has_a_snake_case_name() {
        for status in [
            JobStatus::Idle,
            JobStatus::Pending,
            JobStatus::PullingImage,
            JobStatus::BuildingImage,
            JobStatus::CreatingContainer,
            JobStatus::Running,
            JobStatus::Completed,
            JobStatus::Failed,
        ] {
            let name = status_name(&status);
            assert!(!name.is_empty());
            assert_eq!(name, name.to_lowercase());
            assert!(!name.contains(' '));
        }
    }
}
