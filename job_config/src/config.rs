use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;

/// Represents the type of a parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    String,
    Integer,
    Float,
    Boolean,
    File,
    Directory,
    Enum,
    Array,
}

impl fmt::Display for ParamType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamType::String => write!(f, "string"),
            ParamType::Integer => write!(f, "integer"),
            ParamType::Float => write!(f, "float"),
            ParamType::Boolean => write!(f, "boolean"),
            ParamType::File => write!(f, "file"),
            ParamType::Directory => write!(f, "directory"),
            ParamType::Enum => write!(f, "enum"),
            ParamType::Array => write!(f, "array"),
        }
    }
}

/// Represents a parameter definition in node.json.
/// The format matches the spec: [type, default_value, hint, optional_enum_values]
/// Example: ["string", "4OHU", "The ID of the PDB file to download."]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(into = "ParamDefinitionArray")]
pub struct ParamDefinition {
    pub param_type: ParamType,
    pub default_value: serde_json::Value,
    pub hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

/// Helper struct for serializing ParamDefinition as array
#[derive(Serialize)]
#[serde(untagged)]
enum ParamDefinitionArray {
    WithEnum(String, serde_json::Value, String, Vec<String>),
    WithoutEnum(String, serde_json::Value, String),
}

impl From<ParamDefinition> for ParamDefinitionArray {
    fn from(def: ParamDefinition) -> Self {
        let type_str = def.param_type.to_string();
        if let Some(enum_vals) = def.enum_values {
            ParamDefinitionArray::WithEnum(type_str, def.default_value, def.hint, enum_vals)
        } else {
            ParamDefinitionArray::WithoutEnum(type_str, def.default_value, def.hint)
        }
    }
}

impl<'de> Deserialize<'de> for ParamDefinition {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        // Deserialize as a generic Value first
        let value = serde_json::Value::deserialize(deserializer)?;

        match value {
            serde_json::Value::Array(arr) => {
                // Array format: ["type", default_value, "hint"] or ["type", default_value, "hint", ["enum", "values"]]
                if arr.len() < 3 {
                    return Err(D::Error::custom(format!(
                        "ParamDefinition array must have at least 3 elements, got {}",
                        arr.len()
                    )));
                }

                // Parse type (first element)
                let param_type = arr[0]
                    .as_str()
                    .ok_or_else(|| D::Error::custom("First element must be a string (type)"))?;
                let param_type: ParamType =
                    serde_json::from_value(serde_json::Value::String(param_type.to_string()))
                        .map_err(|e| D::Error::custom(format!("Invalid param type: {e}")))?;

                // Parse default value (second element)
                let default_value = arr[1].clone();

                // Parse hint (third element)
                let hint = arr[2]
                    .as_str()
                    .ok_or_else(|| D::Error::custom("Third element must be a string (hint)"))?
                    .to_string();

                // Parse optional enum values (fourth element)
                let enum_values = if arr.len() > 3 {
                    let enum_arr = arr[3].as_array().ok_or_else(|| {
                        D::Error::custom("Fourth element must be an array (enum values)")
                    })?;
                    let values: Result<Vec<String>, _> = enum_arr
                        .iter()
                        .map(|v| {
                            v.as_str()
                                .ok_or_else(|| D::Error::custom("Enum values must be strings"))
                                .map(|s| s.to_string())
                        })
                        .collect();
                    Some(values?)
                } else {
                    None
                };

                Ok(ParamDefinition {
                    param_type,
                    default_value,
                    hint,
                    enum_values,
                })
            }
            serde_json::Value::Object(_) => {
                // Also support object format for compatibility
                #[derive(Deserialize)]
                struct ParamDefObject {
                    param_type: ParamType,
                    default_value: serde_json::Value,
                    hint: String,
                    enum_values: Option<Vec<String>>,
                }

                let obj: ParamDefObject = serde_json::from_value(value).map_err(|e| {
                    D::Error::custom(format!("Invalid ParamDefinition object: {e}"))
                })?;

                Ok(ParamDefinition {
                    param_type: obj.param_type,
                    default_value: obj.default_value,
                    hint: obj.hint,
                    enum_values: obj.enum_values,
                })
            }
            _ => Err(D::Error::custom(
                "ParamDefinition must be an array or object",
            )),
        }
    }
}

