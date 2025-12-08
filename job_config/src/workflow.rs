use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::job::{JobError, ParamDefinition};
use crate::params::{WorkflowParams, toml_to_json, json_to_toml};

// Re-export WorkflowParams from params module for convenience
pub use crate::params::WorkflowParams as WorkflowParamsType;

/// Represents the metadata for a workflow (workflow.toml).
/// Contains workflow-level configuration including global parameters and job dependencies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowMeta {
    pub name: String,
    pub description: String,
    /// Job dependencies mapping: job_name -> list of jobs it depends on.
    /// Example: { "job_2": ["job_1"], "job_3": ["job_1", "job_2"] }
    #[serde(default)]
    pub dependencies: HashMap<String, Vec<String>>,
    /// Global parameter definitions for this workflow.
    #[serde(default)]
    pub params: HashMap<String, ParamDefinition>,
}

impl WorkflowMeta {
    /// Creates a new workflow metadata with the given name and description.
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            dependencies: HashMap::new(),
            params: HashMap::new(),
        }
    }

    /// Gets the dependencies for a specific job.
    /// Returns an empty slice if the job has no dependencies.
    pub fn get_job_dependencies(&self, job_name: &str) -> &[String] {
        self.dependencies
            .get(job_name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Sets the dependencies for a specific job.
    pub fn set_job_dependencies(&mut self, job_name: String, deps: Vec<String>) {
        self.dependencies.insert(job_name, deps);
    }

    /// Adds a parameter definition to this workflow.
    pub fn add_param(&mut self, name: String, definition: ParamDefinition) {
        self.params.insert(name, definition);
    }

    /// Loads workflow metadata from a TOML file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, JobError> {
        let content = fs::read_to_string(path)?;
        let metadata: WorkflowMeta = toml::from_str(&content)?;
        Ok(metadata)
    }

    /// Saves workflow metadata to a TOML file.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), JobError> {
        let toml_str = toml::to_string_pretty(self).map_err(|e| {
            JobError::SerializeError(e.to_string())
        })?;
        fs::write(path, toml_str)?;
        Ok(())
    }

    /// Validates a params HashMap against this workflow's parameter definitions.
    /// Accepts JSON-based WorkflowParams and converts values for validation against TOML definitions.
    pub fn validate_params(
        &self,
        params: &WorkflowParams,
    ) -> Result<(), String> {
        for (param_name, param_value) in params {
            if let Some(param_def) = self.params.get(param_name) {
                // Convert JSON value to TOML for validation
                let toml_value = json_to_toml(param_value);
                param_def.validate(&toml_value)?;
            } else {
                return Err(format!("Unknown parameter: {param_name}"));
            }
        }
        Ok(())
    }

    /// Generates default parameters based on the parameter definitions.
    /// Returns JSON-based WorkflowParams converted from TOML defaults.
    pub fn generate_default_params(&self) -> WorkflowParams {
        self.params
            .iter()
            .map(|(name, def)| (name.clone(), toml_to_json(&def.default)))
            .collect()
    }
}

// Note: load_workflow_params and save_workflow_params have been moved to params.rs
// Use crate::params::load_workflow_params and crate::params::save_workflow_params instead.
// They now use JSON format (global_params.json) instead of TOML.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_meta_new() {
        let metadata = WorkflowMeta::new("Test Workflow".to_string(), "A test workflow".to_string());
        assert_eq!(metadata.name, "Test Workflow");
        assert_eq!(metadata.description, "A test workflow");
        assert!(metadata.params.is_empty());
        assert!(metadata.dependencies.is_empty());
    }

    #[test]
    fn test_workflow_meta_parse_toml() {
        let toml_str = r#"
            name = "ML Pipeline"
            description = "A machine learning pipeline"
        "#;

        let metadata: WorkflowMeta = toml::from_str(toml_str).unwrap();
        assert_eq!(metadata.name, "ML Pipeline");
        assert_eq!(metadata.description, "A machine learning pipeline");
    }

    #[test]
    fn test_workflow_meta_with_dependencies() {
        let toml_str = r#"
            name = "ML Pipeline"
            description = "A machine learning pipeline"

            [dependencies]
            job_2 = ["job_1"]
            job_3 = ["job_1", "job_2"]
        "#;

        let metadata: WorkflowMeta = toml::from_str(toml_str).unwrap();
        assert_eq!(metadata.name, "ML Pipeline");

        // Check dependencies
        assert_eq!(metadata.get_job_dependencies("job_1"), &[] as &[String]);
        assert_eq!(metadata.get_job_dependencies("job_2"), &["job_1".to_string()]);
        assert_eq!(metadata.get_job_dependencies("job_3"), &["job_1".to_string(), "job_2".to_string()]);
    }

    #[test]
    fn test_workflow_meta_set_dependencies() {
        let mut metadata = WorkflowMeta::new("Test".to_string(), "Test workflow".to_string());

        metadata.set_job_dependencies("job_2".to_string(), vec!["job_1".to_string()]);
        assert_eq!(metadata.get_job_dependencies("job_2"), &["job_1".to_string()]);
    }

    #[test]
    fn test_workflow_params_parse_json() {
        let json_str = r#"{
            "input_path": "/data/input",
            "batch_size": 32,
            "learning_rate": 0.001
        }"#;

        let params: WorkflowParams = serde_json::from_str(json_str).unwrap();
        assert_eq!(params.get("input_path").unwrap().as_str().unwrap(), "/data/input");
        assert_eq!(params.get("batch_size").unwrap().as_i64().unwrap(), 32);
    }
}
