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
    /// Machine-readable category: `workflow`, `job`, `dependency`, `ports`,
    /// `params`, `script`, or `inputs`.
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
    /// Worth telling the author, but not a reason the folder would not run.
    pub notes: Vec<Finding>,
}

impl Report {
    pub fn is_valid(&self) -> bool {
        self.findings.is_empty()
    }

    /// Human output. Success names the execution order, because that is the
    /// thing a person actually wants confirmed.
    pub fn render(&self) -> String {
        let mut out = if self.is_valid() {
            let name = self.workflow_name.as_deref().unwrap_or("workflow");
            format!(
                "{name}: {} job(s) ok\nExecution order: {}\n",
                self.jobs.len(),
                self.order.join(" -> ")
            )
        } else {
            let mut out = format!("{} problem(s) found:\n\n", self.findings.len());
            for finding in &self.findings {
                out.push_str(&render_item(finding));
            }
            out
        };

        if !self.notes.is_empty() {
            out.push_str("\nNote(s):\n\n");
            for note in &self.notes {
                out.push_str(&render_item(note));
            }
        }
        out
    }

    /// Machine output, for a caller that has to act on the result.
    pub fn to_json(&self) -> String {
        let items = |list: &[Finding]| -> Vec<serde_json::Value> {
            list.iter()
                .map(|f| {
                    serde_json::json!({
                        "kind": f.kind,
                        "file": f.file,
                        "job": f.job,
                        "message": f.message,
                    })
                })
                .collect()
        };

        serde_json::to_string_pretty(&serde_json::json!({
            "valid": self.is_valid(),
            "workflow": self.workflow_name,
            "jobs": self.jobs,
            "order": self.order,
            "findings": items(&self.findings),
            "notes": items(&self.notes),
        }))
        .unwrap_or_else(|e| format!("{{\"valid\":false,\"error\":\"{e}\"}}"))
    }
}

fn render_item(finding: &Finding) -> String {
    let where_ = match (&finding.file, &finding.job) {
        (Some(file), _) => format!(" [{file}]"),
        (None, Some(job)) => format!(" [{job}]"),
        (None, None) => String::new(),
    };
    // The prechecks return multi-line messages; indent continuations so one
    // finding still reads as one item.
    let message = finding.message.trim_end().replace('\n', "\n    ");
    format!("  {}{}: {message}\n", finding.kind, where_)
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

    // 1b. The declared format version. Advisory within a major, since unknown
    //     keys parse harmlessly; a newer major may change meaning, so it fails.
    check_schema_version(&metadata, &mut report);
    check_env_passthrough(&metadata, &mut report.findings);

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
        // 4b. Ports: every input pattern can be fed by a direct dependency.
        check_input_ports(&report.jobs, &metas, &metadata, &mut report.findings);
    }

    // 5. Declared defaults against their types: a default is the value a run
    //    uses whenever no file names the param.
    check_default_types(
        &metadata.params,
        ".chiral/workflow.toml",
        None,
        &mut report.findings,
    );
    for job in &parsed {
        check_default_types(
            &metas[&job.name].params,
            &format!("{}/.chiral/job.toml", job.name),
            Some(&job.name),
            &mut report.findings,
        );
    }
    check_param_fallbacks(&parsed, &metas, &metadata, &mut report.notes);

    //    Parameter files, where they exist, against their definitions. This is
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
    if let Err(e) = crate::precheck::check_run_scripts(&parsed) {
        report.findings.push(Finding::new("script", e));
    }
    if let Err(e) = crate::precheck::check_input_files_folder(&workflow_path, &parsed, &metadata) {
        report
            .findings
            .push(Finding::new("inputs", e).at("input_files/"));
    }

    report
}

