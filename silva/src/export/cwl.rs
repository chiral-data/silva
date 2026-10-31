//! CWL v1.2 from an [`ExportWorkflow`]: one `CommandLineTool` per job and a
//! `Workflow` wiring them, written as JSON (valid YAML, so valid CWL).
//!
//! CWL carries the graph, image, environment and limits. What silva does
//! around a job — staging `inputs/`, running `pre`/`run`/`post`, collecting
//! `outputs/` — is a generated bash driver, because those rules work on
//! folders and name patterns that CWL's typed file wiring can't express.

use std::path::Path;

use serde_json::{Map, Value, json};

use super::driver::{INPUT_FILES, JOB_DIR, driver, id};
use super::ir::{Caveat, EnvSource, ExportJob, ExportParam, ExportWorkflow};

/// The driver's file name in a step's working folder.
const DRIVER: &str = ".silva_job.sh";

/// The files to write into the output folder, as (file name, content).
/// `out_dir` must exist: the job folders are referenced relative to it.
pub fn emit(wf: &ExportWorkflow, out_dir: &Path) -> Result<Vec<(String, String)>, String> {
    let mut files = Vec::new();
    for job in &wf.jobs {
        files.push((format!("{}.cwl", id(&job.name)), render(&tool(job))));
    }
    files.push(("workflow.cwl".to_string(), render(&workflow(wf, out_dir)?)));
    Ok(files)
}

fn render(doc: &Value) -> String {
    let mut text = serde_json::to_string_pretty(doc).expect("a CWL document always serializes");
    text.push('\n');
    text
}

fn param_input(param: &ExportParam) -> Value {
    let mut input = Map::new();
    input.insert(
        "type".into(),
        match &param.choices {
            Some(symbols) => json!({"type": "enum", "symbols": symbols}),
            // Every value reaches the job as an environment variable, which
            // CWL only accepts as a string.
            None => json!("string"),
        },
    );
    input.insert("default".into(), json!(param.default));
    if !param.hint.is_empty() {
        input.insert("doc".into(), json!(param.hint));
    }
    Value::Object(input)
}

fn tool(job: &ExportJob) -> Value {
    let mut env = Map::new();
    let mut inputs = Map::new();
    inputs.insert("job_dir".into(), json!({"type": "Directory"}));
    inputs.insert(
        "dependencies".into(),
        json!({"type": "Directory[]", "default": [], "inputBinding": {"position": 1}}),
    );
    if job.takes_input_files {
        inputs.insert("input_files".into(), json!({"type": "Directory"}));
    }
    for (name, source) in &job.env {
        let var = format!("PARAM_{}", name.to_uppercase());
        let value = match source {
            EnvSource::Workflow(_) | EnvSource::Job(_) => {
                let input = format!("param_{}", id(name));
                inputs.insert(input.clone(), json!({"type": "Any"}));
                format!("$(inputs.{input})")
            }
            EnvSource::Constant(value) => escape(value),
        };
        env.insert(var, json!(value));
    }

    let mut listing = vec![
        json!({"entry": "$(inputs.job_dir)", "entryname": JOB_DIR, "writable": true}),
        json!({"entryname": DRIVER, "entry": escape(&driver(job))}),
    ];
    if job.takes_input_files {
        listing.push(json!({"entry": "$(inputs.input_files)", "entryname": INPUT_FILES}));
    }

    let mut requirements = Map::new();
    requirements.insert("DockerRequirement".into(), json!({"dockerPull": job.image}));
    requirements.insert(
        "InitialWorkDirRequirement".into(),
        json!({"listing": listing}),
    );
    // silva's containers have the network; CWL's default is none.
    requirements.insert("NetworkAccess".into(), json!({"networkAccess": true}));
    if !env.is_empty() {
        requirements.insert("EnvVarRequirement".into(), json!({"envDef": env}));
    }
    if let Some(secs) = job.timeout_secs {
        requirements.insert("ToolTimeLimit".into(), json!({"timelimit": secs}));
    }
    if job.cpus.is_some() || job.memory_bytes.is_some() {
        let mut resources = Map::new();
        if let Some(cpus) = job.cpus {
            resources.insert("coresMin".into(), json!(cpus));
            resources.insert("coresMax".into(), json!(cpus));
        }
        if let Some(bytes) = job.memory_bytes {
            let mib = bytes.div_ceil(1024 * 1024);
            resources.insert("ramMin".into(), json!(mib));
            resources.insert("ramMax".into(), json!(mib));
        }
        requirements.insert("ResourceRequirement".into(), Value::Object(resources));
    }

    let mut doc = Map::new();
    doc.insert("cwlVersion".into(), json!("v1.2"));
    doc.insert("class".into(), json!("CommandLineTool"));
    doc.insert("label".into(), json!(job.name));
    if !job.description.is_empty() {
        doc.insert("doc".into(), json!(job.description));
    }
    if job.gpu_required {
        doc.insert(
            "$namespaces".into(),
            json!({"cwltool": "http://commonwl.org/cwltool#"}),
        );
        doc.insert(
            "hints".into(),
            json!({"cwltool:CUDARequirement": {
                "cudaVersionMin": "11.0",
                "cudaComputeCapability": "3.0",
                "cudaDeviceCountMin": 1
            }}),
        );
    }
    doc.insert("requirements".into(), Value::Object(requirements));
    doc.insert("inputs".into(), Value::Object(inputs));
    doc.insert("baseCommand".into(), json!(["bash", DRIVER]));
    doc.insert(
        "outputs".into(),
        json!({"outputs": {"type": "Directory", "outputBinding": {"glob": id(&job.name)}}}),
    );
    Value::Object(doc)
}

