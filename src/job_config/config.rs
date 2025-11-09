use serde::{Deserialize, Deserializer};
use std::fmt;
use std::fs;
use std::path::Path;

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
    IoError(std::io::Error),
}

impl fmt::Display for JobConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobConfigError::FileNotFound(path) => write!(f, "Configuration file not found: {path}"),
            JobConfigError::InvalidToml(err) => write!(f, "Invalid TOML syntax: {err}"),
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

impl JobConfig {
    /// Loads a job configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML configuration file (typically "@job.toml")
    ///
    /// # Returns
    ///
    /// * `Ok(JobConfig)` - Successfully parsed configuration
    /// * `Err(JobConfigError)` - Error reading file or parsing TOML
    ///
    /// # Example
    ///
    /// ```no_run
    /// use silva::job_config::config::JobConfig;
    ///
    /// let config = JobConfig::load_from_file("@job.toml").unwrap();
    /// ```
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, JobConfigError> {
        let content = fs::read_to_string(path)?;
        let config: JobConfig = toml::from_str(&content)?;
        Ok(config)
    }
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
}
