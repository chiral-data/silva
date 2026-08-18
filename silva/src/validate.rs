//! Static validation of a workflow folder — no containers, no Docker, no run.
//!
//! `silva validate <dir>` answers one question: would this folder run? It parses
//! `workflow.toml` and every `job.toml`, checks the dependency graph, applies
//! the same prechecks headless execution applies, and validates any parameter
//! files against their definitions.
//!
//! It exists so that a tool which *writes* workflow folders — `chiral-cli`
//! materializing a workflow from the platform — can have them judged by the
//! schema that owns them, rather than growing a second copy of the rules.

use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use crate::components::workflow::{JobFolder, JobScanner, WorkflowFolder};
use crate::headless::topological_sort_jobs;

/// One problem, addressed to whoever has to fix it.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    /// Machine-readable category: `workflow`, `job`, `dependency`, `params`,
    /// `script`, or `inputs`.
    pub kind: &'static str,
    /// Path this is about, relative to the workflow folder where possible.
    pub file: Option<String>,
    /// Job folder this is about, when it is about one.
    pub job: Option<String>,
    pub message: String,
}

impl Finding {
    fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            file: None,
            job: None,
            message: message.into(),
        }
    }

    fn at(mut self, file: impl Into<String>) -> Self {
        self.file = Some(file.into());
        self
    }

    fn in_job(mut self, job: impl Into<String>) -> Self {
        self.job = Some(job.into());
        self
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub workflow_name: Option<String>,
    /// Job folders found, in scan order.
    pub jobs: Vec<String>,
    /// Execution order, when the graph is sound enough to produce one.
    pub order: Vec<String>,
    pub findings: Vec<Finding>,
}

impl Report {
    pub fn is_valid(&self) -> bool {
        self.findings.is_empty()
    }

    /// Human output. Success names the execution order, because that is the
    /// thing a person actually wants confirmed.
    pub fn render(&self) -> String {
        if self.is_valid() {
            let name = self.workflow_name.as_deref().unwrap_or("workflow");
            return format!(
                "{name}: {} job(s) ok\nExecution order: {}\n",
                self.jobs.len(),
                self.order.join(" -> ")
            );
        }

        let mut out = format!("{} problem(s) found:\n\n", self.findings.len());
        for finding in &self.findings {
            let where_ = match (&finding.file, &finding.job) {
                (Some(file), _) => format!(" [{file}]"),
                (None, Some(job)) => format!(" [{job}]"),
                (None, None) => String::new(),
            };
            // The prechecks return multi-line messages; indent continuations so
            // one finding still reads as one item.
            let message = finding.message.trim_end().replace('\n', "\n    ");
            out.push_str(&format!("  {}{}: {message}\n", finding.kind, where_));
        }
        out
    }

    /// Machine output, for a caller that has to act on the result.
    pub fn to_json(&self) -> String {
        let findings: Vec<serde_json::Value> = self
            .findings
            .iter()
            .map(|f| {
                serde_json::json!({
                    "kind": f.kind,
                    "file": f.file,
                    "job": f.job,
                    "message": f.message,
                })
            })
            .collect();

        serde_json::to_string_pretty(&serde_json::json!({
            "valid": self.is_valid(),
            "workflow": self.workflow_name,
            "jobs": self.jobs,
            "order": self.order,
            "findings": findings,
        }))
        .unwrap_or_else(|e| format!("{{\"valid\":false,\"error\":\"{e}\"}}"))
    }
}

