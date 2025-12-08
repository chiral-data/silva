use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use job_config::job::{JobError as JobConfigError, JobMeta};
use job_config::params::{JobParams, ParamsError, load_job_params, save_job_params};

/// Represents a job folder within a workflow.
#[derive(Debug, Clone, PartialEq)]
pub struct JobFolder {
    pub name: String,
    pub path: PathBuf,
    pub config_path: PathBuf,
    pub chiral_dir: PathBuf,
}

impl JobFolder {
    /// Creates a new JobFolder.
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
    pub fn has_config(&self) -> bool {
        self.config_path.exists() && self.config_path.is_file()
    }

    /// Loads the job metadata (job.toml).
    pub fn load_meta(&self) -> Result<JobMeta, JobConfigError> {
        if self.config_path.exists() {
            JobMeta::load_from_file(&self.config_path)
        } else {
            Err(JobConfigError::FileNotFound(format!(
                "No job configuration found at {}",
                self.config_path.display(),
            )))
        }
    }

    /// Saves the job metadata (job.toml).
    pub fn save_meta(&self, meta: &JobMeta) -> Result<(), JobError> {
        self.ensure_chiral_dir()?;
        meta.save_to_file(&self.config_path)?;
        Ok(())
    }

    /// Ensures the .chiral directory exists.
    pub fn ensure_chiral_dir(&self) -> Result<(), JobError> {
        if !self.chiral_dir.exists() {
            fs::create_dir_all(&self.chiral_dir)?;
        }
        Ok(())
    }

    /// Gets the path to params.json.
    pub fn params_path(&self) -> PathBuf {
        self.path.join("params.json")
    }

    /// Loads job parameters (params.json).
    /// Returns None if the file doesn't exist.
    pub fn load_params(&self) -> Result<Option<JobParams>, JobError> {
        let params_path = self.params_path();
        if !params_path.exists() {
            return Ok(None);
        }

        let params = load_job_params(&params_path)?;
        Ok(Some(params))
    }

    /// Saves job parameters (params.json).
    pub fn save_params(&self, params: &JobParams) -> Result<(), JobError> {
        save_job_params(self.params_path(), params)?;
        Ok(())
    }