fn check_schema_version(metadata: &job_config::workflow::WorkflowMeta, report: &mut Report) {
    use job_config::workflow::{SCHEMA_VERSION, SchemaCompat};

    let major = SCHEMA_VERSION.split('.').next().unwrap_or(SCHEMA_VERSION);
    let at = ".chiral/workflow.toml";
    match metadata.schema_compat() {
        SchemaCompat::Supported => {}
        SchemaCompat::NewerMinor(v) => report.notes.push(
            Finding::new(
                "workflow",
                format!(
                    "workflow targets format {v}; this silva speaks {SCHEMA_VERSION}, \
                     so features newer than {SCHEMA_VERSION} are ignored."
                ),
            )
            .at(at),
        ),
        SchemaCompat::UnsupportedMajor(v) => report.findings.push(
            Finding::new(
                "workflow",
                format!("workflow requires format {v}; this silva speaks {major}.x — upgrade silva."),
            )
            .at(at),
        ),
        SchemaCompat::Malformed(v) => report.findings.push(
            Finding::new(
                "workflow",
                format!(
                    "schema_version \"{v}\" is not a format version; write MAJOR.MINOR, e.g. \"{SCHEMA_VERSION}\"."
                ),
            )
            .at(at),
        ),
    }
}

/// `env_passthrough` entries are forwarded as environment variable names, so a
/// name the shell cannot address is a mistake rather than a value.
fn check_env_passthrough(
    metadata: &job_config::workflow::WorkflowMeta,
    findings: &mut Vec<Finding>,
) {
    for name in metadata.env_passthrough.iter().flatten() {
        let mut chars = name.chars();
        let valid = chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_');
        if !valid {
            findings.push(
                Finding::new(
                    "workflow",
                    format!("env_passthrough entry '{name}' is not an environment variable name."),
                )
                .at(".chiral/workflow.toml"),
            );
        }
    }
}

/// One finding per declared default that does not match its own `type`.
fn check_default_types(
    params: &HashMap<String, job_config::job::ParamDefinition>,
    file: &str,
    job: Option<&str>,
    findings: &mut Vec<Finding>,
) {
    let mut names: Vec<&String> = params.keys().collect();
    names.sort();
    for name in names {
        if let Err(e) = params[name].validate(&params[name].default) {
            let finding = Finding::new("params", format!("Default of '{name}': {e}")).at(file);
            findings.push(match job {
                Some(job) => finding.in_job(job),
                None => finding,
            });
        }
    }
}

/// Notes `${PARAM_X:-…}` and `${PARAM_X-…}` where `X` is declared for the job:
/// a run always sets it, so the fallback is a second default that can drift.
/// A declared `""` is exempt, since `:-` still fires on an empty value.
fn check_param_fallbacks(
    jobs: &[JobFolder],
    metas: &HashMap<String, job_config::job::JobMeta>,
    metadata: &job_config::workflow::WorkflowMeta,
    notes: &mut Vec<Finding>,
) {
    for job in jobs {
        let meta = &metas[&job.name];
        // The job's own declaration shadows the workflow's, as at run time.
        let mut declared: HashMap<String, &toml::Value> = HashMap::new();
        for (name, def) in metadata.params.iter().chain(&meta.params) {
            declared.insert(name.to_uppercase(), &def.default);
        }
        for script in [&meta.scripts.pre, &meta.scripts.run, &meta.scripts.post] {
            let Ok(content) = std::fs::read_to_string(job.path.join(script)) else {
                continue;
            };
            let mut redundant: Vec<String> = content
                .lines()
                .filter(|line| !line.trim_start().starts_with('#'))
                .flat_map(param_fallbacks)
                .filter(|name| declared.get(name).is_some_and(|d| d.as_str() != Some("")))
                .map(|name| format!("PARAM_{name}"))
                .collect();
            redundant.sort();
            redundant.dedup();
            if !redundant.is_empty() {
                notes.push(
                    Finding::new(
                        "script",
                        format!(
                            "{}: declared, so a run always sets it and this fallback is a second \
                             default that can drift from the declared one.",
                            redundant.join(", ")
                        ),
                    )
                    .at(format!("{}/{script}", job.name))
                    .in_job(&job.name),
                );
            }
        }
    }
}

