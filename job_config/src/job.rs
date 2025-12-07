use serde::{Deserialize, Serialize};
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

/// Represents a parameter definition in job.toml.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamDefinition {
    #[serde(rename = "type")]
    pub param_type: ParamType,
    pub default: toml::Value,
    pub hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

impl ParamDefinition {
    /// Creates a new parameter definition.
    pub fn new(
        param_type: ParamType,
        default: toml::Value,
        hint: String,
        enum_values: Option<Vec<String>>,
    ) -> Self {
        Self {
            param_type,
            default,
            hint,
            enum_values,
        }
    }

    /// Validates a value against this parameter definition.
    pub fn validate(&self, value: &toml::Value) -> Result<(), String> {
        match self.param_type {
            ParamType::String => {
                if !value.is_str() {
                    return Err(format!("Expected string, got {value}"));
                }
            }
            ParamType::Integer => {
                if !value.is_integer() {
                    return Err(format!("Expected integer, got {value}"));
                }
            }
            ParamType::Float => {
                if !value.is_float() && !value.is_integer() {
                    return Err(format!("Expected float, got {value}"));
                }
            }
            ParamType::Boolean => {
                if !value.is_bool() {
                    return Err(format!("Expected boolean, got {value}"));
                }
            }
            ParamType::File | ParamType::Directory => {
                if !value.is_str() {
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

/// Represents the container configuration for a job.
/// Contains the Docker image URL and optional GPU support flag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Container {
    /// Docker image URL (e.g., "ubuntu:22.04", "python:3.11")
    pub image: String,
    /// Enable GPU support for this job (requires NVIDIA Container Toolkit).
    #[serde(default)]
    pub use_gpu: bool,
}

impl Container {
    /// Creates a new container configuration with the given image.
    pub fn new(image: String) -> Self {
        Self {
            image,
            use_gpu: false,
        }
    }

    /// Creates a new container configuration with GPU support.
    pub fn with_gpu(image: String) -> Self {
        Self {
            image,
            use_gpu: true,
        }
    }
}

/// Represents the scripts that will be executed for a job.
/// All fields are optional and have default values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Error type for job configuration operations.
#[derive(Debug)]
pub enum JobError {
    FileNotFound(String),
    InvalidToml(toml::de::Error),
    SerializeError(String),
    IoError(std::io::Error),
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobError::FileNotFound(path) => write!(f, "Configuration file not found: {path}"),
            JobError::InvalidToml(err) => write!(f, "Invalid TOML syntax: {err}"),
            JobError::SerializeError(err) => write!(f, "Serialization error: {err}"),
            JobError::IoError(err) => write!(f, "IO error: {err}"),
        }
    }
}

impl std::error::Error for JobError {}

impl From<std::io::Error> for JobError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::NotFound {
            JobError::FileNotFound(err.to_string())
        } else {
            JobError::IoError(err)
        }
    }
}

impl From<toml::de::Error> for JobError {
    fn from(err: toml::de::Error) -> Self {
        JobError::InvalidToml(err)
    }
}

// Note: JobParams type has been moved to params.rs and now uses serde_json::Value.
// Import from crate::params::JobParams for parameter value storage.

/// Main configuration structure for a computation job.
/// Merges the former JobConfig and NodeMetadata into a single TOML file.
///
/// Note: Job dependencies are now defined at the workflow level in WorkflowMetadata,
/// not in individual job configurations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobMeta {
    /// Job name for display purposes.
    pub name: String,
    /// Job description.
    pub description: String,
    /// Container configuration (includes image and GPU settings).
    pub container: Container,
    /// Scripts to execute.
    #[serde(default)]
    pub scripts: Scripts,
    /// List of input file patterns to copy from dependent jobs.
    #[serde(default)]
    pub inputs: Vec<String>,
    /// List of output file patterns to collect from the job.
    #[serde(default)]
    pub outputs: Vec<String>,
    /// Parameter definitions for this job.
    #[serde(default)]
    pub params: HashMap<String, ParamDefinition>,
}

impl JobMeta {
    /// Creates a new job metadata with required fields.
    pub fn new(name: String, description: String, container: Container) -> Self {
        Self {
            name,
            description,
            container,
            scripts: Scripts::default(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            params: HashMap::new(),
        }
    }

    /// Adds a parameter definition to this job.
    pub fn add_param(&mut self, name: String, definition: ParamDefinition) {
        self.params.insert(name, definition);
    }

    /// Loads job metadata from a TOML file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, JobError> {
        let content = fs::read_to_string(path)?;
        let meta: JobMeta = toml::from_str(&content)?;
        Ok(meta)
    }

    /// Saves job metadata to a TOML file.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), JobError> {
        let toml_str = toml::to_string_pretty(self)
            .map_err(|e| JobError::SerializeError(e.to_string()))?;
        fs::write(path, toml_str)?;
        Ok(())
    }

    /// Validates a params HashMap against this job's parameter definitions.
    /// Accepts JSON-based JobParams and converts values for validation against TOML definitions.
    pub fn validate_params(&self, params: &crate::params::JobParams) -> Result<(), String> {
        for (param_name, param_value) in params {
            if let Some(param_def) = self.params.get(param_name) {
                // Convert JSON value to TOML for validation
                let toml_value = crate::params::json_to_toml(param_value);
                param_def.validate(&toml_value)?;
            } else {
                return Err(format!("Unknown parameter: {param_name}"));
            }
        }
        Ok(())
    }

    /// Generates default parameters based on the parameter definitions.
    /// Returns JSON-based JobParams converted from TOML defaults.
    pub fn generate_default_params(&self) -> crate::params::JobParams {
        self.params
            .iter()
            .map(|(name, def)| (name.clone(), crate::params::toml_to_json(&def.default)))
            .collect()
    }
}

