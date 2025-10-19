use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;

use super::home::{WorkflowHome, WorkflowHomeError};

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
}

/// Error types for workflow operations.
#[derive(Debug)]
pub enum WorkflowError {
    HomeError(WorkflowHomeError),
    ScanError(io::Error),
    InvalidWorkflow(String),
}

impl std::fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowError::HomeError(err) => write!(f, "Home directory error: {err}"),
            WorkflowError::ScanError(err) => write!(f, "Scan error: {err}"),
            WorkflowError::InvalidWorkflow(msg) => write!(f, "Invalid workflow: {msg}"),
        }
    }
}

impl std::error::Error for WorkflowError {}

impl From<WorkflowHomeError> for WorkflowError {
    fn from(err: WorkflowHomeError) -> Self {
        WorkflowError::HomeError(err)
    }
}

impl From<io::Error> for WorkflowError {
    fn from(err: io::Error) -> Self {
        WorkflowError::ScanError(err)
    }
}

/// Manages workflow folders in the home directory.
#[derive(Debug)]
pub struct WorkflowManager {
    home: WorkflowHome,
    workflows: Vec<WorkflowFolder>,
    last_scan_error: Option<String>,
}

impl WorkflowManager {
    /// Creates a new WorkflowManager with the given home directory.
    pub fn new(home: WorkflowHome) -> Self {
        Self {
            home,
            workflows: Vec::new(),
            last_scan_error: None,
        }
    }

    /// Initializes the workflow manager by ensuring home exists and scanning.
    pub fn initialize(&mut self) -> Result<(), WorkflowError> {
        self.home.ensure_exists()?;
        self.scan_workflows()?;
        Ok(())
    }

    /// Scans the home directory for workflow folders.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Scan completed successfully
    /// * `Err(WorkflowError)` - Error during scan
    pub fn scan_workflows(&mut self) -> Result<(), WorkflowError> {
        self.last_scan_error = None;
        self.workflows.clear();

        if !self.home.exists() {
            self.last_scan_error = Some("Home directory does not exist".to_string());
            return Ok(());
        }

        let entries = match fs::read_dir(self.home.path()) {
            Ok(entries) => entries,
            Err(e) => {
                self.last_scan_error = Some(format!("Failed to read directory: {e}"));
                return Err(e.into());
            }
        };

        for entry in entries {
            match entry {
                Ok(entry) => {
                    let path = entry.path();

                    // Only include directories
                    if path.is_dir()
                        && let Some(name) = path.file_name()
                    {
                        let name_str = name.to_string_lossy().to_string();

                        // Get creation time
                        let created = entry.metadata().ok().and_then(|m| m.created().ok());

                        let workflow = WorkflowFolder::new(name_str, path, created);
                        self.workflows.push(workflow);
                    }
                }
                Err(e) => {
                    // Log error but continue scanning
                    eprintln!("Error reading entry: {e}");
                }
            }
        }

        // Sort workflows by name
        self.workflows.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(())
    }

    /// Returns a reference to all workflow folders.
    pub fn get_workflows(&self) -> &[WorkflowFolder] {
        &self.workflows
    }

    /// Returns the number of workflow folders.
    pub fn count(&self) -> usize {
        self.workflows.len()
    }

    /// Returns the home directory path.
    pub fn home_path(&self) -> &std::path::Path {
        self.home.path()
    }

    /// Returns the last scan error, if any.
    pub fn last_error(&self) -> Option<&str> {
        self.last_scan_error.as_deref()
    }

    /// Refreshes the workflow list by rescanning the home directory.
    pub fn refresh(&mut self) -> Result<(), WorkflowError> {
        self.scan_workflows()
    }