impl ParamDefinition {
    /// Creates a new parameter definition.
    pub fn new(
        param_type: ParamType,
        default_value: serde_json::Value,
        hint: String,
        enum_values: Option<Vec<String>>,
    ) -> Self {
        Self {
            param_type,
            default_value,
            hint,
            enum_values,
        }
    }

    /// Validates a value against this parameter definition.
    pub fn validate(&self, value: &serde_json::Value) -> Result<(), String> {
        match self.param_type {
            ParamType::String => {
                if !value.is_string() {
                    return Err(format!("Expected string, got {value}"));
                }
            }
            ParamType::Integer => {
                if !value.is_i64() && !value.is_u64() {
                    return Err(format!("Expected integer, got {value}"));
                }
            }
            ParamType::Float => {
                if !value.is_f64() && !value.is_i64() && !value.is_u64() {
                    return Err(format!("Expected float, got {value}"));
                }
            }
            ParamType::Boolean => {
                if !value.is_boolean() {
                    return Err(format!("Expected boolean, got {value}"));
                }
            }
            ParamType::File | ParamType::Directory => {
                if !value.is_string() {
                    return Err(format!("Expected path string, got {value}"));
                }
            }
            ParamType::Enum => {
                if let Some(enum_vals) = &self.enum_values {
                    if let Some(val_str) = value.as_str() {
                        if !enum_vals.contains(&val_str.to_string()) {
                            return Err(format!(
                                "Value '{val_str}' not in allowed values: {enum_vals:?}"
                            ));
                        }
                    } else {
                        return Err(format!("Expected string for enum, got {value}"));
                    }
                } else {
                    return Err("Enum type requires enum_values to be specified".to_string());
                }
            }
            ParamType::Array => {
                if !value.is_array() {
                    return Err(format!("Expected array, got {value}"));
                }
            }
        }
        Ok(())
    }
}

/// Represents the metadata for a job node (node.json).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub name: String,
    pub description: String,
    pub params: HashMap<String, ParamDefinition>,
}

impl NodeMetadata {
    /// Creates a new node metadata with the given name and description.
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            params: HashMap::new(),
        }
    }

    /// Adds a parameter definition to this node.
    pub fn add_param(&mut self, name: String, definition: ParamDefinition) {
        self.params.insert(name, definition);
    }

    /// Loads node metadata from a JSON file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, JobConfigError> {
        let content = fs::read_to_string(path)?;
        let metadata: NodeMetadata =
            serde_json::from_str(&content).map_err(JobConfigError::InvalidJson)?;
        Ok(metadata)
    }

    /// Saves node metadata to a JSON file.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), JobConfigError> {
        let json = serde_json::to_string_pretty(self).map_err(JobConfigError::InvalidJson)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Validates a params HashMap against this node's parameter definitions.
    pub fn validate_params(
        &self,
        params: &HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        for (param_name, param_value) in params {
            if let Some(param_def) = self.params.get(param_name) {
                param_def.validate(param_value)?;
            } else {
                return Err(format!("Unknown parameter: {param_name}"));
            }
        }
        Ok(())
    }

    /// Generates default parameters based on the parameter definitions.
    pub fn generate_default_params(&self) -> HashMap<String, serde_json::Value> {
        self.params
            .iter()
            .map(|(name, def)| (name.clone(), def.default_value.clone()))
            .collect()
    }
}

/// Represents the container configuration for a job.
/// Can be either a Docker image URL or a path to a Dockerfile.
#[derive(Debug, Clone, PartialEq)]
pub enum Container {
    DockerImage(String),
    DockerFile(String),
}

