//! WDL 1.2 from an [`ExportWorkflow`]: one `workflow.wdl` holding a task per
//! job and the workflow calling them, plus `inputs.json` naming the folders.
//!
//! As for CWL, WDL carries the graph, image, environment and limits, and the
//! shared driver does what silva does around a job. WDL 1.2 is required for
//! `Directory`: job folders and dependency outputs pass as whole folders.

use std::fmt::Write;
use std::path::Path;

use serde_json::{Map, Value};

use super::cwl::local_image_warning;
use super::driver::{INPUT_FILES, JOB_DIR, driver, id};
use super::ir::{Caveat, EnvSource, ExportJob, ExportParam, ExportWorkflow};

/// Words the WDL grammar reserves; an identifier equal to one is renamed.
const KEYWORDS: &[&str] = &[
    "alias",
    "Array",
    "as",
    "Boolean",
    "call",
    "command",
    "Directory",
    "else",
    "env",
    "false",
    "File",
    "Float",
    "hints",
    "if",
    "import",
    "in",
    "input",
    "Int",
    "left",
    "Map",
    "meta",
    "None",
    "null",
    "object",
    "Object",
    "output",
    "Pair",
    "parameter_meta",
    "requirements",
    "right",
    "runtime",
    "scatter",
    "String",
    "struct",
    "task",
    "then",
    "true",
    "version",
    "workflow",
];

/// The files to write into the output folder, as (file name, content).
pub fn emit(wf: &ExportWorkflow) -> Result<Vec<(String, String)>, String> {
    let names = task_names(wf)?;
    let workflow_name = identifier(&wf.name, "workflow");

    let mut doc = String::from("version 1.2\n");
    for (job, task) in wf.jobs.iter().zip(&names) {
        doc.push('\n');
        doc.push_str(&task_block(job, task)?);
    }
    doc.push('\n');
    doc.push_str(&workflow_block(wf, &workflow_name, &names));

    let mut inputs = Map::new();
    if wf.jobs.iter().any(|j| j.takes_input_files) {
        inputs.insert(
            format!("{workflow_name}.input_files"),
            Value::String(absolute(&wf.path.join("input_files"))?),
        );
    }
    for (job, task) in wf.jobs.iter().zip(&names) {
        inputs.insert(
            format!("{workflow_name}.{task}_dir"),
            Value::String(absolute(&job.folder)?),
        );
    }
    let mut inputs_json =
        serde_json::to_string_pretty(&Value::Object(inputs)).expect("inputs always serialize");
    inputs_json.push('\n');

    Ok(vec![
        ("workflow.wdl".to_string(), doc),
        ("inputs.json".to_string(), inputs_json),
    ])
}

/// The warnings to show for what WDL can't carry, in the order found.
pub fn warnings(wf: &ExportWorkflow) -> Vec<String> {
    let mut out: Vec<String> = wf
        .caveats
        .iter()
        .filter_map(|caveat| match caveat {
            Caveat::EnvPassthrough(name) => Some(format!(
                "env_passthrough {name}: WDL can't read the host environment; \
                 run miniwdl with --env {name}."
            )),
            Caveat::LocalImage {
                job,
                image,
                context,
            } => Some(local_image_warning(job, image, context)),
            // `max_retries` carries it.
            Caveat::Retry { .. } => None,
            Caveat::GpuAuto { job } => Some(format!(
                "{job}: gpu = \"auto\" has no WDL equivalent; the job runs without a GPU \
                 unless the runner adds one."
            )),
        })
        .collect();
    for job in &wf.jobs {
        if let Some(secs) = job.timeout_secs {
            out.push(format!(
                "{}: a timeout of {secs}s has no WDL equivalent and is dropped; \
                 use your runner's time limit instead.",
                job.name
            ));
        }
    }
    out
}

/// `job_<name>` per job, refusing two jobs that would share one.
fn task_names(wf: &ExportWorkflow) -> Result<Vec<String>, String> {
    let mut names: Vec<String> = Vec::new();
    for job in &wf.jobs {
        let name = format!("job_{}", sanitize(&job.name));
        if let Some(other) = names.iter().position(|n| *n == name) {
            return Err(format!(
                "{} and {} both become the WDL task {name}; rename one of the job folders.",
                wf.jobs[other].name, job.name
            ));
        }
        names.push(name);
    }
    Ok(names)
}

