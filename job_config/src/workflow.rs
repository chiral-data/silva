use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::job::{JobError, ParamDefinition};

/// Type alias for workflow parameters (global_params.toml content).
pub type WorkflowParams = HashMap<String, toml::Value>;

/// Represents the metadata for a workflow (workflow.toml).
/// Similar to JobMeta but for workflow-level global parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub params: HashMap<String, ParamDefinition>,
}

impl WorkflowMetadata {
    /// Creates a new workflow metadata with the given name and description.
    pub fn new(name: String, description: String) -> Self {
        Self {
            name,
            description,
            params: HashMap::new(),
        }
    }

    /// Adds a parameter definition to this workflow.
    pub fn add_param(&mut self, name: String, definition: ParamDefinition) {
        self.params.insert(name, definition);
    }

    /// Loads workflow metadata from a TOML file.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, JobError> {
        let content = fs::read_to_string(path)?;
        let metadata: WorkflowMetadata = toml::from_str(&content)?;
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
    pub fn validate_params(
        &self,
        params: &WorkflowParams,
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
    pub fn generate_default_params(&self) -> WorkflowParams {
        self.params
            .iter()
            .map(|(name, def)| (name.clone(), def.default.clone()))
            .collect()
    }
}

/// Loads workflow parameters from a TOML file (global_params.toml).
///
/// # Arguments
///
/// * `path` - Path to the global_params.toml file
///
/// # Returns
///
/// * `Ok(WorkflowParams)` - Successfully parsed parameters
/// * `Err(JobError)` - Error reading file or parsing TOML
pub fn load_workflow_params<P: AsRef<Path>>(path: P) -> Result<WorkflowParams, JobError> {
    let content = fs::read_to_string(path)?;
    let params: WorkflowParams = toml::from_str(&content)?;
    Ok(params)
}

/// Saves workflow parameters to a TOML file (global_params.toml).
///
/// # Arguments
///
/// * `path` - Path to the global_params.toml file
/// * `params` - Parameters to save
///
/// # Returns
///
/// * `Ok(())` - Successfully saved parameters
/// * `Err(JobError)` - Error writing file or serializing TOML
pub fn save_workflow_params<P: AsRef<Path>>(
    path: P,
    params: &WorkflowParams,
) -> Result<(), JobError> {
    let toml_str = toml::to_string_pretty(params).map_err(|e| {
        JobError::SerializeError(e.to_string())
    })?;
    fs::write(path, toml_str)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_metadata_new() {
        let metadata = WorkflowMetadata::new("Test Workflow".to_string(), "A test workflow".to_string());
        assert_eq!(metadata.name, "Test Workflow");
        assert_eq!(metadata.description, "A test workflow");
        assert!(metadata.params.is_empty());
    }

    #[test]
    fn test_workflow_metadata_parse_toml() {
        let toml_str = r#"
            name = "ML Pipeline"
            description = "A machine learning pipeline"
        "#;

        let metadata: WorkflowMetadata = toml::from_str(toml_str).unwrap();
        assert_eq!(metadata.name, "ML Pipeline");
        assert_eq!(metadata.description, "A machine learning pipeline");
    }

    #[test]
    fn test_workflow_params_parse_toml() {
        let toml_str = r#"
            input_path = "/data/input"
            batch_size = 32
            learning_rate = 0.001
        "#;

        let params: WorkflowParams = toml::from_str(toml_str).unwrap();
        assert_eq!(params.get("input_path").unwrap().as_str().unwrap(), "/data/input");
        assert_eq!(params.get("batch_size").unwrap().as_integer().unwrap(), 32);
    }
}