impl<'de> Deserialize<'de> for Container {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ContainerHelper {
            docker_image: Option<String>,
            docker_file: Option<String>,
        }

        let helper = ContainerHelper::deserialize(deserializer)?;

        match (helper.docker_image, helper.docker_file) {
            (Some(image), None) => Ok(Container::DockerImage(image)),
            (None, Some(dockerfile)) => Ok(Container::DockerFile(dockerfile)),
            (Some(_), Some(_)) => Err(serde::de::Error::custom(
                "container section cannot have both 'docker_image' and 'docker_file'",
            )),
            (None, None) => Err(serde::de::Error::custom(
                "container section must have either 'docker_image' or 'docker_file'",
            )),
        }
    }
}

/// Represents the scripts that will be executed for a job.
/// All fields are optional and have default values.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Scripts {
    #[serde(default = "default_pre_script")]
    pub pre: String,
    #[serde(default = "default_run_script")]
    pub run: String,
    #[serde(default = "default_post_script")]
    pub post: String,
}

fn default_pre_script() -> String {
    "./pre_run.sh".to_string()
}

fn default_run_script() -> String {
    "./run.sh".to_string()
}

fn default_post_script() -> String {
    "./post_run.sh".to_string()
}

impl Default for Scripts {
    fn default() -> Self {
        Self {
            pre: default_pre_script(),
            run: default_run_script(),
            post: default_post_script(),
        }
    }
}

/// Main configuration structure for a computation job.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct JobConfig {
    pub container: Container,
    #[serde(default)]
    pub scripts: Scripts,
    /// Enable GPU support for this job (requires NVIDIA Container Toolkit).
    /// Defaults to false.
    #[serde(default)]
    pub use_gpu: bool,
    /// List of input file patterns to copy from dependent jobs.
    /// Supports glob patterns (e.g., "*.csv", "data_*.json").
    /// If empty, all output files from dependencies will be copied.
    /// Defaults to empty vector.
    #[serde(default)]
    pub inputs: Vec<String>,
    /// List of output file patterns to collect from the job.
    /// Supports glob patterns (e.g., "results/*.json", "*.png").
    /// Output files are collected into an "outputs/" folder in the job directory.
    /// Defaults to empty vector.
    #[serde(default)]
    pub outputs: Vec<String>,
    /// List of job names that this job depends on.
    /// Jobs will execute in dependency order (topological sort).
    /// Defaults to empty vector.
    #[serde(default)]
    pub depends_on: Vec<String>,
}

/// Error type for job configuration operations.
#[derive(Debug)]
pub enum JobConfigError {
    FileNotFound(String),
    InvalidToml(toml::de::Error),
    InvalidJson(serde_json::Error),
    IoError(std::io::Error),
}

impl fmt::Display for JobConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobConfigError::FileNotFound(path) => write!(f, "Configuration file not found: {path}"),
            JobConfigError::InvalidToml(err) => write!(f, "Invalid TOML syntax: {err}"),
            JobConfigError::InvalidJson(err) => write!(f, "Invalid JSON syntax: {err}"),
            JobConfigError::IoError(err) => write!(f, "IO error: {err}"),
        }
    }
}

impl std::error::Error for JobConfigError {}

impl From<std::io::Error> for JobConfigError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::NotFound {
            JobConfigError::FileNotFound(err.to_string())
        } else {
            JobConfigError::IoError(err)
        }
    }
}

impl From<toml::de::Error> for JobConfigError {
    fn from(err: toml::de::Error) -> Self {
        JobConfigError::InvalidToml(err)
    }
}

impl From<serde_json::Error> for JobConfigError {
    fn from(err: serde_json::Error) -> Self {
        JobConfigError::InvalidJson(err)
    }
}

