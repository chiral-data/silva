use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use job_config::config::{JobConfig, JobConfigError};

/// Represents a job within a workflow.
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    pub name: String,
    pub path: PathBuf,
    pub config_path: PathBuf,
}

impl Job {
    /// Creates a new Job.
    pub fn new(name: String, path: PathBuf) -> Self {
        let config_path = path.join("@job.toml");
        Self {
            name,
            path,
            config_path,
        }
    }

    /// Checks if the job has a valid configuration file.
    pub fn has_config(&self) -> bool {
        self.config_path.exists() && self.config_path.is_file()
    }

    /// Loads the job configuration.
    pub fn load_config(&self) -> Result<JobConfig, JobConfigError> {
        JobConfig::load_from_file(&self.config_path)
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
    /// A valid job is a subdirectory containing a @job.toml file.
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

                        // Only include if it has a @job.toml file
                        if job.has_config() {
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
    pub fn is_job_folder(path: &Path) -> bool {
        if !path.is_dir() {
            return false;
        }

        let config_path = path.join("@job.toml");
        config_path.exists() && config_path.is_file()
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