fn workflow(wf: &ExportWorkflow, out_dir: &Path) -> Result<Value, String> {
    let mut inputs = Map::new();
    if wf.jobs.iter().any(|j| j.takes_input_files) {
        inputs.insert(
            "input_files".into(),
            directory_input(&wf.path.join("input_files"), out_dir)?,
        );
    }
    for param in &wf.params {
        inputs.insert(format!("param_{}", id(&param.name)), param_input(param));
    }

    let mut outputs = Map::new();
    let mut steps = Map::new();
    for job in &wf.jobs {
        let job_id = id(&job.name);
        inputs.insert(
            format!("job_{job_id}"),
            directory_input(&job.folder, out_dir)?,
        );
        for param in &job.params {
            inputs.insert(format!("{job_id}__{}", id(&param.name)), param_input(param));
        }

        let mut step_in = Map::new();
        step_in.insert("job_dir".into(), json!(format!("job_{job_id}")));
        if !job.dependencies.is_empty() {
            let sources: Vec<String> = job
                .dependencies
                .iter()
                .map(|d| format!("{}/outputs", id(d)))
                .collect();
            step_in.insert(
                "dependencies".into(),
                json!({"source": sources, "linkMerge": "merge_flattened"}),
            );
        }
        if job.takes_input_files {
            step_in.insert("input_files".into(), json!("input_files"));
        }
        for (name, source) in &job.env {
            let wf_input = match source {
                EnvSource::Workflow(p) => format!("param_{}", id(p)),
                EnvSource::Job(p) => format!("{job_id}__{}", id(p)),
                EnvSource::Constant(_) => continue,
            };
            step_in.insert(format!("param_{}", id(name)), json!(wf_input));
        }
        steps.insert(
            job_id.clone(),
            json!({"run": format!("{job_id}.cwl"), "in": step_in, "out": ["outputs"]}),
        );
        outputs.insert(
            job_id.clone(),
            json!({"type": "Directory", "outputSource": format!("{job_id}/outputs")}),
        );
    }

    let mut doc = Map::new();
    doc.insert("cwlVersion".into(), json!("v1.2"));
    doc.insert("class".into(), json!("Workflow"));
    doc.insert("label".into(), json!(wf.name));
    if !wf.description.is_empty() {
        doc.insert("doc".into(), json!(wf.description));
    }
    doc.insert(
        "requirements".into(),
        json!({"MultipleInputFeatureRequirement": {}}),
    );
    doc.insert("inputs".into(), Value::Object(inputs));
    doc.insert("outputs".into(), Value::Object(outputs));
    doc.insert("steps".into(), Value::Object(steps));
    Ok(Value::Object(doc))
}

/// A Directory input defaulting to `target`, located relative to `out_dir`.
fn directory_input(target: &Path, out_dir: &Path) -> Result<Value, String> {
    let target = target
        .canonicalize()
        .map_err(|e| format!("Cannot resolve {}: {e}", target.display()))?;
    let location = relative(out_dir, &target);
    Ok(json!({"type": "Directory", "default": {"class": "Directory", "location": location}}))
}

