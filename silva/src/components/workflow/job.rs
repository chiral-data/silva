use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use job_config::config::{JobConfig, JobConfigError, NodeMetadata, JobParams, load_params, save_params};

/// Represents a job within a workflow.
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub name: String,
    pub path: PathBuf,
    pub config_path: PathBuf,
    pub chiral_dir: PathBuf,
}

impl Job {
    /// Creates a new Job.
    pub fn new(name: String, path: PathBuf) -> Self {
        let chiral_dir = path.join(".chiral");
        let config_path = chiral_dir.join("job.toml");
        Self {
            name,
            path,
            config_path,
            chiral_dir,
        }
    }

    /// Checks if the job has a valid configuration file.
    /// Also checks for legacy @job.toml for backward compatibility.
    pub fn has_config(&self) -> bool {
        // Check new location
        if self.config_path.exists() && self.config_path.is_file() {
            return true;
        }

        // Check legacy location
        let legacy_config = self.path.join("@job.toml");
        legacy_config.exists() && legacy_config.is_file()
    }

    /// Loads the job configuration.
    /// Attempts to load from .chiral/job.toml first, falls back to @job.toml if not found.
    pub fn load_config(&self) -> Result<JobConfig, JobConfigError> {
        if self.config_path.exists() {
            JobConfig::load_from_file(&self.config_path)
        } else {
            // Try legacy path
            let legacy_config = self.path.join("@job.toml");
            if legacy_config.exists() {
                JobConfig::load_from_file(&legacy_config)
            } else {
                Err(JobConfigError::FileNotFound(format!(
                    "No job configuration found at {} or {}",
                    self.config_path.display(),
                    legacy_config.display()
                )))
            }
        }
    }

    /// Ensures the .chiral directory exists.
    pub fn ensure_chiral_dir(&self) -> Result<(), JobError> {
        if !self.chiral_dir.exists() {
            fs::create_dir_all(&self.chiral_dir)?;
        }
        Ok(())
    }

    /// Migrates the job configuration from @job.toml to .chiral/job.toml.
    /// Returns true if migration was performed, false if already migrated or no legacy config exists.
    pub fn migrate_to_chiral(&self) -> Result<bool, JobError> {
        let legacy_config = self.path.join("@job.toml");

        // If new config already exists, no need to migrate
        if self.config_path.exists() {
            return Ok(false);
        }

        // If legacy config doesn't exist, nothing to migrate
        if !legacy_config.exists() {
            return Ok(false);
        }

        // Create .chiral directory
        self.ensure_chiral_dir()?;

        // Copy the config file
        fs::copy(&legacy_config, &self.config_path)?;

        // Remove legacy file
        fs::remove_file(&legacy_config)?;

        Ok(true)
    }

    /// Gets the path to node.json.
    pub fn node_metadata_path(&self) -> PathBuf {
        self.chiral_dir.join("node.json")
    }

    /// Gets the path to params.json.
    pub fn params_path(&self) -> PathBuf {
        self.chiral_dir.join("params.json")
    }

    /// Loads node metadata (node.json).
    /// Returns None if the file doesn't exist.
    pub fn load_node_metadata(&self) -> Result<Option<NodeMetadata>, JobError> {
        let node_path = self.node_metadata_path();
        if !node_path.exists() {
            return Ok(None);
        }

        let metadata = NodeMetadata::load_from_file(&node_path)?;
        Ok(Some(metadata))
    }

    /// Saves node metadata (node.json).
    pub fn save_node_metadata(&self, metadata: &NodeMetadata) -> Result<(), JobError> {
        self.ensure_chiral_dir()?;
        metadata.save_to_file(&self.node_metadata_path())?;
        Ok(())
    }

    /// Loads job parameters (params.json).
    /// Returns None if the file doesn't exist.
    pub fn load_params(&self) -> Result<Option<JobParams>, JobError> {
        let params_path = self.params_path();
        if !params_path.exists() {
            return Ok(None);
        }

        let params = load_params(&params_path)?;
        Ok(Some(params))
    }

    /// Saves job parameters (params.json).
    pub fn save_params(&self, params: &JobParams) -> Result<(), JobError> {
        self.ensure_chiral_dir()?;
        save_params(&self.params_path(), params)?;
        Ok(())
    }