// Note: load_params and save_params have been moved to params.rs
// Use crate::params::load_job_params and crate::params::save_job_params instead.

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
    fn test_parse_job_meta_basic() {
        let toml_str = r#"
            name = "Test Job"
            description = "A test job"

            [container]
            image = "ubuntu:22.04"
        "#;

        let meta: JobMeta = toml::from_str(toml_str).unwrap();
        assert_eq!(meta.name, "Test Job");
        assert_eq!(meta.description, "A test job");
        assert_eq!(meta.container.image, "ubuntu:22.04");
        assert!(!meta.container.use_gpu);
    }

    #[test]
    fn test_parse_job_meta_with_scripts() {
        let toml_str = r#"
            name = "Test Job"
            description = "A test job"

            [container]
            image = "ubuntu:22.04"

            [scripts]
            pre = "setup.sh"
            run = "compute.sh"
            post = "cleanup.sh"
        "#;

        let meta: JobMeta = toml::from_str(toml_str).unwrap();
        assert_eq!(meta.scripts.pre, "setup.sh");
        assert_eq!(meta.scripts.run, "compute.sh");
        assert_eq!(meta.scripts.post, "cleanup.sh");
    }

    #[test]
    fn test_parse_job_meta_with_params() {
        let toml_str = r#"
            name = "Test Job"
            description = "A test job"

            [container]
            image = "ubuntu:22.04"

            [params.pdb_id]
            type = "string"
            default = "4OHU"
            hint = "The ID of the PDB file to download."

            [params.num_iterations]
            type = "integer"
            default = 100
            hint = "Number of iterations to run."
        "#;

        let meta: JobMeta = toml::from_str(toml_str).unwrap();
        assert_eq!(meta.params.len(), 2);

        let pdb_id = meta.params.get("pdb_id").unwrap();
        assert_eq!(pdb_id.param_type, ParamType::String);
        assert_eq!(pdb_id.default.as_str().unwrap(), "4OHU");

        let num_iterations = meta.params.get("num_iterations").unwrap();
        assert_eq!(num_iterations.param_type, ParamType::Integer);
        assert_eq!(num_iterations.default.as_integer().unwrap(), 100);
    }

    #[test]
    fn test_parse_job_meta_with_enum_param() {
        let toml_str = r#"
            name = "Test Job"
            description = "A test job"

            [container]
            image = "ubuntu:22.04"

            [params.format]
            type = "enum"
            default = "pdb"
            hint = "Output format"
            enum_values = ["pdb", "cif", "xml"]
        "#;

        let meta: JobMeta = toml::from_str(toml_str).unwrap();
        let format_param = meta.params.get("format").unwrap();
        assert_eq!(format_param.param_type, ParamType::Enum);
        assert_eq!(
            format_param.enum_values,
            Some(vec!["pdb".to_string(), "cif".to_string(), "xml".to_string()])
        );
    }

    #[test]
    fn test_parse_job_meta_with_io_patterns() {
        let toml_str = r#"
            name = "Test Job"
            description = "A test job"
            inputs = ["*.csv"]
            outputs = ["results/*.json"]

            [container]
            image = "ubuntu:22.04"
        "#;

        let meta: JobMeta = toml::from_str(toml_str).unwrap();
        assert_eq!(meta.inputs, vec!["*.csv"]);
        assert_eq!(meta.outputs, vec!["results/*.json"]);
    }

    #[test]
    fn test_validate_params() {
        let toml_str = r#"
            name = "Test Job"
            description = "A test job"

            [container]
            image = "ubuntu:22.04"

            [params.count]
            type = "integer"
            default = 10
            hint = "A count"
        "#;

        let meta: JobMeta = toml::from_str(toml_str).unwrap();

        // Use JSON-based params (from params.rs)
        let mut params = crate::params::JobParams::new();
        params.insert("count".to_string(), serde_json::json!(42));
        assert!(meta.validate_params(&params).is_ok());

        params.insert("count".to_string(), serde_json::json!("not a number"));
        assert!(meta.validate_params(&params).is_err());
    }

    #[test]
    fn test_generate_default_params() {
        let toml_str = r#"
            name = "Test Job"
            description = "A test job"

            [container]
            image = "ubuntu:22.04"

            [params.name]
            type = "string"
            default = "test"
            hint = "A name"

            [params.count]
            type = "integer"
            default = 10
            hint = "A count"
        "#;

        let meta: JobMeta = toml::from_str(toml_str).unwrap();
        let defaults = meta.generate_default_params();

        // Now returns JSON values
        assert_eq!(defaults.get("name").unwrap().as_str().unwrap(), "test");
        assert_eq!(defaults.get("count").unwrap().as_i64().unwrap(), 10);
    }

    #[test]
    fn test_gpu_support() {
        let toml_str = r#"
            name = "GPU Job"
            description = "A GPU job"

            [container]
            image = "nvidia/cuda:11.8.0-base-ubuntu22.04"
            use_gpu = true
        "#;

        let meta: JobMeta = toml::from_str(toml_str).unwrap();
        assert!(meta.container.use_gpu);
        assert_eq!(meta.container.image, "nvidia/cuda:11.8.0-base-ubuntu22.04");
    }

    #[test]
    fn test_container_new() {
        let container = Container::new("ubuntu:22.04".to_string());
        assert_eq!(container.image, "ubuntu:22.04");
        assert!(!container.use_gpu);
    }

    #[test]
    fn test_container_with_gpu() {
        let container = Container::with_gpu("nvidia/cuda:11.8.0".to_string());
        assert_eq!(container.image, "nvidia/cuda:11.8.0");
        assert!(container.use_gpu);
    }
}