fn sanitize(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// A WDL identifier from free text: starts with a letter, no keyword.
fn identifier(name: &str, fallback: &str) -> String {
    let name = sanitize(name);
    if name.starts_with(|c: char| c.is_ascii_alphabetic()) && !KEYWORDS.contains(&name.as_str()) {
        name
    } else {
        format!("{fallback}_{name}")
    }
}

/// `PARAM_<NAME>`, as silva sets it, which a WDL `env` input must also be called.
fn env_name(job: &ExportJob, name: &str) -> Result<String, String> {
    let var = format!("PARAM_{}", name.to_uppercase());
    if var.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        Ok(var)
    } else {
        Err(format!(
            "{}: param {name} becomes {var}, which isn't a WDL identifier; \
             rename the param.",
            job.name
        ))
    }
}

fn absolute(path: &Path) -> Result<String, String> {
    path.canonicalize()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Cannot resolve {}: {e}", path.display()))
}

/// A WDL string literal for an expression: also keeps `~{` and `${` literal,
/// which WDL would read as placeholders.
fn string(text: &str) -> String {
    meta_string(text)
        .replace("~{", "~{\"~\"}{")
        .replace("${", "~{\"$\"}{")
}

/// A string in `meta` / `parameter_meta`, where placeholders aren't read.
fn meta_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// In `command <<< >>>`, only `~{` and `>>>` are special; an empty
/// placeholder splits each so it stays literal.
fn command_text(text: &str) -> String {
    text.replace("~{", "~~{\"\"}{").replace(">>>", ">>~{\"\"}>")
}

fn param_meta(param: &ExportParam) -> Option<String> {
    let mut fields = Vec::new();
    if !param.hint.is_empty() {
        fields.push(format!("help: {}", meta_string(&param.hint)));
    }
    if let Some(choices) = &param.choices {
        let list: Vec<String> = choices.iter().map(|c| meta_string(c)).collect();
        fields.push(format!("choices: [{}]", list.join(", ")));
    }
    (!fields.is_empty()).then(|| format!("{{ {} }}", fields.join(", ")))
}

fn task_block(job: &ExportJob, task: &str) -> Result<String, String> {
    let mut inputs = vec!["    Directory job_dir".to_string()];
    if !job.dependencies.is_empty() {
        inputs.push("    Array[Directory] dependencies = []".to_string());
    }
    if job.takes_input_files {
        inputs.push("    Directory input_files".to_string());
    }
    let mut constants = Vec::new();
    for (name, source) in &job.env {
        let var = env_name(job, name)?;
        match source {
            // Overridable: the workflow passes its value in.
            EnvSource::Workflow(_) | EnvSource::Job(_) => {
                inputs.push(format!("    env String {var}"));
            }
            EnvSource::Constant(value) => {
                constants.push(format!("  env String {var} = {}", string(value)));
            }
        }
    }

    let mut prelude = String::from("set -eu\n");
    // Engine-made paths, so single quotes are enough.
    let _ = writeln!(prelude, "cp -r '~{{job_dir}}' {JOB_DIR}");
    if job.takes_input_files {
        let _ = writeln!(prelude, "cp -r '~{{input_files}}' {INPUT_FILES}");
    }
    if !job.dependencies.is_empty() {
        prelude.push_str("set -- ~{sep(\" \", squote(dependencies))}\n");
    }

    let mut requirements = vec![format!("    container: {}", string(&job.image))];
    if let Some(cpus) = job.cpus {
        requirements.push(format!("    cpu: {cpus}"));
    }
    if let Some(bytes) = job.memory_bytes {
        requirements.push(format!(
            "    memory: \"{} MiB\"",
            bytes.div_ceil(1024 * 1024)
        ));
    }
    if job.gpu_required {
        requirements.push("    gpu: true".to_string());
    }
    if let Some(retries) = job.retry {
        requirements.push(format!("    max_retries: {retries}"));
    }

    let mut s = String::new();
    let _ = writeln!(s, "task {task} {{");
    let _ = writeln!(s, "  meta {{");
    let _ = writeln!(s, "    silva_job: {}", meta_string(&job.name));
    if !job.description.is_empty() {
        let _ = writeln!(s, "    description: {}", meta_string(&job.description));
    }
    let _ = writeln!(s, "  }}");
    let _ = writeln!(s, "  input {{\n{}\n  }}", inputs.join("\n"));
    for constant in constants {
        let _ = writeln!(s, "{constant}");
    }
    let _ = writeln!(
        s,
        "  command <<<\n{prelude}{}  >>>",
        command_text(&driver(job))
    );
    let _ = writeln!(
        s,
        "  output {{\n    Directory outputs = {}\n  }}",
        string(&id(&job.name))
    );
    let _ = writeln!(s, "  requirements {{\n{}\n  }}", requirements.join("\n"));
    s.push_str("}\n");
    Ok(s)
}