/// The `X` of every `${PARAM_X:-` or `${PARAM_X-` on a line.
fn param_fallbacks(line: &str) -> Vec<String> {
    line.split("${PARAM_")
        .skip(1)
        .filter_map(|rest| {
            let end = rest
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len());
            let after = &rest[end..];
            (end > 0 && (after.starts_with(":-") || after.starts_with('-')))
                .then(|| rest[..end].to_string())
        })
        .collect()
}

/// Every `inputs` pattern of a job must be able to match something one of its
/// direct dependencies declares in `outputs`; otherwise the job starts without
/// the file, silently.
///
/// Only provable misses are reported. A job is skipped when any dependency
/// declares no `outputs`, since its script may write to `outputs/` directly.
fn check_input_ports(
    job_names: &[String],
    metas: &HashMap<String, job_config::job::JobMeta>,
    metadata: &job_config::workflow::WorkflowMeta,
    findings: &mut Vec<Finding>,
) {
    for job in job_names {
        let Some(meta) = metas.get(job) else { continue };
        let deps = metadata.get_job_dependencies(job);
        if meta.inputs.is_empty() || deps.is_empty() {
            continue;
        }
        let Some(dep_metas) = deps
            .iter()
            .map(|d| metas.get(d).filter(|m| !m.outputs.is_empty()))
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        // Collection flattens paths, so a dependent sees only the last component.
        let outputs: Vec<&str> = dep_metas
            .iter()
            .flat_map(|m| &m.outputs)
            .filter_map(|o| o.trim_end_matches('/').rsplit('/').next())
            .filter(|o| !o.is_empty())
            .collect();

        for input in &meta.inputs {
            let message = if input.contains('/') {
                let name = input
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or(input);
                format!(
                    "input '{input}' contains a path, but inputs match file names only, \
                     so it can never match; write '{name}'."
                )
            } else if !outputs.iter().any(|o| may_overlap(input, o)) {
                format!(
                    "input '{input}' matches none of the outputs declared by {}: {}.",
                    deps.join(", "),
                    outputs.join(", ")
                )
            } else {
                continue;
            };
            findings.push(
                Finding::new("ports", message)
                    .at(format!("{job}/.chiral/job.toml"))
                    .in_job(job),
            );
        }
    }
}

/// Whether some file name could match both patterns. Exact for a literal
/// against a glob; for two globs, `false` only when their literal prefixes or
/// suffixes conflict, which every common match would have to carry.
fn may_overlap(a: &str, b: &str) -> bool {
    const META: &[char] = &['*', '?', '['];
    let matches = |glob: &str, name: &str| {
        globset::Glob::new(glob).map_or(true, |g| g.compile_matcher().is_match(name))
    };
    let prefix = |p: &str| -> String { p.split(META).next().unwrap_or("").to_string() };
    let suffix = |p: &str| -> String { p.rsplit(['*', '?', ']']).next().unwrap_or("").to_string() };

    expand_braces(a).iter().any(|a| {
        expand_braces(b)
            .iter()
            .any(|b| match (a.contains(META), b.contains(META)) {
                (false, _) => matches(b, a),
                (_, false) => matches(a, b),
                _ => {
                    let (pa, pb, sa, sb) = (prefix(a), prefix(b), suffix(a), suffix(b));
                    (pa.starts_with(&pb) || pb.starts_with(&pa))
                        && (sa.ends_with(&sb) || sb.ends_with(&sa))
                }
            })
    })
}