    /// Ensures params.toml exists with default values from job.toml.
    pub fn ensure_default_params(&self) -> Result<JobParams, JobError> {
        if let Some(params) = self.load_params()? {
            return Ok(params);
        }

        // Get job metadata
        let meta = self.load_meta()?;

        // Generate default params
        let params = meta.generate_default_params();

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
    ParamsError(ParamsError),
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobError::IoError(err) => write!(f, "IO error: {err}"),
            JobError::InvalidJob(msg) => write!(f, "Invalid job: {msg}"),
            JobError::ConfigError(err) => write!(f, "Configuration error: {err}"),
            JobError::ParamsError(err) => write!(f, "Parameters error: {err}"),
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

impl From<ParamsError> for JobError {
    fn from(err: ParamsError) -> Self {
        JobError::ParamsError(err)
    }
}

/// Scans a workflow directory for job folders.
pub struct JobScanner;

impl JobScanner {
    /// Scans the workflow directory and returns all valid jobs.
    ///
    /// A valid job is a subdirectory containing .chiral/job.toml.
    ///
    /// # Arguments
    ///
    /// * `workflow_path` - Path to the workflow directory
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<JobFolder>)` - List of jobs found, sorted by name
    /// * `Err(JobError)` - Error scanning directory
    pub fn scan_jobs(workflow_path: &Path) -> Result<Vec<JobFolder>, JobError> {
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
                        let job = JobFolder::new(name_str, path);

                        // Only include if it has a config file
                        if job.has_config() {
                            jobs.push(job);
                        }
                    }
                }
                Err(_e) => {
                    // Skip entries that can't be read
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

        // Check for .chiral/job.toml
        let config = path.join(".chiral").join("job.toml");
        config.exists() && config.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
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

    fn create_job_config(job_path: &Path) {
        let chiral_dir = job_path.join(".chiral");
        fs::create_dir_all(&chiral_dir).unwrap();
        fs::write(
            chiral_dir.join("job.toml"),
            r#"name = "Test Job"
description = "A test job"

[container]
image = "ubuntu:22.04"
"#,
        )
        .unwrap();
    }

    #[test]
    #[serial]
    fn test_job_folder_new() {
        let path = PathBuf::from("/tmp/test_job");
        let job = JobFolder::new("test_job".to_string(), path.clone());

        assert_eq!(job.name, "test_job");
        assert_eq!(job.path, path);
        assert_eq!(job.config_path, path.join(".chiral").join("job.toml"));
    }

    #[test]
    #[serial]
    fn test_job_folder_has_config() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();
        let job_path = workflow_path.join("job_1");
        fs::create_dir_all(&job_path).unwrap();

        let job = JobFolder::new("job_1".to_string(), job_path.clone());
        assert!(!job.has_config());

        // Create config file
        create_job_config(&job_path);
        assert!(job.has_config());

        teardown_test_workflow(&test_path);
    }

    #[test]
    #[serial]
    fn test_scan_jobs_empty_workflow() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();

        let result = JobScanner::scan_jobs(&workflow_path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);

        teardown_test_workflow(&test_path);
    }

    #[test]
    #[serial]
    fn test_scan_jobs_with_jobs() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();

        // Create job folders with config files
        for i in 1..=3 {
            let job_path = workflow_path.join(format!("job_{i}"));
            fs::create_dir_all(&job_path).unwrap();
            create_job_config(&job_path);
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
    #[serial]
    fn test_scan_jobs_ignores_folders_without_config() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();

        // Job with config
        let job1_path = workflow_path.join("job_1");
        fs::create_dir_all(&job1_path).unwrap();
        create_job_config(&job1_path);

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
    #[serial]
    fn test_scan_jobs_ignores_files() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();

        // Create a job
        let job_path = workflow_path.join("job_1");
        fs::create_dir_all(&job_path).unwrap();
        create_job_config(&job_path);

        // Create a file
        fs::write(workflow_path.join("readme.txt"), "test").unwrap();

        let result = JobScanner::scan_jobs(&workflow_path);
        assert!(result.is_ok());

        let jobs = result.unwrap();
        assert_eq!(jobs.len(), 1);

        teardown_test_workflow(&test_path);
    }

    #[test]
    #[serial]
    fn test_scan_jobs_nonexistent_path() {
        let workflow_path = PathBuf::from("/nonexistent/path");
        let result = JobScanner::scan_jobs(&workflow_path);

        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_is_job_folder() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();
        let job_path = workflow_path.join("job_1");
        fs::create_dir_all(&job_path).unwrap();

        assert!(!JobScanner::is_job_folder(&job_path));

        create_job_config(&job_path);
        assert!(JobScanner::is_job_folder(&job_path));

        teardown_test_workflow(&test_path);
    }

    #[test]
    #[serial]
    fn test_job_folder_load_meta() {
        let (test_path, workflow_path) = setup_test_workflow();

        fs::create_dir_all(&workflow_path).unwrap();
        let job_path = workflow_path.join("job_1");
        fs::create_dir_all(&job_path).unwrap();

        let chiral_dir = job_path.join(".chiral");
        fs::create_dir_all(&chiral_dir).unwrap();

        let config_content = r#"
name = "Test Job"
description = "A test job"

[container]
image = "ubuntu:22.04"

[scripts]
run = "test.sh"
"#;
        fs::write(chiral_dir.join("job.toml"), config_content).unwrap();

        let job = JobFolder::new("job_1".to_string(), job_path);
        let meta = job.load_meta();

        assert!(meta.is_ok());
        let meta = meta.unwrap();
        assert_eq!(meta.name, "Test Job");
        assert_eq!(meta.scripts.run, "test.sh");

        teardown_test_workflow(&test_path);
    }
}
