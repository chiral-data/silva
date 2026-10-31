//! A workflow folder lowered to what any target language needs, with every
//! silva rule already applied: job order, existing scripts, resolved param
//! values, brace-free name patterns. Emitters read this, never the TOML.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use job_config::job::GpuMode;
use job_config::params::with_defaults;
use job_config::workflow::WorkflowMeta;

use crate::components::docker::executor::env_value;
use crate::components::workflow::{JobScanner, WorkflowFolder, apps_to_build};
use crate::headless::topological_sort_jobs;
use crate::precheck;

/// The workflow, in run order.
#[derive(Debug)]
pub struct ExportWorkflow {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub jobs: Vec<ExportJob>,
    /// Params declared in `workflow.toml`, defaulting to their resolved value.
    pub params: Vec<ExportParam>,
    /// What the target can't express; the export still goes ahead.
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct ExportJob {
    pub name: String,
    pub description: String,
    pub folder: PathBuf,
    pub image: String,
    /// The scripts silva would run, in order: `run`, plus `pre`/`post` when present.
    pub scripts: Vec<String>,
    /// `inputs` patterns with braces expanded; empty means "everything".
    pub inputs: Vec<String>,
    /// `outputs` patterns, shell-evaluated in the job folder as silva does.
    pub outputs: Vec<String>,
    /// Direct dependencies, in the order `[dependencies]` lists them.
    pub dependencies: Vec<String>,
    /// Dependency-free: receives `input_files/`.
    pub takes_input_files: bool,
    /// Params the job declares itself; they shadow the workflow's.
    pub params: Vec<ExportParam>,
    /// Every `PARAM_*` variable the job runs with, by name, sorted.
    pub env: Vec<(String, EnvSource)>,
    pub timeout_secs: Option<u64>,
    pub cpus: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub gpu_required: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExportParam {
    pub name: String,
    pub hint: String,
    /// The value as its environment variable carries it.
    pub default: String,
    /// Allowed values of an `enum` param.
    pub choices: Option<Vec<String>>,
}

/// Where a `PARAM_*` variable's value comes from.
#[derive(Debug, Clone, PartialEq)]
pub enum EnvSource {
    /// A param declared in `workflow.toml`.
    Workflow(String),
    /// A param the job declares itself.
    Job(String),
    /// An undeclared key from `global_params.json` or `params.json`, passed through.
    Constant(String),
}

/// Lowers the workflow at `path`, refusing what silva itself would refuse to run.
pub fn lower(path: &Path) -> Result<ExportWorkflow, String> {
    let folder_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let folder = WorkflowFolder::new(folder_name.clone(), path.to_path_buf(), None);
    let meta = folder
        .load_workflow_metadata()
        .map_err(|e| format!("Cannot read .chiral/workflow.toml: {e}"))?
        .unwrap_or_else(|| WorkflowMeta::new(folder_name, String::new()));
    precheck::check_schema_version(&meta)?;

    let jobs = JobScanner::scan_jobs(path).map_err(|e| format!("Cannot scan jobs: {e}"))?;
    if jobs.is_empty() {
        return Err("No jobs found: no folder holds a .chiral/job.toml.".to_string());
    }
    let sorted = topological_sort_jobs(&jobs, &meta)?;
    precheck::check_install_commands(&sorted)?;
    precheck::check_cross_node_references(&sorted)?;
    precheck::check_run_scripts(&sorted)?;
    precheck::check_input_files_folder(path, &sorted, &meta)?;

    let global_values = folder
        .load_workflow_params()
        .map_err(|e| format!("Cannot read global_params.json: {e}"))?
        .unwrap_or_default();
    let workflow_values = with_defaults(&meta.params, &global_values);
    let params = declared(&meta.params, &workflow_values);

    let mut warnings = Vec::new();
    for name in meta.env_passthrough.iter().flatten() {
        warnings.push(format!(
            "env_passthrough {name}: CWL can't read the host environment; \
             run cwltool with --preserve-environment {name}."
        ));
    }
    let local_apps: HashMap<String, PathBuf> = apps_to_build(path, &sorted)
        .into_iter()
        .map(|app| (app.image, app.dockerfile))
        .collect();

    let mut export_jobs = Vec::new();
    for job in &sorted {
        let config = job
            .load_meta()
            .map_err(|e| format!("Cannot read {}/.chiral/job.toml: {e}", job.name))?;
        let image = &config.container.image;
        if image.ends_with(".tar") || image.ends_with(".sif") {
            return Err(format!(
                "{}: image \"{image}\" is an image file; export needs an image name \
                 a container engine can pull.",
                job.name
            ));
        }
        if let Some(dockerfile) = local_apps.get(image) {
            let context = dockerfile.parent().unwrap_or(path);
            warnings.push(format!(
                "{}: image {image} is built from apps/, not pulled; build it before running: \
                 docker build -t {image} {}",
                job.name,
                context.display()
            ));
        }
        if let Some(retry) = config.retry {
            warnings.push(format!(
                "{}: retry = {retry} has no CWL equivalent and is dropped; \
                 use your runner's retry option instead.",
                job.name
            ));
        }
        let gpu = config.resources.as_ref().and_then(|r| r.gpu);
        if gpu == Some(GpuMode::Auto) {
            warnings.push(format!(
                "{}: gpu = \"auto\" has no CWL equivalent; the job runs without a GPU \
                 unless the runner adds one.",
                job.name
            ));
        }

        let job_values = job
            .load_params()
            .map_err(|e| format!("Cannot read {}/params.json: {e}", job.name))?
            .unwrap_or_default();
        let own_values = with_defaults(&config.params, &job_values);

        // The same layering as `resolve_params`: the job's own layer wins.
        let mut env: HashMap<String, EnvSource> = HashMap::new();
        for (name, value) in &workflow_values {
            let source = if meta.params.contains_key(name) {
                EnvSource::Workflow(name.clone())
            } else {
                EnvSource::Constant(env_value(value))
            };
            env.insert(name.clone(), source);
        }
        for (name, value) in &own_values {
            let source = if config.params.contains_key(name) {
                EnvSource::Job(name.clone())
            } else {
                EnvSource::Constant(env_value(value))
            };
            env.insert(name.clone(), source);
        }
        let mut env: Vec<(String, EnvSource)> = env.into_iter().collect();
        env.sort_by(|a, b| a.0.cmp(&b.0));

        let mut scripts = Vec::new();
        for (script, optional) in [
            (&config.scripts.pre, true),
            (&config.scripts.run, false),
            (&config.scripts.post, true),
        ] {
            if !optional || job.path.join(script).is_file() {
                scripts.push(script.clone());
            }
        }

        let dependencies = meta.get_job_dependencies(&job.name).to_vec();
        export_jobs.push(ExportJob {
            name: job.name.clone(),
            description: config.description.clone(),
            folder: job.path.clone(),
            image: image.clone(),
            scripts,
            inputs: config
                .inputs
                .iter()
                .flat_map(|p| expand_braces(p))
                .collect(),
            outputs: config.outputs.clone(),
            takes_input_files: dependencies.is_empty(),
            dependencies,
            params: declared(&config.params, &own_values),
            env,
            timeout_secs: config.timeout.as_ref().map(|t| t.duration().as_secs()),
            cpus: config.resources.as_ref().and_then(|r| r.cpus),
            memory_bytes: config
                .resources
                .as_ref()
                .and_then(|r| r.memory.as_ref())
                .map(|m| m.bytes()),
            gpu_required: gpu == Some(GpuMode::Required),
        });
    }

    Ok(ExportWorkflow {
        name: meta.name.clone(),
        description: meta.description.clone(),
        path: path.to_path_buf(),
        jobs: export_jobs,
        params,
        warnings,
    })
}

/// Declared params, sorted by name, each defaulting to its resolved value.
fn declared(
    defs: &HashMap<String, job_config::job::ParamDefinition>,
    values: &job_config::params::JobParams,
) -> Vec<ExportParam> {
    let mut params: Vec<ExportParam> = defs
        .iter()
        .map(|(name, def)| ExportParam {
            name: name.clone(),
            hint: def.hint.clone(),
            default: values.get(name).map(env_value).unwrap_or_default(),
            choices: (def.param_type == job_config::job::ParamType::Enum)
                .then(|| def.enum_values.clone())
                .flatten(),
        })
        .collect();
    params.sort_by(|a, b| a.name.cmp(&b.name));
    params
}

/// `*.{fasta,fa}` → `*.fasta`, `*.fa`, as `globset` reads braces; nested braces too.
pub(crate) fn expand_braces(pattern: &str) -> Vec<String> {
    let Some(open) = pattern.find('{') else {
        return vec![pattern.to_string()];
    };
    let mut depth = 0;
    let mut close = None;
    let mut commas = Vec::new();
    for (i, c) in pattern[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open + i);
                    break;
                }
            }
            ',' if depth == 1 => commas.push(open + i),
            _ => {}
        }
    }
    let Some(close) = close else {
        return vec![pattern.to_string()];
    };
    let (head, tail) = (&pattern[..open], &pattern[close + 1..]);
    let mut bounds = vec![open];
    bounds.extend(&commas);
    bounds.push(close);
    bounds
        .windows(2)
        .flat_map(|w| {
            let alternative = &pattern[w[0] + 1..w[1]];
            expand_braces(&format!("{head}{alternative}{tail}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(root: &Path, rel: &str, content: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn workflow(workflow_toml: &str, first_job: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            ".chiral/workflow.toml",
            &format!("name = \"w\"\ndescription = \"d\"\n{workflow_toml}"),
        );
        write(
            root,
            "01-first/.chiral/job.toml",
            &format!(
                "name = \"First\"\ndescription = \"d\"\n{first_job}\n[container]\nimage = \"alpine\"\n"
            ),
        );
        write(root, "01-first/run.sh", "echo hi > a.txt\n");
        write(
            root,
            "02-second/.chiral/job.toml",
            "name = \"Second\"\ndescription = \"d\"\ninputs = [\"*.{txt,csv}\"]\n[container]\nimage = \"alpine\"\n",
        );
        write(root, "02-second/run.sh", "cat inputs/a.txt\n");
        fs::create_dir_all(root.join("input_files")).unwrap();
        dir
    }

    #[test]
    fn braces_expand_like_globset() {
        assert_eq!(
            expand_braces("*.{fasta,fa,faa}"),
            vec!["*.fasta", "*.fa", "*.faa"]
        );
        assert_eq!(expand_braces("{a,b{1,2}}.x"), vec!["a.x", "b1.x", "b2.x"]);
        assert_eq!(expand_braces("plain.txt"), vec!["plain.txt"]);
        assert_eq!(expand_braces("open{.txt"), vec!["open{.txt"]);
    }

    #[test]
    fn lowering_orders_jobs_and_resolves_the_param_layers() {
        let dir = workflow(
            "[dependencies]\n02-second = [\"01-first\"]\n\
             [params.shared]\ntype = \"integer\"\ndefault = 1\nhint = \"h\"\n\
             [params.mode]\ntype = \"string\"\ndefault = \"slow\"\nhint = \"h\"\n",
            "pre = \"pre_run.sh\"\n[params.mode]\ntype = \"enum\"\ndefault = \"fast\"\nhint = \"h\"\nenum_values = [\"fast\", \"slow\"]\n",
        );
        write(
            dir.path(),
            "global_params.json",
            r#"{"shared": 7, "extra": true}"#,
        );
        let wf = lower(dir.path()).unwrap();

        assert_eq!(
            wf.jobs.iter().map(|j| j.name.as_str()).collect::<Vec<_>>(),
            ["01-first", "02-second"]
        );
        assert_eq!(
            wf.params
                .iter()
                .find(|p| p.name == "shared")
                .unwrap()
                .default,
            "7"
        );

        let first = &wf.jobs[0];
        assert!(first.takes_input_files);
        // No pre_run.sh on disk, so only `run` is kept.
        assert_eq!(first.scripts, vec!["./run.sh"]);
        assert_eq!(
            first.params[0].choices.as_deref(),
            Some(&["fast".to_string(), "slow".to_string()][..])
        );
        assert_eq!(
            first.env,
            vec![
                ("extra".to_string(), EnvSource::Constant("true".to_string())),
                ("mode".to_string(), EnvSource::Job("mode".to_string())),
                (
                    "shared".to_string(),
                    EnvSource::Workflow("shared".to_string())
                ),
            ]
        );

        let second = &wf.jobs[1];
        assert_eq!(second.dependencies, vec!["01-first"]);
        assert!(!second.takes_input_files);
        assert_eq!(second.inputs, vec!["*.txt", "*.csv"]);
        assert!(wf.warnings.is_empty(), "{:?}", wf.warnings);
    }

    #[test]
    fn an_image_file_is_refused_and_retry_is_warned_about() {
        let dir = workflow("", "retry = 2\n");
        let wf = lower(dir.path()).unwrap();
        assert!(
            wf.warnings
                .iter()
                .any(|w| w.starts_with("01-first: retry = 2 has no CWL equivalent"))
        );

        let toml = dir.path().join("02-second/.chiral/job.toml");
        let text = fs::read_to_string(&toml)
            .unwrap()
            .replace("\"alpine\"", "\"image.sif\"");
        fs::write(&toml, text).unwrap();
        let err = lower(dir.path()).unwrap_err();
        assert!(
            err.starts_with("02-second: image \"image.sif\" is an image file"),
            "{err}"
        );
    }

    #[test]
    fn a_newer_major_format_is_refused() {
        let dir = workflow("schema_version = \"2.0\"\n", "");
        assert!(lower(dir.path()).unwrap_err().contains("upgrade silva"));
    }
}