    /// Creates a new workflow folder with the given name.
    pub fn create_workflow(&mut self, name: &str) -> Result<PathBuf, WorkflowError> {
        if name.is_empty() {
            return Err(WorkflowError::InvalidWorkflow(
                "Workflow name cannot be empty".to_string(),
            ));
        }

        // Sanitize name (remove invalid characters)
        let sanitized = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();

        let workflow_path = self.home.path().join(&sanitized);

        if workflow_path.exists() {
            return Err(WorkflowError::InvalidWorkflow(format!(
                "Workflow '{sanitized}' already exists"
            )));
        }

        fs::create_dir_all(&workflow_path)?;

        // Rescan to update list
        self.scan_workflows()?;

        Ok(workflow_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn setup_test_env() -> (String, WorkflowHome) {
        let test_path = format!("/tmp/silva_workflow_test_{}", std::process::id());
        unsafe { env::set_var("SILVA_HOME_DIR", &test_path) };

        // Clean up if exists
        let _ = fs::remove_dir_all(&test_path);

        let home = WorkflowHome::new().unwrap();
        (test_path, home)
    }

    fn teardown_test_env(test_path: &str) {
        let _ = fs::remove_dir_all(test_path);
        unsafe { env::remove_var("SILVA_HOME_DIR") };
    }

    #[test]
    fn test_workflow_manager_new() {
        let (test_path, home) = setup_test_env();

        let manager = WorkflowManager::new(home);
        assert_eq!(manager.count(), 0);
        assert!(manager.last_error().is_none());

        teardown_test_env(&test_path);
    }

    #[test]
    fn test_workflow_manager_initialize() {
        let (test_path, home) = setup_test_env();

        let mut manager = WorkflowManager::new(home);
        let result = manager.initialize();

        assert!(result.is_ok());
        assert!(PathBuf::from(&test_path).exists());

        teardown_test_env(&test_path);
    }

    #[test]
    fn test_scan_workflows_empty_directory() {
        let (test_path, home) = setup_test_env();

        fs::create_dir_all(&test_path).unwrap();

        let mut manager = WorkflowManager::new(home);
        let result = manager.scan_workflows();

        assert!(result.is_ok());
        assert_eq!(manager.count(), 0);

        teardown_test_env(&test_path);
    }

    #[test]
    fn test_scan_workflows_with_folders() {
        let (test_path, home) = setup_test_env();

        fs::create_dir_all(&test_path).unwrap();
        fs::create_dir_all(format!("{test_path}/workflow_1")).unwrap();
        fs::create_dir_all(format!("{test_path}/workflow_2")).unwrap();
        fs::create_dir_all(format!("{test_path}/workflow_3")).unwrap();

        let mut manager = WorkflowManager::new(home);
        let result = manager.scan_workflows();

        assert!(result.is_ok());
        assert_eq!(manager.count(), 3);

        let workflows = manager.get_workflows();
        assert_eq!(workflows[0].name, "workflow_1");
        assert_eq!(workflows[1].name, "workflow_2");
        assert_eq!(workflows[2].name, "workflow_3");

        teardown_test_env(&test_path);
    }

    #[test]
    fn test_scan_workflows_ignores_files() {
        let (test_path, home) = setup_test_env();

        fs::create_dir_all(&test_path).unwrap();
        fs::create_dir_all(format!("{test_path}/workflow_1")).unwrap();
        fs::write(format!("{test_path}/file.txt"), "test").unwrap();

        let mut manager = WorkflowManager::new(home);
        let result = manager.scan_workflows();

        assert!(result.is_ok());
        assert_eq!(manager.count(), 1);
        assert_eq!(manager.get_workflows()[0].name, "workflow_1");

        teardown_test_env(&test_path);
    }

    #[test]
    fn test_workflow_folder_created_display() {
        let now = SystemTime::now();
        let workflow = WorkflowFolder::new("test".to_string(), PathBuf::from("/test"), Some(now));

        let display = workflow.created_display();
        assert!(display.contains("ago") || display == "0s ago");
    }

    #[test]
    fn test_workflow_folder_created_display_none() {
        let workflow = WorkflowFolder::new("test".to_string(), PathBuf::from("/test"), None);

        assert_eq!(workflow.created_display(), "Unknown");
    }

    #[test]
    fn test_refresh_workflows() {
        let (test_path, home) = setup_test_env();

        fs::create_dir_all(&test_path).unwrap();

        let mut manager = WorkflowManager::new(home);
        manager.scan_workflows().unwrap();
        assert_eq!(manager.count(), 0);

        // Add a workflow
        fs::create_dir_all(format!("{test_path}/new_workflow")).unwrap();

        manager.refresh().unwrap();
        assert_eq!(manager.count(), 1);

        teardown_test_env(&test_path);
    }

    #[test]
    fn test_create_workflow() {
        let (test_path, home) = setup_test_env();

        fs::create_dir_all(&test_path).unwrap();

        let mut manager = WorkflowManager::new(home);
        manager.scan_workflows().unwrap();

        let result = manager.create_workflow("test_workflow");
        assert!(result.is_ok());
        assert_eq!(manager.count(), 1);
        assert!(PathBuf::from(format!("{test_path}/test_workflow")).exists());

        teardown_test_env(&test_path);
    }

    #[test]
    fn test_create_workflow_sanitizes_name() {
        let (test_path, home) = setup_test_env();

        fs::create_dir_all(&test_path).unwrap();

        let mut manager = WorkflowManager::new(home);
        manager.scan_workflows().unwrap();

        let result = manager.create_workflow("test workflow!");
        assert!(result.is_ok());

        let workflows = manager.get_workflows();
        assert_eq!(workflows[0].name, "test_workflow_");

        teardown_test_env(&test_path);
    }

    #[test]
    fn test_create_workflow_duplicate() {
        let (test_path, home) = setup_test_env();

        fs::create_dir_all(&test_path).unwrap();

        let mut manager = WorkflowManager::new(home);
        manager.scan_workflows().unwrap();

        manager.create_workflow("duplicate").unwrap();
        let result = manager.create_workflow("duplicate");

        assert!(result.is_err());

        teardown_test_env(&test_path);
    }

    #[test]
    fn test_create_workflow_empty_name() {
        let (test_path, home) = setup_test_env();

        fs::create_dir_all(&test_path).unwrap();

        let mut manager = WorkflowManager::new(home);
        let result = manager.create_workflow("");

        assert!(result.is_err());

        teardown_test_env(&test_path);
    }
}