    /// Generates default node metadata if it doesn't exist.
    pub fn ensure_default_node_metadata(&self) -> Result<NodeMetadata, JobError> {
        if let Some(metadata) = self.load_node_metadata()? {
            return Ok(metadata);
        }

        // Create default metadata
        let metadata = NodeMetadata::new(
            self.name.clone(),
            format!("Job: {}", self.name),
        );

        self.save_node_metadata(&metadata)?;
        Ok(metadata)
    }

    /// Ensures params.json exists with default values from node.json.
    pub fn ensure_default_params(&self) -> Result<JobParams, JobError> {
        if let Some(params) = self.load_params()? {
            return Ok(params);
        }

        // Get or create node metadata
        let metadata = self.ensure_default_node_metadata()?;

        // Generate default params
        let params = metadata.generate_default_params();

        self.save_params(&params)?;
        Ok(params)
    }
}

/// Error types for job operations.
#[derive(Debug)]
pub enum JobError {
    IoError(io::Error),
    InvalidJob(String),
    ConfigError(JobConfigError),
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobError::IoError(err) => write!(f, "IO error: {err}"),
            JobError::InvalidJob(msg) => write!(f, "Invalid job: {msg}"),
            JobError::ConfigError(err) => write!(f, "Configuration error: {err}"),
        }
    }
}

impl std::error::Error for JobError {}

impl From<io::Error> for JobError {
    fn from(err: io::Error) -> Self {
        JobError::IoError(err)
    }
}

impl From<JobConfigError> for JobError {
    fn from(err: JobConfigError) -> Self {
        JobError::ConfigError(err)
    }
}

/// Scans a workflow directory for job folders.
pub struct JobScanner;