impl JobConfig {
    /// Loads a job configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML configuration file (typically ".chiral/job.toml")
    ///
    /// # Returns
    ///
    /// * `Ok(JobConfig)` - Successfully parsed configuration
    /// * `Err(JobConfigError)` - Error reading file or parsing TOML
    ///
    /// # Example
    ///
    /// ```no_run
    /// use job_config::config::JobConfig;
    ///
    /// let config = JobConfig::load_from_file(".chiral/job.toml").unwrap();
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, JobConfigError> {
        let content = fs::read_to_string(path)?;
        let config: JobConfig = toml::from_str(&content)?;
        Ok(config)
    }
}

/// Type alias for job parameters (params.json content).
pub type JobParams = HashMap<String, serde_json::Value>;

/// Loads job parameters from a JSON file.
///
/// # Arguments
///
/// * `path` - Path to the params.json file
///
/// # Returns
///
/// * `Ok(JobParams)` - Successfully parsed parameters
/// * `Err(JobConfigError)` - Error reading file or parsing JSON
pub fn load_params<P: AsRef<Path>>(path: P) -> Result<JobParams, JobConfigError> {
    let content = fs::read_to_string(path)?;
    let params: JobParams = serde_json::from_str(&content)?;
    Ok(params)
}

/// Saves job parameters to a JSON file.
///
/// # Arguments
///
/// * `path` - Path to the params.json file
/// * `params` - Parameters to save
///
/// # Returns
///
/// * `Ok(())` - Successfully saved parameters
/// * `Err(JobConfigError)` - Error writing file or serializing JSON
pub fn save_params<P: AsRef<Path>>(path: P, params: &JobParams) -> Result<(), JobConfigError> {
    let json = serde_json::to_string_pretty(params)?;
    fs::write(path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scripts_default_values() {
        let scripts = Scripts::default();
        assert_eq!(scripts.pre, "./pre_run.sh");
        assert_eq!(scripts.run, "./run.sh");
        assert_eq!(scripts.post, "./post_run.sh");
    }

    #[test]
    fn test_parse_config_with_docker_image() {
        let toml_str = r#"
            [container]
            docker_image = "ubuntu:22.04"

            [scripts]
            pre = "setup.sh"
            run = "compute.sh"
            post = "cleanup.sh"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.container,
            Container::DockerImage("ubuntu:22.04".to_string())
        );
        assert_eq!(config.scripts.pre, "setup.sh");
        assert_eq!(config.scripts.run, "compute.sh");
        assert_eq!(config.scripts.post, "cleanup.sh");
    }

    #[test]
    fn test_parse_config_with_dockerfile() {
        let toml_str = r#"
            [container]
            docker_file = "./Dockerfile"

            [scripts]
            run = "execute.sh"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.container,
            Container::DockerFile("./Dockerfile".to_string())
        );
        assert_eq!(config.scripts.run, "execute.sh");
        // Check defaults are applied
        assert_eq!(config.scripts.pre, "./pre_run.sh");
        assert_eq!(config.scripts.post, "./post_run.sh");
    }

    #[test]
    fn test_parse_config_with_default_scripts() {
        let toml_str = r#"
            [container]
            docker_image = "alpine:latest"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.container,
            Container::DockerImage("alpine:latest".to_string())
        );
        // All scripts should use defaults
        assert_eq!(config.scripts.pre, "./pre_run.sh");
        assert_eq!(config.scripts.run, "./run.sh");
        assert_eq!(config.scripts.post, "./post_run.sh");
    }

    #[test]
    fn test_parse_config_with_partial_scripts() {
        let toml_str = r#"
            [container]
            docker_image = "python:3.11"

            [scripts]
            run = "main.py"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.scripts.pre, "./pre_run.sh");
        assert_eq!(config.scripts.run, "main.py");
        assert_eq!(config.scripts.post, "./post_run.sh");
    }

    #[test]
    fn test_error_both_container_types() {
        let toml_str = r#"
            [container]
            docker_image = "ubuntu:22.04"
            docker_file = "./Dockerfile"
        "#;

        let result: Result<JobConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("both"));
    }

    #[test]
    fn test_error_no_container_type() {
        let toml_str = r#"
            [container]

            [scripts]
            run = "test.sh"
        "#;

        let result: Result<JobConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("must have either"));
    }

    #[test]
    fn test_error_missing_container_section() {
        let toml_str = r#"
            [scripts]
            run = "test.sh"
        "#;

        let result: Result<JobConfig, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_nonexistent_file() {
        let result = JobConfig::load_from_file("nonexistent_file.toml");
        assert!(result.is_err());
        match result.unwrap_err() {
            JobConfigError::FileNotFound(_) | JobConfigError::IoError(_) => {}
            _ => panic!("Expected FileNotFound or IoError"),
        }
    }

    #[test]
    fn test_gpu_disabled_by_default() {
        let toml_str = r#"
            [container]
            docker_image = "ubuntu:22.04"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.use_gpu);
    }

    #[test]
    fn test_gpu_enabled() {
        let toml_str = r#"
            use_gpu = true

            [container]
            docker_image = "nvidia/cuda:11.8.0-base-ubuntu22.04"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert!(config.use_gpu);
    }

    #[test]
    fn test_gpu_explicitly_disabled() {
        let toml_str = r#"
            use_gpu = false

            [container]
            docker_image = "ubuntu:22.04"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.use_gpu);
    }

    #[test]
    fn test_inputs_outputs_depends_on_defaults() {
        let toml_str = r#"
            [container]
            docker_image = "ubuntu:22.04"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert!(config.inputs.is_empty());
        assert!(config.outputs.is_empty());
        assert!(config.depends_on.is_empty());
    }

    #[test]
    fn test_parse_config_with_inputs() {
        let toml_str = r#"
            inputs = ["*.csv", "data_*.json"]

            [container]
            docker_image = "ubuntu:22.04"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.inputs.len(), 2);
        assert_eq!(config.inputs[0], "*.csv");
        assert_eq!(config.inputs[1], "data_*.json");
    }

    #[test]
    fn test_parse_config_with_outputs() {
        let toml_str = r#"
            outputs = ["results/*.json", "*.png", "report.pdf"]

            [container]
            docker_image = "python:3.11"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.outputs.len(), 3);
        assert_eq!(config.outputs[0], "results/*.json");
        assert_eq!(config.outputs[1], "*.png");
        assert_eq!(config.outputs[2], "report.pdf");
    }

    #[test]
    fn test_parse_config_with_depends_on() {
        let toml_str = r#"
            depends_on = ["job1", "job2", "preprocessing"]

            [container]
            docker_image = "alpine:latest"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.depends_on.len(), 3);
        assert_eq!(config.depends_on[0], "job1");
        assert_eq!(config.depends_on[1], "job2");
        assert_eq!(config.depends_on[2], "preprocessing");
    }

    #[test]
    fn test_parse_config_with_all_new_fields() {
        let toml_str = r#"
            inputs = ["*.csv"]
            outputs = ["results/*.json", "summary.txt"]
            depends_on = ["data_preparation"]

            [container]
            docker_image = "ubuntu:22.04"

            [scripts]
            run = "process.sh"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.inputs, vec!["*.csv"]);
        assert_eq!(config.outputs, vec!["results/*.json", "summary.txt"]);
        assert_eq!(config.depends_on, vec!["data_preparation"]);
        assert_eq!(config.scripts.run, "process.sh");
    }

    #[test]
    fn test_empty_inputs_outputs_depends_on() {
        let toml_str = r#"
            inputs = []
            outputs = []
            depends_on = []

            [container]
            docker_image = "ubuntu:22.04"
        "#;

        let config: JobConfig = toml::from_str(toml_str).unwrap();
        assert!(config.inputs.is_empty());
        assert!(config.outputs.is_empty());
        assert!(config.depends_on.is_empty());
    }

    #[test]
    fn test_param_definition_array_format() {
        // Test the array format from the spec: ["type", default_value, "hint"]
        let json_str = r#"["string", "4OHU", "The ID of the PDB file to download."]"#;
        let param_def: ParamDefinition = serde_json::from_str(json_str).unwrap();

        assert_eq!(param_def.param_type, ParamType::String);
        assert_eq!(
            param_def.default_value,
            serde_json::Value::String("4OHU".to_string())
        );
        assert_eq!(param_def.hint, "The ID of the PDB file to download.");
        assert_eq!(param_def.enum_values, None);
    }

    #[test]
    fn test_param_definition_array_format_with_enum() {
        // Test array format with enum values
        let json_str = r#"["enum", "pdb", "Output file format", ["pdb", "cif", "xml"]]"#;
        let param_def: ParamDefinition = serde_json::from_str(json_str).unwrap();

        assert_eq!(param_def.param_type, ParamType::Enum);
        assert_eq!(
            param_def.default_value,
            serde_json::Value::String("pdb".to_string())
        );
        assert_eq!(param_def.hint, "Output file format");
        assert_eq!(
            param_def.enum_values,
            Some(vec![
                "pdb".to_string(),
                "cif".to_string(),
                "xml".to_string()
            ])
        );
    }

    #[test]
    fn test_node_metadata_with_array_format() {
        // Test the full node.json format from the spec
        let json_str = r#"{
            "name": "1 Download PDB",
            "description": "Download a PDB file from the RCSB Protein Data Bank.",
            "params": {
                "pdb_id": ["string", "4OHU", "The ID of the PDB file to download."]
            }
        }"#;

        let metadata: NodeMetadata = serde_json::from_str(json_str).unwrap();

        assert_eq!(metadata.name, "1 Download PDB");
        assert_eq!(
            metadata.description,
            "Download a PDB file from the RCSB Protein Data Bank."
        );
        assert_eq!(metadata.params.len(), 1);

        let pdb_id_param = metadata.params.get("pdb_id").unwrap();
        assert_eq!(pdb_id_param.param_type, ParamType::String);
        assert_eq!(
            pdb_id_param.default_value,
            serde_json::Value::String("4OHU".to_string())
        );
        assert_eq!(pdb_id_param.hint, "The ID of the PDB file to download.");
    }

    #[test]
    fn test_param_definition_object_format_backward_compat() {
        // Test that object format still works for backward compatibility
        let json_str = r#"{
            "param_type": "integer",
            "default_value": 42,
            "hint": "Answer to everything"
        }"#;

        let param_def: ParamDefinition = serde_json::from_str(json_str).unwrap();

        assert_eq!(param_def.param_type, ParamType::Integer);
        assert_eq!(
            param_def.default_value,
            serde_json::Value::Number(42.into())
        );
        assert_eq!(param_def.hint, "Answer to everything");
        assert_eq!(param_def.enum_values, None);
    }

    #[test]
    fn test_param_definition_serialization_to_array() {
        // Test that serialization produces array format
        let param_def = ParamDefinition::new(
            ParamType::String,
            serde_json::Value::String("test".to_string()),
            "Test parameter".to_string(),
            None,
        );

        let json = serde_json::to_string(&param_def).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Should be an array
        assert!(parsed.is_array());
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], "string");
        assert_eq!(arr[1], "test");
        assert_eq!(arr[2], "Test parameter");
    }

    #[test]
    fn test_node_metadata_roundtrip() {
        // Test that we can deserialize and serialize correctly
        let json_str = r#"{
            "name": "Test Job",
            "description": "A test job",
            "params": {
                "param1": ["string", "default", "A string parameter"],
                "param2": ["integer", 100, "An integer parameter"]
            }
        }"#;

        let metadata: NodeMetadata = serde_json::from_str(json_str).unwrap();
        let serialized = serde_json::to_string_pretty(&metadata).unwrap();
        let deserialized: NodeMetadata = serde_json::from_str(&serialized).unwrap();

        assert_eq!(metadata, deserialized);
    }
}