/// `x.{pdb,cif}` -> `x.pdb`, `x.cif`, innermost group first, so nesting works.
fn expand_braces(pattern: &str) -> Vec<String> {
    let Some(close) = pattern.find('}') else {
        return vec![pattern.to_string()];
    };
    let Some(open) = pattern[..close].rfind('{') else {
        return vec![pattern.to_string()];
    };
    pattern[open + 1..close]
        .split(',')
        .flat_map(|alt| {
            expand_braces(&format!(
                "{}{alt}{}",
                &pattern[..open],
                &pattern[close + 1..]
            ))
        })
        .collect()
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

    #[test]
    fn a_future_major_schema_version_fails_and_says_to_upgrade() {
        let dir = workflow("schema_version = \"2.0\"\n");
        let report = validate_workflow(dir.path());

        assert!(!report.is_valid());
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].kind, "workflow");
        assert!(report.findings[0].message.contains("upgrade silva"));
    }

    #[test]
    fn a_future_minor_schema_version_passes_with_a_note() {
        let dir = workflow("schema_version = \"1.1\"\n");
        let report = validate_workflow(dir.path());

        assert!(report.is_valid(), "{:?}", report.findings);
        assert_eq!(report.notes.len(), 1);
        assert!(report.render().contains("format 1.1"));
        assert!(report.to_json().contains("\"notes\": ["));
    }

    #[test]
    fn a_malformed_schema_version_is_reported() {
        let dir = workflow("schema_version = \"v1\"\n");
        let report = validate_workflow(dir.path());

        assert!(!report.is_valid());
        assert!(report.findings[0].message.contains("MAJOR.MINOR"));
    }

    #[test]
    fn a_known_or_absent_schema_version_adds_nothing() {
        for header in ["", "schema_version = \"1.0\"\n"] {
            let report = validate_workflow(workflow(header).path());
            assert!(report.is_valid(), "{:?}", report.findings);
            assert!(report.notes.is_empty());
        }
    }

    /// Gives `job` an `inputs` line, keeping the rest of the helper's job.toml.
    fn set_inputs(dir: &TempDir, job: &str, inputs: &str) {
        let path = dir.path().join(job).join(".chiral/job.toml");
        let toml = fs::read_to_string(&path).unwrap();
        fs::write(
            &path,
            toml.replace("outputs =", &format!("inputs = {inputs}\noutputs =")),
        )
        .unwrap();
    }

    fn ports(report: &Report) -> Vec<&Finding> {
        report
            .findings
            .iter()
            .filter(|f| f.kind == "ports")
            .collect()
    }

    #[test]
    fn an_input_an_upstream_output_can_satisfy_passes() {
        let dir = workflow("[dependencies]\n02-second = [\"01-first\"]\n");
        set_inputs(&dir, "02-second", "[\"result.txt\", \"*.{txt,csv}\"]");

        let report = validate_workflow(dir.path());
        assert!(report.is_valid(), "{:?}", report.findings);
    }

    #[test]
    fn an_input_no_upstream_output_can_satisfy_is_reported() {
        let dir = workflow("[dependencies]\n02-second = [\"01-first\"]\n");
        set_inputs(&dir, "02-second", "[\"*.txt\", \"*.pdb\"]");

        let report = validate_workflow(dir.path());
        let ports = ports(&report);
        assert_eq!(ports.len(), 1, "{:?}", report.findings);
        assert!(ports[0].message.contains("'*.pdb'"));
        assert_eq!(ports[0].job.as_deref(), Some("02-second"));
        assert_eq!(ports[0].file.as_deref(), Some("02-second/.chiral/job.toml"));
    }

    #[test]
    fn an_input_with_a_path_can_never_match_and_says_what_to_write() {
        let dir = workflow("[dependencies]\n02-second = [\"01-first\"]\n");
        set_inputs(&dir, "02-second", "[\"results/a.txt\"]");

        let report = validate_workflow(dir.path());
        let ports = ports(&report);
        assert_eq!(ports.len(), 1, "{:?}", report.findings);
        assert!(ports[0].message.contains("write 'a.txt'"));
    }

    #[test]
    fn a_dependency_without_declared_outputs_is_not_judged() {
        // It may write into outputs/ directly, which nothing here can see.
        let dir = workflow("[dependencies]\n02-second = [\"01-first\"]\n");
        let first = dir.path().join("01-first/.chiral/job.toml");
        let toml = fs::read_to_string(&first).unwrap();
        fs::write(&first, toml.replace("outputs = [\"*.txt\"]\n", "")).unwrap();
        set_inputs(&dir, "02-second", "[\"*.pdb\"]");

        let report = validate_workflow(dir.path());
        assert!(report.is_valid(), "{:?}", report.findings);
    }

    #[test]
    fn a_missing_run_script_is_reported() {
        let dir = workflow("[dependencies]\n02-second = [\"01-first\"]\n");
        fs::remove_file(dir.path().join("02-second/run.sh")).unwrap();

        let report = validate_workflow(dir.path());
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.kind == "script" && f.message.contains("[02-second] run.sh")),
            "{:?}",
            report.findings
        );
    }

    #[test]
    fn an_env_passthrough_entry_that_is_not_a_variable_name_is_reported() {
        let dir = workflow("env_passthrough = [\"HF_TOKEN\", \"NGC-API-KEY\"]\n");

        let report = validate_workflow(dir.path());
        assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
        assert!(report.findings[0].message.contains("NGC-API-KEY"));
    }

    #[test]
    fn globs_overlap_unless_their_literal_ends_conflict() {
        assert!(may_overlap("*.pdb", "design_*.pdb"));
        assert!(may_overlap("design_*_rank001.pdb", "design_*.pdb"));
        assert!(may_overlap("ligand_library", "ligand_*"));
        assert!(may_overlap("*.{fasta,fa}", "seq.fa"));
        assert!(!may_overlap("*.pdb", "*.sdf"));
        assert!(!may_overlap("a_*", "b_*"));
        assert!(!may_overlap("model.pkl", "*.txt"));
    }

    #[test]
    fn a_default_that_does_not_match_its_type_is_reported() {
        let dir = workflow("[params.top_n]\ntype = \"integer\"\ndefault = \"20\"\nhint = \"h\"\n");
        let report = validate_workflow(dir.path());
        let params: Vec<_> = report
            .findings
            .iter()
            .filter(|f| f.kind == "params")
            .collect();
        assert_eq!(params.len(), 1, "{:?}", report.findings);
        assert_eq!(params[0].file.as_deref(), Some(".chiral/workflow.toml"));
        assert!(
            params[0].message.contains("'top_n'"),
            "{}",
            params[0].message
        );
        // The fixture's job defaults (float 1.0) are well typed.
        assert!(params.iter().all(|f| f.job.is_none()));
    }

    #[test]
    fn a_fallback_on_a_declared_param_is_a_note_not_a_finding() {
        let dir = workflow("");
        fs::write(
            dir.path().join("01-first/run.sh"),
            "t=${PARAM_THRESHOLD:-0.5}\nu=${PARAM_THRESHOLD-0.5} ${PARAM_UNDECLARED:-x}\n# ${PARAM_THRESHOLD:-1}\n",
        )
        .unwrap();
        let report = validate_workflow(dir.path());
        assert!(report.is_valid(), "{:?}", report.findings);
        assert_eq!(report.notes.len(), 1, "{:?}", report.notes);
        let note = &report.notes[0];
        assert_eq!(note.kind, "script");
        assert_eq!(note.file.as_deref(), Some("01-first/run.sh"));
        assert!(
            note.message.starts_with("PARAM_THRESHOLD:"),
            "{}",
            note.message
        );
        assert!(!note.message.contains("UNDECLARED"));
    }

    #[test]
    fn a_fallback_on_an_empty_default_is_not_noted() {
        let dir = workflow("[params.hotspots]\ntype = \"string\"\ndefault = \"\"\nhint = \"h\"\n");
        fs::write(
            dir.path().join("01-first/run.sh"),
            "h=${PARAM_HOTSPOTS:-A10}\n",
        )
        .unwrap();
        let report = validate_workflow(dir.path());
        assert!(report.notes.is_empty(), "{:?}", report.notes);
    }
}
