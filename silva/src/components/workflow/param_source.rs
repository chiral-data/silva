//! Parameter source abstraction for the parameter editor.
//!
//! This module provides a trait-based abstraction over different parameter sources,
//! allowing the same editor UI to be used for both job-level and workflow-level parameters.

use std::collections::HashMap;

use job_config::job::{JobMeta, ParamDefinition};
use job_config::params::JobParams;
use job_config::workflow::WorkflowMeta;

use super::job_folder::JobFolder;
use super::workflow_folder::WorkflowFolder;

/// Trait for types that can provide parameters for editing.
/// This abstracts over JobFolder (job-level params) and WorkflowFolder (global params).
pub trait ParamSource: Clone {
    /// Returns the display name for the editor title.
    fn display_name(&self) -> &str;

    /// Returns the description text.
    fn description(&self) -> &str;

    /// Returns the parameter definitions.
    fn param_definitions(&self) -> &HashMap<String, ParamDefinition>;

    /// Loads current parameter values.
    /// Returns None if no params file exists yet.
    fn load_params(&self) -> Result<Option<JobParams>, String>;

    /// Saves parameter values.
    fn save_params(&self, params: &JobParams) -> Result<(), String>;

    /// Generates default parameter values from definitions.
    fn generate_default_params(&self) -> JobParams;

    /// Returns true if this is a global/workflow-level editor.
    fn is_global(&self) -> bool;
}

/// Wrapper for JobFolder with its metadata for parameter editing.
#[derive(Debug, Clone)]
pub struct JobParamSource {
    pub job: JobFolder,
    pub meta: JobMeta,
}

impl JobParamSource {
    pub fn new(job: JobFolder, meta: JobMeta) -> Self {
        Self { job, meta }
    }
}

impl ParamSource for JobParamSource {
    fn display_name(&self) -> &str {
        &self.job.name
    }

    fn description(&self) -> &str {
        &self.meta.description
    }

    fn param_definitions(&self) -> &HashMap<String, ParamDefinition> {
        &self.meta.params
    }

    fn load_params(&self) -> Result<Option<JobParams>, String> {
        self.job
            .load_params()
            .map_err(|e| format!("Failed to load params: {e}"))
    }

    fn save_params(&self, params: &JobParams) -> Result<(), String> {
        self.job
            .save_params(params)
            .map_err(|e| format!("Failed to save params: {e}"))
    }

    fn generate_default_params(&self) -> JobParams {
        self.meta.generate_default_params()
    }

    fn is_global(&self) -> bool {
        false
    }
}

/// Wrapper for WorkflowFolder with its metadata for parameter editing.
#[derive(Debug, Clone)]
pub struct WorkflowParamSource {
    pub workflow: WorkflowFolder,
    pub meta: WorkflowMeta,
}

impl WorkflowParamSource {
    pub fn new(workflow: WorkflowFolder, meta: WorkflowMeta) -> Self {
        Self { workflow, meta }
    }
}

impl ParamSource for WorkflowParamSource {
    fn display_name(&self) -> &str {
        &self.workflow.name
    }

    fn description(&self) -> &str {
        &self.meta.description
    }

    fn param_definitions(&self) -> &HashMap<String, ParamDefinition> {
        &self.meta.params
    }

    fn load_params(&self) -> Result<Option<JobParams>, String> {
        self.workflow
            .load_workflow_params()
            .map_err(|e| format!("Failed to load global params: {e}"))
    }

    fn save_params(&self, params: &JobParams) -> Result<(), String> {
        self.workflow
            .save_workflow_params(params)
            .map_err(|e| format!("Failed to save global params: {e}"))
    }

    fn generate_default_params(&self) -> JobParams {
        self.meta.generate_default_params()
    }

    fn is_global(&self) -> bool {
        true
    }
}