impl JobScanner {
    /// Scans the workflow directory and returns all valid jobs.
    ///
    /// A valid job is a subdirectory containing either .chiral/job.toml or @job.toml (legacy).
    /// Automatically migrates legacy @job.toml to .chiral/job.toml when found.
    ///
    /// # Arguments
    ///
    /// * `workflow_path` - Path to the workflow directory
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<Job>)` - List of jobs found, sorted by name
    /// * `Err(JobError)` - Error scanning directory
    pub fn scan_jobs(workflow_path: &Path) -> Result<Vec<Job>, JobError> {
        if !workflow_path.exists() {
            return Err(JobError::InvalidJob(format!(
                "Workflow path does not exist: {}",
                workflow_path.display()
            )));
        }

        if !workflow_path.is_dir() {
            return Err(JobError::InvalidJob(format!(
                "Workflow path is not a directory: {}",
                workflow_path.display()
            )));
        }

        let mut jobs = Vec::new();

        let entries = fs::read_dir(workflow_path)?;

        for entry in entries {
            match entry {
                Ok(entry) => {
                    let path = entry.path().canonicalize().unwrap();

                    // Only consider directories
                    if path.is_dir()
                        && let Some(name) = path.file_name()
                    {
                        let name_str = name.to_string_lossy().to_string();
                        let job = Job::new(name_str, path);

                        // Only include if it has a config file
                        if job.has_config() {
                            // Automatically migrate legacy configs
                            let _ = job.migrate_to_chiral();
                            jobs.push(job);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading entry: {e}");
                }
            }
        }

        // Sort jobs by name for consistent execution order
        jobs.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(jobs)
    }

    /// Checks if a path is a valid job folder.
    /// Checks for both .chiral/job.toml and @job.toml (legacy).
    pub fn is_job_folder(path: &Path) -> bool {
        if !path.is_dir() {
            return false;
        }

        // Check new location
        let new_config = path.join(".chiral").join("job.toml");
        if new_config.exists() && new_config.is_file() {
            return true;
        }

        // Check legacy location
        let legacy_config = path.join("@job.toml");
        legacy_config.exists() && legacy_config.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_workflow() -> (String, PathBuf) {
        let test_path = format!("/tmp/silva_job_test_{}", std::process::id());
        let workflow_path = PathBuf::from(&test_path);

        // Clean up if exists
        let _ = fs::remove_dir_all(&test_path);

        (test_path, workflow_path)
    }

    fn teardown_test_workflow(test_path: &str) {
        let _ = fs::remove_dir_all(test_path);
    }

    #[test]
    fn test_job_new() {
        let path = PathBuf::from("/tmp/test_job");
        let job = Job::new("test_job".to_string(), path.clone());

        assert_eq!(job.name, "test_job");
        assert_eq!(job.path, path);
        assert_eq!(job.config_path, path.join("@job.toml"));
    }

    #[test]
    fn test_job_has_config() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();
        let job_path = workflow_path.join("job_1");
        fs::create_dir_all(&job_path).unwrap();

        let job = Job::new("job_1".to_string(), job_path.clone());
        assert!(!job.has_config());

        // Create config file
        fs::write(
            job_path.join("@job.toml"),
            "[container]\ndocker_image = \"ubuntu:22.04\"",
        )
        .unwrap();
        assert!(job.has_config());

        teardown_test_workflow(&test_path);
    }

    #[test]
    fn test_scan_jobs_empty_workflow() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();

        let result = JobScanner::scan_jobs(&workflow_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);

        teardown_test_workflow(&test_path);
    }

    #[test]
    fn test_scan_jobs_with_jobs() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();

        // Create job folders with config files
        for i in 1..=3 {
            let job_path = workflow_path.join(format!("job_{i}"));
            fs::create_dir_all(&job_path).unwrap();
            fs::write(
                job_path.join("@job.toml"),
                "[container]\ndocker_image = \"ubuntu:22.04\"",
            )
            .unwrap();
        }

        let result = JobScanner::scan_jobs(&workflow_path);
        assert!(result.is_ok());

        let jobs = result.unwrap();
        assert_eq!(jobs.len(), 3);
        assert_eq!(jobs[0].name, "job_1");
        assert_eq!(jobs[1].name, "job_2");
        assert_eq!(jobs[2].name, "job_3");

        teardown_test_workflow(&test_path);
    }

    #[test]
    fn test_scan_jobs_ignores_folders_without_config() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();

        // Job with config
        let job1_path = workflow_path.join("job_1");
        fs::create_dir_all(&job1_path).unwrap();
        fs::write(
            job1_path.join("@job.toml"),
            "[container]\ndocker_image = \"ubuntu:22.04\"",
        )
        .unwrap();

        // Folder without config
        let job2_path = workflow_path.join("not_a_job");
        fs::create_dir_all(&job2_path).unwrap();

        let result = JobScanner::scan_jobs(&workflow_path);
        assert!(result.is_ok());

        let jobs = result.unwrap();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "job_1");

        teardown_test_workflow(&test_path);
    }

    #[test]
    fn test_scan_jobs_ignores_files() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();

        // Create a job
        let job_path = workflow_path.join("job_1");
        fs::create_dir_all(&job_path).unwrap();
        fs::write(
            job_path.join("@job.toml"),
            "[container]\ndocker_image = \"ubuntu:22.04\"",
        )
        .unwrap();

        // Create a file
        fs::write(workflow_path.join("readme.txt"), "test").unwrap();

        let result = JobScanner::scan_jobs(&workflow_path);
        assert!(result.is_ok());

        let jobs = result.unwrap();
        assert_eq!(jobs.len(), 1);

        teardown_test_workflow(&test_path);
    }

    #[test]
    fn test_scan_jobs_nonexistent_path() {
        let workflow_path = PathBuf::from("/nonexistent/path");
        let result = JobScanner::scan_jobs(&workflow_path);

        assert!(result.is_err());
    }

    #[test]
    fn test_is_job_folder() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();
        let job_path = workflow_path.join("job_1");
        fs::create_dir_all(&job_path).unwrap();

        assert!(!JobScanner::is_job_folder(&job_path));

        fs::write(
            job_path.join("@job.toml"),
            "[container]\ndocker_image = \"ubuntu:22.04\"",
        )
        .unwrap();
        assert!(JobScanner::is_job_folder(&job_path));

        teardown_test_workflow(&test_path);
    }

    #[test]
    fn test_job_load_config() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();
        let job_path = workflow_path.join("job_1");
        fs::create_dir_all(&job_path).unwrap();

        let config_content = r#"
[container]
docker_image = "ubuntu:22.04"

[scripts]
run = "test.sh"
"#;
        fs::write(job_path.join("@job.toml"), config_content).unwrap();

        let job = Job::new("job_1".to_string(), job_path);
        let config = job.load_config();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.scripts.run, "test.sh");

        teardown_test_workflow(&test_path);
    }
}
