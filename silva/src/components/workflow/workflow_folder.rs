//! Workflow folder representation.
//!
//! This module provides the `WorkflowFolder` struct which represents a single
//! workflow directory and provides methods for loading/saving workflow metadata
//! and parameters.

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use job_config::job::JobError;
use job_config::params::{ParamsError, WorkflowParams, load_workflow_params, save_workflow_params};
use job_config::workflow::WorkflowMeta;

/// Represents a single workflow folder.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkflowFolder {
    pub name: String,
    pub path: PathBuf,
    pub created: Option<SystemTime>,
}

impl WorkflowFolder {
    /// Creates a new WorkflowFolder.
    pub fn new(name: String, path: PathBuf, created: Option<SystemTime>) -> Self {
        Self {
            name,
            path,
            created,
        }
    }

    /// Returns a display string for the creation time.
    pub fn created_display(&self) -> String {
        match self.created {
            Some(created) => {
                let elapsed = SystemTime::now()
                    .duration_since(created)
                    .unwrap_or_default();

                let secs = elapsed.as_secs();
                if secs < 60 {
                    format!("{secs}s ago")
                } else if secs < 3600 {
                    format!("{}m ago", secs / 60)
                } else if secs < 86400 {
                    format!("{}h ago", secs / 3600)
                } else {
                    format!("{}d ago", secs / 86400)
                }
            }
            None => "Unknown".to_string(),
        }
    }

    /// Returns the path to the workflow's .chiral directory.
    pub fn chiral_dir(&self) -> PathBuf {
        self.path.join(".chiral")
    }

    /// Returns the path to the workflow.toml metadata file.
    pub fn workflow_metadata_path(&self) -> PathBuf {
        self.chiral_dir().join("workflow.toml")
    }

    /// Returns the path to the global_params.json file.
    pub fn workflow_params_path(&self) -> PathBuf {
        self.path.join("global_params.json")
    }

    /// Loads workflow metadata from workflow.toml.
    /// Returns None if the file doesn't exist.
    pub fn load_workflow_metadata(&self) -> Result<Option<WorkflowMeta>, JobError> {
        let path = self.workflow_metadata_path();
        if !path.exists() {
            return Ok(None);
        }
        WorkflowMeta::load_from_file(path).map(Some)
    }

    /// Loads workflow parameters from global_params.json.
    /// Returns None if the file doesn't exist.
    pub fn load_workflow_params(&self) -> Result<Option<WorkflowParams>, ParamsError> {
        let path = self.workflow_params_path();
        if !path.exists() {
            return Ok(None);
        }
        load_workflow_params(path).map(Some)
    }

    /// Saves workflow parameters to global_params.json.
    pub fn save_workflow_params(&self, params: &WorkflowParams) -> Result<(), ParamsError> {
        let path = self.workflow_params_path();
        save_workflow_params(path, params)
    }

    /// Saves workflow metadata to workflow.toml.
    /// Creates the .chiral directory if it doesn't exist.
    pub fn save_workflow_metadata(&self, metadata: &WorkflowMeta) -> Result<(), JobError> {
        let chiral_dir = self.chiral_dir();
        if !chiral_dir.exists() {
            fs::create_dir_all(&chiral_dir)?;
        }
        let path = self.workflow_metadata_path();
        metadata.save_to_file(path)
    }
}
