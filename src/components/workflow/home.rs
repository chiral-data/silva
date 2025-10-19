use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const DEFAULT_HOME_DIR: &str = "./home";
const ENV_VAR_NAME: &str = "SILVA_HOME_DIR";

/// Error types for workflow home operations.
#[derive(Debug)]
pub enum WorkflowHomeError {
    InvalidPath(String),
    NotADirectory(PathBuf),
    PermissionDenied(PathBuf),
    IoError(io::Error),
}

impl std::fmt::Display for WorkflowHomeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowHomeError::InvalidPath(msg) => write!(f, "Invalid path: {msg}"),
            WorkflowHomeError::NotADirectory(path) => {
                write!(f, "Path is not a directory: {}", path.display())
            }
            WorkflowHomeError::PermissionDenied(path) => {
                write!(f, "Permission denied: {}", path.display())
            }
            WorkflowHomeError::IoError(err) => write!(f, "IO error: {err}"),
        }
    }
}

impl std::error::Error for WorkflowHomeError {}

impl From<io::Error> for WorkflowHomeError {
    fn from(err: io::Error) -> Self {
        WorkflowHomeError::IoError(err)
    }
}

/// Represents the home directory for workflows.
/// The path is resolved from SILVA_HOME_DIR environment variable,
/// or defaults to "./home" if not set.
#[derive(Debug, Clone)]
pub struct WorkflowHome {
    path: PathBuf,
}

impl WorkflowHome {
    /// Creates a new WorkflowHome by resolving the home directory path.
    /// Checks SILVA_HOME_DIR environment variable first, falls back to "./home".
    ///
    /// # Returns
    ///
    /// * `Ok(WorkflowHome)` - Successfully resolved home directory
    /// * `Err(WorkflowHomeError)` - Error resolving or validating path
    pub fn new() -> Result<Self, WorkflowHomeError> {
        let path = Self::resolve_home_path();
        Ok(Self { path })
    }

    /// Resolves the home directory path from environment or default.
    fn resolve_home_path() -> PathBuf {
        env::var(ENV_VAR_NAME)
            .ok()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_HOME_DIR))
    }

    /// Returns the home directory path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Ensures the home directory exists, creating it if necessary.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Directory exists or was created successfully
    /// * `Err(WorkflowHomeError)` - Error creating directory or validating
    pub fn ensure_exists(&self) -> Result<(), WorkflowHomeError> {
        if self.path.exists() {
            // Verify it's a directory
            if !self.path.is_dir() {
                return Err(WorkflowHomeError::NotADirectory(self.path.clone()));
            }

            // Check if we can read the directory
            fs::read_dir(&self.path).map_err(|e| {
                if e.kind() == io::ErrorKind::PermissionDenied {
                    WorkflowHomeError::PermissionDenied(self.path.clone())
                } else {
                    WorkflowHomeError::IoError(e)
                }
            })?;
        } else {
            // Create the directory
            fs::create_dir_all(&self.path)?;
        }

        Ok(())
    }

    /// Checks if the home directory exists.
    pub fn exists(&self) -> bool {
        self.path.exists() && self.path.is_dir()
    }

    /// Returns the absolute path of the home directory.
    pub fn absolute_path(&self) -> Result<PathBuf, WorkflowHomeError> {
        self.path.canonicalize().map_err(WorkflowHomeError::IoError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_default_home_path() {
        // Clear env var to test default
        unsafe { env::remove_var(ENV_VAR_NAME) };

        let home = WorkflowHome::new().unwrap();
        assert_eq!(home.path(), Path::new(DEFAULT_HOME_DIR));
    }

    #[test]
    fn test_home_path_from_env() {
        let test_path = "/tmp/test_silva_home";
        unsafe { env::set_var(ENV_VAR_NAME, test_path) };

        let home = WorkflowHome::new().unwrap();
        assert_eq!(home.path(), Path::new(test_path));

        unsafe { env::remove_var(ENV_VAR_NAME) };
    }

    #[test]
    fn test_ensure_exists_creates_directory() {
        let test_path = format!("/tmp/silva_test_home_create_{}", std::process::id());
        unsafe { env::set_var(ENV_VAR_NAME, &test_path) };

        // Clean up if exists
        let _ = fs::remove_dir_all(&test_path);

        let home = WorkflowHome::new().unwrap();
        assert!(!home.exists());

        home.ensure_exists().unwrap();
        assert!(home.exists());

        // Clean up
        let _ = fs::remove_dir_all(&test_path);
        unsafe { env::remove_var(ENV_VAR_NAME) };
    }

    #[test]
    fn test_ensure_exists_validates_existing_directory() {
        let test_path = "/tmp/silva_test_home_existing";
        unsafe { env::set_var(ENV_VAR_NAME, test_path) };

        // Create directory first
        fs::create_dir_all(test_path).unwrap();

        let home = WorkflowHome::new().unwrap();
        assert!(home.exists());

        let result = home.ensure_exists();
        assert!(result.is_ok());

        // Clean up
        fs::remove_dir_all(test_path).unwrap();
        unsafe { env::remove_var(ENV_VAR_NAME) };
    }

    #[test]
    fn test_not_a_directory_error() {
        let test_file = format!("/tmp/silva_test_file_{}", std::process::id());
        unsafe { env::set_var(ENV_VAR_NAME, &test_file) };

        // Clean up if exists as directory
        let _ = fs::remove_dir_all(&test_file);

        // Create a file, not a directory
        fs::write(&test_file, "test").unwrap();

        let home = WorkflowHome::new().unwrap();
        let result = home.ensure_exists();

        assert!(result.is_err());
        match result.unwrap_err() {
            WorkflowHomeError::NotADirectory(_) => {}
            _ => panic!("Expected NotADirectory error"),
        }

        // Clean up
        let _ = fs::remove_file(&test_file);
        unsafe { env::remove_var(ENV_VAR_NAME) };
    }

    #[test]
    fn test_exists_returns_false_for_nonexistent() {
        let test_path = "/tmp/silva_nonexistent_dir";
        unsafe { env::set_var(ENV_VAR_NAME, test_path) };

        // Ensure it doesn't exist
        let _ = fs::remove_dir_all(test_path);

        let home = WorkflowHome::new().unwrap();
        assert!(!home.exists());

        unsafe { env::remove_var(ENV_VAR_NAME) };
    }

    #[test]
    fn test_absolute_path() {
        let test_path = format!("/tmp/silva_test_absolute_{}", std::process::id());
        unsafe { env::set_var(ENV_VAR_NAME, &test_path) };

        fs::create_dir_all(&test_path).unwrap();

        let home = WorkflowHome::new().unwrap();
        let abs_path = home.absolute_path().unwrap();

        assert!(abs_path.is_absolute());

        // Clean up
        let _ = fs::remove_dir_all(&test_path);
        unsafe { env::remove_var(ENV_VAR_NAME) };
    }
}
