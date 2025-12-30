//! Parameter types and utilities for job and workflow parameters.
//!
//! This module provides JSON-based parameter storage types:
//! - `JobParams`: Parameters for individual jobs (stored in params.json)
//! - `WorkflowParams`: Global parameters for workflows (stored in global_params.json)
//!
//! Note: Parameter *definitions* (ParamDefinition) use TOML and are stored in job.toml,
//! while parameter *values* use JSON for runtime storage.

use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Error type for parameter operations.
#[derive(Debug)]
pub enum ParamsError {
    FileNotFound(String),
    InvalidJson(serde_json::Error),
    SerializeError(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for ParamsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParamsError::FileNotFound(path) => write!(f, "Parameters file not found: {path}"),
            ParamsError::InvalidJson(err) => write!(f, "Invalid JSON syntax: {err}"),
            ParamsError::SerializeError(err) => write!(f, "Serialization error: {err}"),
            ParamsError::IoError(err) => write!(f, "IO error: {err}"),
        }
    }
}

impl std::error::Error for ParamsError {}

impl From<std::io::Error> for ParamsError {
    fn from(err: std::io::Error) -> Self {
        if err.kind() == std::io::ErrorKind::NotFound {
            ParamsError::FileNotFound(err.to_string())
        } else {
            ParamsError::IoError(err)
        }
    }
}

impl From<serde_json::Error> for ParamsError {
    fn from(err: serde_json::Error) -> Self {
        ParamsError::InvalidJson(err)
    }
}

/// Type alias for job parameters (params.json content).
pub type JobParams = HashMap<String, serde_json::Value>;

/// Type alias for workflow parameters (global_params.json content).
pub type WorkflowParams = HashMap<String, serde_json::Value>;

/// Loads job parameters from a JSON file (params.json).
///
/// # Arguments
///
/// * `path` - Path to the params.json file
///
/// # Returns
///
/// * `Ok(JobParams)` - Successfully parsed parameters
/// * `Err(ParamsError)` - Error reading file or parsing JSON
pub fn load_job_params<P: AsRef<Path>>(path: P) -> Result<JobParams, ParamsError> {
    let content = fs::read_to_string(path)?;
    let params: JobParams = serde_json::from_str(&content)?;
    Ok(params)
}

/// Saves job parameters to a JSON file (params.json).
///
/// # Arguments
///
/// * `path` - Path to the params.json file
/// * `params` - Parameters to save
///
/// # Returns
///
/// * `Ok(())` - Successfully saved parameters
/// * `Err(ParamsError)` - Error writing file or serializing JSON
pub fn save_job_params<P: AsRef<Path>>(path: P, params: &JobParams) -> Result<(), ParamsError> {
    let json_str = serde_json::to_string_pretty(params)
        .map_err(|e| ParamsError::SerializeError(e.to_string()))?;
    fs::write(path, json_str)?;
    Ok(())
}

/// Loads workflow parameters from a JSON file (global_params.json).
///
/// # Arguments
///
/// * `path` - Path to the global_params.json file
///
/// # Returns
///
/// * `Ok(WorkflowParams)` - Successfully parsed parameters
/// * `Err(ParamsError)` - Error reading file or parsing JSON
pub fn load_workflow_params<P: AsRef<Path>>(path: P) -> Result<WorkflowParams, ParamsError> {
    let content = fs::read_to_string(path)?;
    let params: WorkflowParams = serde_json::from_str(&content)?;
    Ok(params)
}

/// Saves workflow parameters to a JSON file (global_params.json).
///
/// # Arguments
///
/// * `path` - Path to the global_params.json file
/// * `params` - Parameters to save
///
/// # Returns
///
/// * `Ok(())` - Successfully saved parameters
/// * `Err(ParamsError)` - Error writing file or serializing JSON
pub fn save_workflow_params<P: AsRef<Path>>(
    path: P,
    params: &WorkflowParams,
) -> Result<(), ParamsError> {
    let json_str = serde_json::to_string_pretty(params)
        .map_err(|e| ParamsError::SerializeError(e.to_string()))?;
    fs::write(path, json_str)?;
    Ok(())
}

/// Converts a toml::Value to serde_json::Value.
/// Used when generating default params from ParamDefinition defaults.
pub fn toml_to_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => {
            let map: serde_json::Map<String, serde_json::Value> = table
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
    }
}

/// Converts a serde_json::Value to toml::Value.
/// Used when validating JSON params against TOML-based ParamDefinition.
pub fn json_to_toml(value: &serde_json::Value) -> toml::Value {
    match value {
        serde_json::Value::Null => toml::Value::String("null".to_string()),
        serde_json::Value::Bool(b) => toml::Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                toml::Value::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => toml::Value::String(s.clone()),
        serde_json::Value::Array(arr) => toml::Value::Array(arr.iter().map(json_to_toml).collect()),
        serde_json::Value::Object(obj) => {
            let table: toml::map::Map<String, toml::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_toml(v)))
                .collect();
            toml::Value::Table(table)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_params_json() {
        let json_str = r#"{
            "input_path": "/data/input",
            "batch_size": 32,
            "learning_rate": 0.001
        }"#;

        let params: JobParams = serde_json::from_str(json_str).unwrap();
        assert_eq!(
            params.get("input_path").unwrap().as_str().unwrap(),
            "/data/input"
        );
        assert_eq!(params.get("batch_size").unwrap().as_i64().unwrap(), 32);
    }

    #[test]
    fn test_workflow_params_json() {
        let json_str = r#"{
            "project_name": "ml-pipeline",
            "max_workers": 4
        }"#;

        let params: WorkflowParams = serde_json::from_str(json_str).unwrap();
        assert_eq!(
            params.get("project_name").unwrap().as_str().unwrap(),
            "ml-pipeline"
        );
        assert_eq!(params.get("max_workers").unwrap().as_i64().unwrap(), 4);
    }

    #[test]
    fn test_toml_to_json_conversion() {
        let toml_val = toml::Value::String("test".to_string());
        let json_val = toml_to_json(&toml_val);
        assert_eq!(json_val, serde_json::Value::String("test".to_string()));

        let toml_val = toml::Value::Integer(42);
        let json_val = toml_to_json(&toml_val);
        assert_eq!(json_val, serde_json::json!(42));

        let toml_val = toml::Value::Boolean(true);
        let json_val = toml_to_json(&toml_val);
        assert_eq!(json_val, serde_json::Value::Bool(true));
    }

    #[test]
    fn test_json_to_toml_conversion() {
        let json_val = serde_json::Value::String("test".to_string());
        let toml_val = json_to_toml(&json_val);
        assert_eq!(toml_val, toml::Value::String("test".to_string()));

        let json_val = serde_json::json!(42);
        let toml_val = json_to_toml(&json_val);
        assert_eq!(toml_val, toml::Value::Integer(42));
    }
}