/// Validates a workflow folder in place. Never runs anything and never needs
/// Docker.
pub fn validate_workflow(workflow_path: &Path) -> Report {
    let mut report = Report::default();

    let workflow_path = match workflow_path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            report.findings.push(Finding::new(
                "workflow",
                format!("Cannot read {}: {e}", workflow_path.display()),
            ));
            return report;
        }
    };

    if !workflow_path.is_dir() {
        report.findings.push(Finding::new(
            "workflow",
            format!("Not a directory: {}", workflow_path.display()),
        ));
        return report;
    }

    let workflow_name = workflow_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workflow")
        .to_string();

    let folder = WorkflowFolder::new(
        workflow_name.clone(),
        workflow_path.clone(),
        Some(SystemTime::now()),
    );

    // 1. workflow.toml — the one file whose absence stops everything else.
    let metadata = match folder.load_workflow_metadata() {
        Ok(Some(meta)) => {
            report.workflow_name = Some(meta.name.clone());
            meta
        }
        Ok(None) => {
            report.findings.push(
                Finding::new("workflow", "No workflow metadata found.").at(".chiral/workflow.toml"),
            );
            return report;
        }
        Err(e) => {
            report
                .findings
                .push(Finding::new("workflow", format!("{e}")).at(".chiral/workflow.toml"));
            return report;
        }
    };

    // 2. Job folders.
    let jobs = match JobScanner::scan_jobs(&workflow_path) {
        Ok(jobs) => jobs,
        Err(e) => {
            report
                .findings
                .push(Finding::new("workflow", format!("Cannot scan jobs: {e}")));
            return report;
        }
    };
    report.jobs = jobs.iter().map(|j| j.name.clone()).collect();

    if jobs.is_empty() {
        report.findings.push(Finding::new(
            "workflow",
            "No jobs found. A job is a folder containing .chiral/job.toml.",
        ));
        return report;
    }

    // 3. Every job.toml parses. Jobs that do not are excluded from later checks,
    //    so one broken file does not produce a cascade of derived complaints.
    let mut metas: HashMap<String, job_config::job::JobMeta> = HashMap::new();
    for job in &jobs {
        match job.load_meta() {
            Ok(meta) => {
                metas.insert(job.name.clone(), meta);
            }
            Err(e) => report.findings.push(
                Finding::new("job", format!("{e}"))
                    .at(format!("{}/.chiral/job.toml", job.name))
                    .in_job(&job.name),
            ),
        }
    }
    let parsed: Vec<JobFolder> = jobs
        .iter()
        .filter(|j| metas.contains_key(&j.name))
        .cloned()
        .collect();

    // 4. Dependencies: names that resolve, and no cycles.
    //
    // The name check runs first and reports every bad name; the topological
    // sort reports only the first it meets, in its own words, so running both
    // would say the same thing twice. Once the names are sound the sort has
    // nothing left to find but cycles, which is what it is here for.
    let names_before = report.findings.len();
    check_dependency_names(&metadata, &report.jobs, &mut report.findings);
    if report.findings.len() == names_before {
        match topological_sort_jobs(&parsed, &metadata) {
            Ok(sorted) => report.order = sorted.iter().map(|j| j.name.clone()).collect(),
            Err(e) => report.findings.push(Finding::new("dependency", e)),
        }
    }

    // 5. Parameter files, where they exist, against their definitions. This is
    //    what catches a generated params.json that names a parameter the job
    //    does not have.
    if let Ok(Some(params)) = folder.load_workflow_params()
        && let Err(e) = metadata.validate_params(&params)
    {
        report
            .findings
            .push(Finding::new("params", e).at("global_params.json"));
    }
    for job in &parsed {
        let Some(meta) = metas.get(&job.name) else {
            continue;
        };
        match job.load_params() {
            Ok(Some(params)) => {
                if let Err(e) = meta.validate_params(&params) {
                    report.findings.push(
                        Finding::new("params", e)
                            .at(format!("{}/params.json", job.name))
                            .in_job(&job.name),
                    );
                }
            }
            Ok(None) => {}
            Err(e) => report.findings.push(
                Finding::new("params", format!("{e}"))
                    .at(format!("{}/params.json", job.name))
                    .in_job(&job.name),
            ),
        }
    }

    // 6. The same conventions headless execution enforces, so validate agrees
    //    with what a run would do.
    if let Err(e) = crate::precheck::check_install_commands(&parsed) {
        report.findings.push(Finding::new("script", e));
    }
    if let Err(e) = crate::precheck::check_cross_node_references(&parsed) {
        report.findings.push(Finding::new("script", e));
    }
    if let Err(e) = crate::precheck::check_input_files_folder(&workflow_path, &parsed, &metadata) {
        report
            .findings
            .push(Finding::new("inputs", e).at("input_files/"));
    }

    report
}