fn workflow_block(wf: &ExportWorkflow, name: &str, tasks: &[String]) -> String {
    let mut inputs = Vec::new();
    let mut param_meta_lines = Vec::new();
    if wf.jobs.iter().any(|j| j.takes_input_files) {
        inputs.push("    Directory input_files".to_string());
    }
    for task in tasks {
        inputs.push(format!("    Directory {task}_dir"));
    }
    let mut declare = |input: String, param: &ExportParam| {
        if let Some(meta) = param_meta(param) {
            param_meta_lines.push(format!("    {input}: {meta}"));
        }
        inputs.push(format!("    String {input} = {}", string(&param.default)));
    };
    for param in &wf.params {
        declare(format!("param_{}", sanitize(&param.name)), param);
    }
    for (job, task) in wf.jobs.iter().zip(tasks) {
        for param in &job.params {
            declare(format!("{task}__{}", sanitize(&param.name)), param);
        }
    }

    let mut s = String::new();
    let _ = writeln!(s, "workflow {name} {{");
    let _ = writeln!(s, "  meta {{");
    let _ = writeln!(s, "    silva_workflow: {}", meta_string(&wf.name));
    if !wf.description.is_empty() {
        let _ = writeln!(s, "    description: {}", meta_string(&wf.description));
    }
    let _ = writeln!(s, "  }}");
    if !param_meta_lines.is_empty() {
        let _ = writeln!(
            s,
            "  parameter_meta {{\n{}\n  }}",
            param_meta_lines.join("\n")
        );
    }
    let _ = writeln!(s, "  input {{\n{}\n  }}", inputs.join("\n"));

    for (job, task) in wf.jobs.iter().zip(tasks) {
        let mut args = vec![format!("job_dir = {task}_dir")];
        if !job.dependencies.is_empty() {
            let deps: Vec<String> = job
                .dependencies
                .iter()
                .map(|d| format!("job_{}.outputs", sanitize(d)))
                .collect();
            args.push(format!("dependencies = [{}]", deps.join(", ")));
        }
        if job.takes_input_files {
            args.push("input_files = input_files".to_string());
        }
        for (param, source) in &job.env {
            let value = match source {
                EnvSource::Workflow(p) => format!("param_{}", sanitize(p)),
                EnvSource::Job(p) => format!("{task}__{}", sanitize(p)),
                EnvSource::Constant(_) => continue,
            };
            args.push(format!("PARAM_{} = {value}", param.to_uppercase()));
        }
        let _ = writeln!(
            s,
            "  call {task} {{ input:\n    {}\n  }}",
            args.join(",\n    ")
        );
    }

    let outputs: Vec<String> = tasks
        .iter()
        .map(|t| format!("    Directory out_{t} = {t}.outputs"))
        .collect();
    let _ = writeln!(s, "  output {{\n{}\n  }}", outputs.join("\n"));
    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn job(name: &str) -> ExportJob {
        ExportJob {
            name: name.to_string(),
            description: "d".to_string(),
            folder: PathBuf::from(name),
            image: "alpine".to_string(),
            scripts: vec!["./run.sh".to_string()],
            inputs: vec![],
            outputs: vec!["*.txt".to_string()],
            dependencies: vec![],
            takes_input_files: true,
            params: vec![],
            env: vec![],
            timeout_secs: None,
            cpus: None,
            memory_bytes: None,
            gpu_required: false,
            retry: None,
        }
    }

    #[test]
    fn no_command_carries_a_live_placeholder_or_an_early_end() {
        let mut j = job("02-x");
        j.takes_input_files = false;
        j.dependencies = vec!["01-a".to_string()];
        j.inputs = vec!["*.pdb".to_string()];
        // `outputs` reach bash as written, so they carry WDL's special text.
        j.outputs = vec!["~{x}>>>b.txt".to_string()];
        let block = task_block(&j, "job_02_x").unwrap();
        let start = block.find("command <<<").unwrap();
        let end = block.find("\n  >>>").unwrap();
        let command = &block[start + "command <<<".len()..end];
        // The prelude's own placeholders are the only live ones.
        let live: Vec<&str> = command
            .match_indices("~{")
            .map(|(i, _)| &command[i..])
            .collect();
        assert!(
            live.iter().all(|p| p.starts_with("~{job_dir")
                || p.starts_with("~{sep")
                || p.starts_with("~{\"\"}")),
            "{command}"
        );
        assert!(!command.contains(">>>"), "{command}");
        assert!(command.contains("~~{\"\"}{x}"), "{command}");
    }

    #[test]
    fn limits_retries_and_gpu_become_requirements() {
        let mut j = job("01-a");
        j.cpus = Some(0.5);
        j.memory_bytes = Some(3 * 1024 * 1024 * 1024 / 2);
        j.gpu_required = true;
        j.retry = Some(3);
        let block = task_block(&j, "job_01_a").unwrap();
        for line in [
            "container: \"alpine\"",
            "cpu: 0.5",
            "memory: \"1536 MiB\"",
            "gpu: true",
            "max_retries: 3",
        ] {
            assert!(block.contains(line), "{line} missing:\n{block}");
        }
    }

    #[test]
    fn params_are_env_inputs_and_a_bad_name_is_refused() {
        let mut j = job("01-a");
        j.env = vec![
            (
                "extra".to_string(),
                EnvSource::Constant("${x} ~{y}".to_string()),
            ),
            ("mode".to_string(), EnvSource::Job("mode".to_string())),
        ];
        let block = task_block(&j, "job_01_a").unwrap();
        assert!(block.contains("    env String PARAM_MODE\n"), "{block}");
        assert!(
            block.contains("  env String PARAM_EXTRA = \"~{\"$\"}{x} ~{\"~\"}{y}\""),
            "{block}"
        );

        j.env = vec![("my-mode".to_string(), EnvSource::Job("my-mode".to_string()))];
        let err = task_block(&j, "job_01_a").unwrap_err();
        assert!(
            err.contains("PARAM_MY-MODE, which isn't a WDL identifier"),
            "{err}"
        );
    }

    #[test]
    fn job_names_become_identifiers_and_a_collision_is_refused() {
        assert_eq!(identifier("two-node basic", "workflow"), "two_node_basic");
        assert_eq!(identifier("033 binder", "workflow"), "workflow_033_binder");
        assert_eq!(identifier("task", "workflow"), "workflow_task");

        let wf = ExportWorkflow {
            name: "w".to_string(),
            description: String::new(),
            path: PathBuf::from("."),
            jobs: vec![job("01-a"), job("01_a")],
            params: vec![],
            caveats: vec![],
        };
        let err = task_names(&wf).unwrap_err();
        assert!(err.contains("both become the WDL task job_01_a"), "{err}");
    }

    #[test]
    fn retry_needs_no_warning_but_a_timeout_does() {
        let mut j = job("01-a");
        j.timeout_secs = Some(1800);
        let wf = ExportWorkflow {
            name: "w".to_string(),
            description: String::new(),
            path: PathBuf::from("."),
            jobs: vec![j],
            params: vec![],
            caveats: vec![Caveat::Retry {
                job: "01-a".to_string(),
                count: 2,
            }],
        };
        assert_eq!(
            warnings(&wf),
            vec![
                "01-a: a timeout of 1800s has no WDL equivalent and is dropped; \
                 use your runner's time limit instead."
                    .to_string()
            ]
        );
    }
}