/// `to` as a URI reference relative to the folder `from`; both canonical.
fn relative(from: &Path, to: &Path) -> String {
    let from: Vec<_> = from.components().collect();
    let to: Vec<_> = to.components().collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = vec!["..".to_string(); from.len() - common];
    parts.extend(
        to[common..]
            .iter()
            .map(|c| percent_encode(&c.as_os_str().to_string_lossy())),
    );
    parts.join("/")
}

fn percent_encode(segment: &str) -> String {
    segment
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The warnings to show for what CWL can't carry, in the order found.
pub fn warnings(caveats: &[Caveat]) -> Vec<String> {
    caveats
        .iter()
        .map(|caveat| match caveat {
            Caveat::EnvPassthrough(name) => format!(
                "env_passthrough {name}: CWL can't read the host environment; \
                 run cwltool with --preserve-environment {name}."
            ),
            Caveat::LocalImage {
                job,
                image,
                context,
            } => local_image_warning(job, image, context),
            Caveat::Retry { job, count } => format!(
                "{job}: retry = {count} has no CWL equivalent and is dropped; \
                 use your runner's retry option instead."
            ),
            Caveat::GpuAuto { job } => format!(
                "{job}: gpu = \"auto\" has no CWL equivalent; the job runs without a GPU \
                 unless the runner adds one."
            ),
        })
        .collect()
}

/// Every target pulls by tag, so a local image has to exist first.
pub(super) fn local_image_warning(job: &str, image: &str, context: &Path) -> String {
    format!(
        "{job}: image {image} is built from apps/, not pulled; build it before running: \
         docker build -t {image} {}",
        context.display()
    )
}

/// CWL reads `$(` and `${` in literal text as expressions; a backslash keeps them literal.
fn escape(text: &str) -> String {
    text.replace("$(", "\\$(").replace("${", "\\${")
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
    fn no_driver_carries_an_unescaped_cwl_expression() {
        let mut j = job("02-x");
        j.takes_input_files = false;
        j.dependencies = vec!["01-a".to_string(), "01-b".to_string()];
        j.inputs = vec!["*.pdb".to_string(), "a b.txt".to_string()];
        j.outputs = vec!["$(echo x).txt".to_string()];
        j.scripts = vec!["./pre_run.sh".to_string(), "./run.sh".to_string()];
        let text = driver(&j);
        assert!(text.contains("*.pdb|a\\ b.txt) return 0"), "{text}");
        assert!(text.contains("mv outputs ../02-x"), "{text}");
        let escaped = escape(&text);
        let bare = escaped.replace("\\$(", "").replace("\\${", "");
        assert!(!bare.contains("$(") && !bare.contains("${"), "{escaped}");
    }

    #[test]
    fn limits_become_cwl_requirements() {
        let mut j = job("01-a");
        j.timeout_secs = Some(1800);
        j.cpus = Some(0.5);
        j.memory_bytes = Some(3 * 1024 * 1024 * 1024 / 2);
        j.gpu_required = true;
        let doc = tool(&j);
        assert_eq!(doc["requirements"]["NetworkAccess"]["networkAccess"], true);
        assert_eq!(doc["requirements"]["ToolTimeLimit"]["timelimit"], 1800);
        assert_eq!(doc["requirements"]["ResourceRequirement"]["coresMin"], 0.5);
        assert_eq!(doc["requirements"]["ResourceRequirement"]["ramMin"], 1536);
        assert_eq!(
            doc["hints"]["cwltool:CUDARequirement"]["cudaDeviceCountMin"],
            1
        );
    }

    #[test]
    fn params_reach_the_tool_as_param_environment_variables() {
        let mut j = job("01-a");
        j.env = vec![
            ("extra".to_string(), EnvSource::Constant("$(x)".to_string())),
            ("mode".to_string(), EnvSource::Job("mode".to_string())),
        ];
        let doc = tool(&j);
        let env = &doc["requirements"]["EnvVarRequirement"]["envDef"];
        assert_eq!(env["PARAM_MODE"], "$(inputs.param_mode)");
        assert_eq!(env["PARAM_EXTRA"], "\\$(x)");
        assert_eq!(doc["inputs"]["param_mode"]["type"], "Any");
    }

    #[test]
    fn relative_locations_climb_out_of_the_output_folder() {
        assert_eq!(
            relative(Path::new("/w/cwl"), Path::new("/w/01-a")),
            "../01-a"
        );
        assert_eq!(
            relative(Path::new("/out"), Path::new("/w/my job")),
            "../w/my%20job"
        );
    }
}