/// `[dependencies]` may name a job that does not exist — the topological sort
/// reports the first such name, so check them all here for one full list.
fn check_dependency_names(
    metadata: &job_config::workflow::WorkflowMeta,
    job_names: &[String],
    findings: &mut Vec<Finding>,
) {
    for (job, deps) in &metadata.dependencies {
        if !job_names.contains(job) {
            findings.push(
                Finding::new(
                    "dependency",
                    format!("[dependencies] names '{job}', which is not a job folder here."),
                )
                .at(".chiral/workflow.toml"),
            );
        }
        for dep in deps {
            if !job_names.contains(dep) {
                findings.push(
                    Finding::new(
                        "dependency",
                        format!("Job '{job}' depends on '{dep}', which is not a job folder here."),
                    )
                    .at(".chiral/workflow.toml")
                    .in_job(job),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Minimal workflow: `01-first` -> `02-second`, both with a trivial job.toml.
    fn workflow(dependencies: &str) -> TempDir {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join(".chiral")).unwrap();
        fs::write(
            root.join(".chiral/workflow.toml"),
            format!("name = \"Test\"\ndescription = \"a test workflow\"\n\n{dependencies}"),
        )
        .unwrap();

        for job in ["01-first", "02-second"] {
            fs::create_dir_all(root.join(job).join(".chiral")).unwrap();
            fs::write(
                root.join(job).join(".chiral/job.toml"),
                "name = \"Job\"\ndescription = \"a job\"\noutputs = [\"*.txt\"]\n\n\
                 [container]\nimage = \"alpine:latest\"\n\n\
                 [scripts]\nrun = \"run.sh\"\n\n\
                 [params.threshold]\ntype = \"float\"\ndefault = 1.0\nhint = \"a threshold\"\n",
            )
            .unwrap();
            fs::write(root.join(job).join("run.sh"), "#!/bin/bash\necho hi\n").unwrap();
        }

        // `01-first` has no dependencies, so input_files/ has to exist.
        fs::create_dir_all(root.join("input_files")).unwrap();
        dir
    }

    #[test]
    fn valid_workflow_reports_execution_order() {
        let dir = workflow("[dependencies]\n02-second = [\"01-first\"]\n");
        let report = validate_workflow(dir.path());

        assert!(report.is_valid(), "{:?}", report.findings);
        assert_eq!(report.order, vec!["01-first", "02-second"]);
        assert_eq!(report.workflow_name.as_deref(), Some("Test"));
    }

    #[test]
    fn missing_workflow_toml_is_reported_once() {
        let dir = TempDir::new().unwrap();
        let report = validate_workflow(dir.path());

        assert!(!report.is_valid());
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].kind, "workflow");
    }

    #[test]
    fn dependency_on_a_job_that_does_not_exist_names_the_job() {
        let dir = workflow("[dependencies]\n02-second = [\"01-missing\"]\n");
        let report = validate_workflow(dir.path());

        let deps: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.kind == "dependency")
            .collect();
        // Exactly one: the name check and the topological sort must not both
        // report the same missing dependency.
        assert_eq!(deps.len(), 1, "{:?}", report.findings);
        assert!(deps[0].message.contains("01-missing"));
        assert_eq!(deps[0].job.as_deref(), Some("02-second"));
    }

    #[test]
    fn dependencies_naming_an_unknown_job_are_reported() {
        let dir = workflow("[dependencies]\n03-ghost = [\"01-first\"]\n");
        let report = validate_workflow(dir.path());

        assert!(
            report
                .findings
                .iter()
                .any(|f| f.message.contains("03-ghost"))
        );
    }

    #[test]
    fn a_cycle_is_reported() {
        let dir =
            workflow("[dependencies]\n01-first = [\"02-second\"]\n02-second = [\"01-first\"]\n");
        let report = validate_workflow(dir.path());

        assert!(!report.is_valid());
        assert!(report.findings.iter().any(|f| f.kind == "dependency"));
        assert!(report.order.is_empty());
    }

    #[test]
    fn a_malformed_job_toml_is_reported_and_does_not_cascade() {
        let dir = workflow("[dependencies]\n02-second = [\"01-first\"]\n");
        fs::write(
            dir.path().join("02-second/.chiral/job.toml"),
            "name = \"broken\"\n",
        )
        .unwrap();

        let report = validate_workflow(dir.path());

        let jobs: Vec<&Finding> = report.findings.iter().filter(|f| f.kind == "job").collect();
        assert_eq!(jobs.len(), 1, "{:?}", report.findings);
        assert_eq!(jobs[0].job.as_deref(), Some("02-second"));
    }

    #[test]
    fn a_params_file_naming_an_unknown_parameter_is_reported() {
        let dir = workflow("[dependencies]\n02-second = [\"01-first\"]\n");
        fs::write(dir.path().join("01-first/params.json"), "{\"cutoff\": 2.0}").unwrap();

        let report = validate_workflow(dir.path());

        let params: Vec<&Finding> = report
            .findings
            .iter()
            .filter(|f| f.kind == "params")
            .collect();
        assert_eq!(params.len(), 1, "{:?}", report.findings);
        assert!(params[0].message.contains("cutoff"));
        assert_eq!(params[0].file.as_deref(), Some("01-first/params.json"));
    }

    #[test]
    fn a_params_file_matching_its_definitions_passes() {
        let dir = workflow("[dependencies]\n02-second = [\"01-first\"]\n");
        fs::write(
            dir.path().join("01-first/params.json"),
            "{\"threshold\": 2.5}",
        )
        .unwrap();

        let report = validate_workflow(dir.path());

        assert!(report.is_valid(), "{:?}", report.findings);
    }

    #[test]
    fn missing_input_files_folder_is_reported() {
        let dir = workflow("[dependencies]\n02-second = [\"01-first\"]\n");
        fs::remove_dir_all(dir.path().join("input_files")).unwrap();

        let report = validate_workflow(dir.path());

        assert!(report.findings.iter().any(|f| f.kind == "inputs"));
    }

    #[test]
    fn a_folder_with_no_jobs_is_reported() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".chiral")).unwrap();
        fs::write(
            dir.path().join(".chiral/workflow.toml"),
            "name = \"Empty\"\ndescription = \"no jobs\"\n",
        )
        .unwrap();

        let report = validate_workflow(dir.path());

        assert!(!report.is_valid());
        assert!(report.jobs.is_empty());
    }

    #[test]
    fn a_path_that_does_not_exist_is_reported_rather_than_panicking() {
        let report = validate_workflow(Path::new("/nonexistent/workflow/folder"));
        assert!(!report.is_valid());
        assert_eq!(report.findings[0].kind, "workflow");
    }

    #[test]
    fn json_output_carries_the_findings() {
        let dir = workflow("[dependencies]\n02-second = [\"01-missing\"]\n");
        let json = validate_workflow(dir.path()).to_json();

        assert!(json.contains("\"valid\": false"));
        assert!(json.contains("\"kind\": \"dependency\""));
        assert!(json.contains("01-missing"));
    }

    #[test]
    fn json_output_of_a_valid_workflow_reports_the_order() {
        let dir = workflow("[dependencies]\n02-second = [\"01-first\"]\n");
        let json = validate_workflow(dir.path()).to_json();

        assert!(json.contains("\"valid\": true"));
        assert!(json.contains("01-first"));
    }
}
