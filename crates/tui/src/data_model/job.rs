//! Jobs run locally to manage cloud infrastructure
//!

use std::{collections::{HashMap, VecDeque}, fs, path::{Path, PathBuf}, time::{Duration, SystemTime}};

use serde::{Deserialize, Serialize};

use crate::{constants, utils};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum JobStatus {
    Created,
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "Created"),
            Self::Queued => write!(f, "Queued"),
            Self::Running => write!(f, "Running"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Cancelled => write!(f, "Cancelled"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum Infra {
    None,
    Local,
    // (task id, http uri)
    SakuraInternetDOK(String, Option<String>)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Job {
    pub id: usize,
    pub status: JobStatus,
    pub infra: Infra,
    pub project_path: Option<String>,
    pub config_index: Option<usize>, // Which @job_X.toml this job uses
    pub created_at: SystemTime,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub duration: Option<Duration>,
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
    pub resource_usage: Option<ResourceUsage>,
    // Advanced features
    #[serde(default)]
    pub dependencies: Vec<usize>, // Job IDs this job depends on
    #[serde(default)]
    pub scheduled_at: Option<SystemTime>, // When to run the job
    #[serde(default)]
    pub recurring: Option<RecurringSchedule>, // For recurring jobs
    #[serde(default)]
    pub priority: i32, // Job priority (higher = more important)
    #[serde(default)]
    pub resource_limits: Option<ResourceLimits>, // Resource constraints
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ResourceUsage {
    pub cpu_usage: Option<f64>,
    pub memory_usage: Option<u64>,
    pub disk_usage: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct RecurringSchedule {
    pub interval: RecurringInterval,
    pub next_run: SystemTime,
    pub end_date: Option<SystemTime>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum RecurringInterval {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Custom(Duration),
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ResourceLimits {
    pub max_cpu_percent: Option<f64>,
    pub max_memory_mb: Option<u64>,
    pub max_runtime_seconds: Option<u64>,
    pub max_disk_mb: Option<u64>,
}

impl std::fmt::Display for Job {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let duration_str = match &self.duration {
            Some(d) => format!("{}s", d.as_secs()),
            None => "--".to_string(),
        };
        
        let config_str = match self.config_index {
            Some(0) => "@job.toml".to_string(),
            Some(index) => format!("@job_{}.toml", index + 1),
            None => "--".to_string(),
        };
        
        let project_name = self.project_path.as_deref()
            .and_then(|p| std::path::Path::new(p).file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("--");
        
        write!(f, "{:5} {:10} {:12} {:8} {}", self.id, self.status, config_str, duration_str, project_name)
    }
}

impl Job {
    pub fn new(id: usize) -> Self {
        Self { 
            id, 
            status: JobStatus::Created,
            infra: Infra::None,
            project_path: None,
            config_index: None,
            created_at: SystemTime::now(),
            started_at: None,
            completed_at: None,
            duration: None,
            exit_code: None,
            error_message: None,
            resource_usage: None,
            dependencies: Vec::new(),
            scheduled_at: None,
            recurring: None,
            priority: 0,
            resource_limits: None,
        }
    }

    pub fn with_project_path(mut self, project_path: String) -> Self {
        self.project_path = Some(project_path);
        self
    }

    pub fn with_config_index(mut self, config_index: usize) -> Self {
        self.config_index = Some(config_index);
        self
    }

    pub fn set_status(&mut self, status: JobStatus) {
        self.status = status.clone();
        match status {
            JobStatus::Running => {
                self.started_at = Some(SystemTime::now());
            }
            JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled => {
                self.completed_at = Some(SystemTime::now());
                if let Some(started) = self.started_at {
                    self.duration = SystemTime::now().duration_since(started).ok();
                }
            }
            _ => {}
        }
    }

    pub fn set_exit_code(&mut self, exit_code: i32) {
        self.exit_code = Some(exit_code);
        if exit_code == 0 {
            self.set_status(JobStatus::Completed);
        } else {
            self.set_status(JobStatus::Failed);
        }
    }

    pub fn set_error(&mut self, error: String) {
        self.error_message = Some(error);
        self.set_status(JobStatus::Failed);
    }

    pub fn set_resource_usage(&mut self, usage: ResourceUsage) {
        self.resource_usage = Some(usage);
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, JobStatus::Running)
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.status, JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled)
    }

    pub fn can_run(&self) -> bool {
        matches!(self.status, JobStatus::Created | JobStatus::Queued | JobStatus::Failed)
    }

    pub fn add_dependency(&mut self, job_id: usize) {
        if !self.dependencies.contains(&job_id) {
            self.dependencies.push(job_id);
        }
    }

    pub fn remove_dependency(&mut self, job_id: usize) {
        self.dependencies.retain(|&id| id != job_id);
    }

    pub fn has_dependencies(&self) -> bool {
        !self.dependencies.is_empty()
    }

    pub fn set_schedule(&mut self, time: SystemTime) {
        self.scheduled_at = Some(time);
    }

    pub fn set_recurring(&mut self, interval: RecurringInterval, end_date: Option<SystemTime>) {
        self.recurring = Some(RecurringSchedule {
            interval,
            next_run: SystemTime::now(),
            end_date,
        });
    }

    pub fn should_run_now(&self) -> bool {
        match self.scheduled_at {
            Some(scheduled) => SystemTime::now() >= scheduled,
            None => true,
        }
    }

    // TODO: to deprecate
    // pub fn get_settings(proj_dir: &Path) -> anyhow::Result<settings::Settings> {
    //     let settings_filepath = proj_dir.join("@job.toml");
    //     let content = std::fs::read_to_string(&settings_filepath)
    //         .map_err(|e| anyhow::Error::msg(format!("{e}: no settings file {settings_filepath:?}")))?;
    //     let job_settings = settings::Settings::new(&content)
    //         .map_err(|e| anyhow::Error::msg(format!("{e}: incorrect settings in {settings_filepath:?}")))?;

    //     Ok(job_settings)
    // }

    pub fn get_settings_vec(proj_dir: &Path) -> anyhow::Result<Vec<settings::Settings>> {
        let mut job_sv = vec![];
        
        // First check for @job.toml (index 0)
        let single_filepath = proj_dir.join("@job.toml");
        if single_filepath.exists() {
            let content = std::fs::read_to_string(&single_filepath)
                .map_err(|e| anyhow::Error::msg(format!("{e}: no settings file {single_filepath:?}")))?;
            let job_s = settings::Settings::new(&content)
                .map_err(|e| anyhow::Error::msg(format!("{e}: incorrect settings in {single_filepath:?}")))?;
            job_sv.push(job_s);
        }
        
        // Then check for @job_1.toml, @job_2.toml, etc. (indices 1, 2, ...)
        let mut index = 1;
        loop {
            let multi_filepath = proj_dir.join(format!("@job_{}.toml", index));
            match std::fs::read_to_string(&multi_filepath) {
                Ok(content) => {
                    let job_s = settings::Settings::new(&content)
                        .map_err(|e| anyhow::Error::msg(format!("{e}: incorrect settings in {multi_filepath:?}")))?;
                    job_sv.push(job_s);
                    index += 1;
                }
                Err(_e) => break
            }
        }

        if job_sv.is_empty() {
            Err(anyhow::Error::msg("no job settings file, single settings with @job.toml and multiple settings with @job_i.toml"))
        } else {
            Ok(job_sv)
        }
    }
}

#[cfg(test)]
fn test_job_settings() {
    let toml_str = r#"
        [files]
        inputs = [
            "a.in",
            "b.in",
            "c.in"
        ]
        outputs = [
            "1.out",
        ]
        scripts = [
            "start.sh",
            "finish.sh"
        ]

        [dok]
        base_image = "a"
        extra_build_commands = ["python load_model.py"]
        http_path = "/dok"
        http_port = 11203
        plan = "v100-32gb"
        "#;

    let temp_dir = std::env::temp_dir();
    let test_dir = temp_dir.join("silva_test_job_settings");
    std::fs::create_dir_all(&test_dir).unwrap();

    // test case 1: no job settings file 
    let proj_dir_1 = test_dir.join("proj_1");
    std::fs::create_dir_all(&proj_dir_1).unwrap();
    let result_err =Job::get_settings_vec(&proj_dir_1);
    assert!(result_err.is_err());

    // test case 2: one file @job.toml under the project folder
    let proj_dir_2 = test_dir.join("proj_2");
    std::fs::create_dir_all(&proj_dir_2).unwrap();
    std::fs::write(proj_dir_2.join("@job.toml"), toml_str).unwrap();
    let job_sv_2 =Job::get_settings_vec(&proj_dir_2).unwrap();
    assert!(job_sv_2.len() == 1);

    // test case 2: 3 files @job_1.toml, @job_2.toml, @job_3.toml under the project folder
    let proj_dir_3 = test_dir.join("proj_3");
    std::fs::create_dir_all(&proj_dir_3).unwrap();
    std::fs::write(proj_dir_3.join("@job_1.toml"), toml_str).unwrap();
    std::fs::write(proj_dir_3.join("@job_2.toml"), toml_str).unwrap();
    std::fs::write(proj_dir_3.join("@job_3.toml"), toml_str).unwrap();
    let job_sv_3 =Job::get_settings_vec(&proj_dir_3).unwrap();
    assert!(job_sv_3.len() == 3);

    // clean
    std::fs::remove_dir_all(&test_dir).unwrap();
}


#[derive(Debug, Deserialize, Serialize)]
struct DataFile {
    jobs: Option<Vec<Job>>,
}

impl DataFile {
    fn new(content: &str) -> anyhow::Result<Self> {
        let df: Self = toml::from_str(content)?;
        Ok(df)
    }
}

pub struct Manager {
    pub jobs: HashMap<usize, Job>,
    /// job logs: <job id, log contents>
    pub chat_stream: String,
    pub logs: HashMap<usize, VecDeque<String>>,
    pub logs_tmp: HashMap<usize, String>,
    pub local_infra_cancel_job: bool,
    pub next_job_id: usize,
    /// Job queue for managing execution order
    pub job_queue: VecDeque<usize>,
    /// Maximum concurrent jobs (0 = unlimited)
    pub max_concurrent_jobs: usize,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
            chat_stream: String::new(),
            logs: HashMap::new(),
            logs_tmp: HashMap::new(),
            local_infra_cancel_job: false,
            next_job_id: 1,
            job_queue: VecDeque::new(),
            max_concurrent_jobs: 3, // Allow up to 3 concurrent jobs by default
        }
    }

    fn data_filepath() -> anyhow::Result<PathBuf> {
        let data_dir = utils::dirs::data_dir();
        let fp = data_dir.join(constants::FILENAME_JOBS);
        Ok(fp)
    }

    pub fn load() -> anyhow::Result<Self> {
        let filepath = Self::data_filepath()?; 
        if !filepath.exists() {
            std::fs::File::create(&filepath)?;
        }

        let content = fs::read_to_string(&filepath)?;
        let df = if content.trim().is_empty() {
            DataFile { jobs: None }
        } else {
            DataFile::new(&content)?
        };
        
        let jobs = match df.jobs {
            Some(jobs) => jobs.into_iter()
                .map(|job| (job.id, job))
                .collect(),
            None => HashMap::new() 
        };

        let next_job_id = jobs.keys().max().unwrap_or(&0) + 1;
        let chat_stream = String::new();
        let s = Self { 
            jobs, chat_stream, 
            logs: HashMap::new(), logs_tmp: HashMap::new(),
            local_infra_cancel_job: false,
            next_job_id,
            job_queue: VecDeque::new(),
            max_concurrent_jobs: 3,
        };
        Ok(s)
    }

    /// add a new log message for job 
    pub fn add_log(&mut self, job_id: usize, log: String) {
        let job_logs = self.logs.entry(job_id).or_default();
        job_logs.push_back(log);
        if job_logs.len() > 20 {
            job_logs.pop_front();
        }
    }

    pub fn add_log_tmp(&mut self, job_id: usize, log: String) {
        let _ = self.logs_tmp.insert(job_id, log);
    }

    pub fn clear_log_tmp(&mut self, job_id: &usize) {
        let _ = self.logs_tmp.remove(job_id);
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let filepath = Self::data_filepath()?;
        let jobs: Vec<Job> = self.jobs.values().cloned().collect();
        let data_file = DataFile {
            jobs: if jobs.is_empty() { None } else { Some(jobs) },
        };
        
        let content = toml::to_string(&data_file)
            .map_err(|e| anyhow::Error::msg(format!("Failed to serialize jobs: {}", e)))?;
        
        fs::write(&filepath, content)
            .map_err(|e| anyhow::Error::msg(format!("Failed to write jobs file: {}", e)))?;
        
        Ok(())
    }

    pub fn create_job(&mut self, project_path: Option<String>, config_index: Option<usize>) -> usize {
        let job_id = self.next_job_id;
        self.next_job_id += 1;
        
        let mut job = Job::new(job_id);
        if let Some(path) = project_path {
            job = job.with_project_path(path);
        }
        if let Some(index) = config_index {
            job = job.with_config_index(index);
        }
        
        self.jobs.insert(job_id, job);
        
        // Auto-save after creating job
        if let Err(e) = self.save() {
            eprintln!("Warning: Failed to save job {}: {}", job_id, e);
        }
        
        job_id
    }

    pub fn update_job_status(&mut self, job_id: usize, status: JobStatus) -> anyhow::Result<()> {
        if let Some(job) = self.jobs.get_mut(&job_id) {
            job.set_status(status);
            self.save()?;
        } else {
            return Err(anyhow::Error::msg(format!("Job {} not found", job_id)));
        }
        Ok(())
    }

    pub fn get_active_jobs(&self) -> Vec<&Job> {
        self.jobs.values()
            .filter(|job| matches!(job.status, JobStatus::Created | JobStatus::Queued | JobStatus::Running))
            .collect()
    }

    pub fn get_completed_jobs(&self) -> Vec<&Job> {
        self.jobs.values()
            .filter(|job| job.is_completed())
            .collect()
    }

    pub fn get_runnable_jobs(&self) -> Vec<&Job> {
        self.jobs.values()
            .filter(|job| job.can_run())
            .collect()
    }

    pub fn delete_job(&mut self, job_id: usize) -> anyhow::Result<()> {
        if let Some(job) = self.jobs.get(&job_id) {
            if job.is_running() {
                return Err(anyhow::Error::msg("Cannot delete running job"));
            }
        }
        
        self.jobs.remove(&job_id);
        self.logs.remove(&job_id);
        self.logs_tmp.remove(&job_id);
        
        self.save()?;
        Ok(())
    }

    pub fn cleanup_old_jobs(&mut self, max_completed_jobs: usize) -> anyhow::Result<()> {
        let mut completed_jobs: Vec<_> = self.jobs.values()
            .filter(|job| job.is_completed())
            .map(|job| job.id)
            .collect();
        
        if completed_jobs.len() > max_completed_jobs {
            // Sort by completed_at timestamp
            completed_jobs.sort_by(|a, b| {
                let job_a = &self.jobs[a];
                let job_b = &self.jobs[b];
                job_a.completed_at.cmp(&job_b.completed_at)
            });
            
            let to_remove = completed_jobs.len() - max_completed_jobs;
            for job_id in completed_jobs.iter().take(to_remove) {
                self.delete_job(*job_id)?;
            }
        }
        
        Ok(())
    }

    /// Queue a job for execution
    pub fn queue_job(&mut self, job_id: usize) -> anyhow::Result<()> {
        if !self.jobs.contains_key(&job_id) {
            return Err(anyhow::Error::msg(format!("Job {} not found", job_id)));
        }

        // Check if job can run and dependencies are satisfied
        let can_queue = {
            if let Some(job) = self.jobs.get(&job_id) {
                if !job.can_run() {
                    return Err(anyhow::Error::msg(format!("Job {} cannot be queued (current status: {})", job_id, job.status)));
                }
                true
            } else {
                return Err(anyhow::Error::msg(format!("Job {} not found", job_id)));
            }
        };

        if can_queue {
            // Check if dependencies are satisfied before queuing
            if !self.are_dependencies_satisfied(job_id) {
                return Err(anyhow::Error::msg(format!("Job {} has unsatisfied dependencies", job_id)));
            }
            
            if let Some(job) = self.jobs.get_mut(&job_id) {
                job.set_status(JobStatus::Queued);
            }
            
            self.job_queue.push_back(job_id);
            
            // Sort queue by priority after adding new job
            self.sort_queue_by_priority();
            
            self.save()?;
            Ok(())
        } else {
            Err(anyhow::Error::msg("Cannot queue job"))
        }
    }

    /// Get the next job from the queue that can be executed
    pub fn get_next_queued_job(&mut self) -> Option<usize> {
        // Check if we've reached the concurrent job limit
        if self.max_concurrent_jobs > 0 {
            let running_jobs = self.get_running_jobs().len();
            if running_jobs >= self.max_concurrent_jobs {
                return None;
            }
        }

        // Find the next job that can be executed
        while let Some(job_id) = self.job_queue.pop_front() {
            if let Some(job) = self.jobs.get(&job_id) {
                if job.status == JobStatus::Queued {
                    return Some(job_id);
                }
            }
        }

        None
    }

    /// Get jobs that are currently running
    pub fn get_running_jobs(&self) -> Vec<&Job> {
        self.jobs.values()
            .filter(|job| job.is_running())
            .collect()
    }

    /// Get jobs that are currently queued
    pub fn get_queued_jobs(&self) -> Vec<&Job> {
        self.jobs.values()
            .filter(|job| matches!(job.status, JobStatus::Queued))
            .collect()
    }

    /// Cancel a queued job
    pub fn cancel_queued_job(&mut self, job_id: usize) -> anyhow::Result<()> {
        if let Some(job) = self.jobs.get_mut(&job_id) {
            if job.status == JobStatus::Queued {
                job.set_status(JobStatus::Cancelled);
                
                // Remove from queue
                self.job_queue.retain(|&id| id != job_id);
                
                self.save()?;
                Ok(())
            } else {
                Err(anyhow::Error::msg(format!("Job {} is not queued (current status: {})", job_id, job.status)))
            }
        } else {
            Err(anyhow::Error::msg(format!("Job {} not found", job_id)))
        }
    }

    /// Set the maximum number of concurrent jobs
    pub fn set_max_concurrent_jobs(&mut self, max_jobs: usize) {
        self.max_concurrent_jobs = max_jobs;
    }

    /// Get the current queue status
    pub fn get_queue_status(&self) -> (usize, usize, usize) {
        let queued = self.get_queued_jobs().len();
        let running = self.get_running_jobs().len();
        let available_slots = if self.max_concurrent_jobs > 0 {
            self.max_concurrent_jobs.saturating_sub(running)
        } else {
            usize::MAX
        };
        
        (queued, running, available_slots)
    }

    /// Process the job queue - start jobs that can be executed
    pub fn process_queue(&mut self) -> Vec<usize> {
        let mut started_jobs = Vec::new();
        
        while let Some(job_id) = self.get_next_queued_job() {
            // Check if job dependencies are satisfied
            if !self.are_dependencies_satisfied(job_id) {
                continue;
            }
            
            // Check if job is scheduled for future
            if let Some(job) = self.jobs.get(&job_id) {
                if !job.should_run_now() {
                    continue;
                }
            }
            
            if let Err(e) = self.update_job_status(job_id, JobStatus::Running) {
                self.add_log(job_id, format!("Failed to start job: {}", e));
                continue;
            }
            
            started_jobs.push(job_id);
        }
        
        started_jobs
    }

    /// Check if all dependencies for a job are satisfied
    pub fn are_dependencies_satisfied(&self, job_id: usize) -> bool {
        if let Some(job) = self.jobs.get(&job_id) {
            for dep_id in &job.dependencies {
                if let Some(dep_job) = self.jobs.get(dep_id) {
                    if !dep_job.is_completed() || dep_job.status == JobStatus::Failed {
                        return false;
                    }
                } else {
                    // Dependency job doesn't exist
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Get jobs that depend on the given job
    pub fn get_dependent_jobs(&self, job_id: usize) -> Vec<usize> {
        self.jobs.values()
            .filter(|job| job.dependencies.contains(&job_id))
            .map(|job| job.id)
            .collect()
    }

    /// Sort jobs by priority and dependencies (topological sort)
    pub fn sort_queue_by_priority(&mut self) {
        // Sort queue by priority (higher priority first) and dependencies
        let jobs = &self.jobs;
        self.job_queue.make_contiguous().sort_by(|&a, &b| {
            let job_a = jobs.get(&a);
            let job_b = jobs.get(&b);
            
            match (job_a, job_b) {
                (Some(ja), Some(jb)) => {
                    // First compare by priority
                    match jb.priority.cmp(&ja.priority) {
                        std::cmp::Ordering::Equal => {
                            // Then by dependencies (jobs with no deps come first)
                            ja.dependencies.len().cmp(&jb.dependencies.len())
                        }
                        other => other,
                    }
                }
                _ => std::cmp::Ordering::Equal,
            }
        });
    }
}

pub mod settings;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ser() {
        let toml_str = r#"
            [[jobs]]
            id = 1 
            status = "Created"
            infra = "Local"
            created_at = { secs_since_epoch = 1000000000, nanos_since_epoch = 0 }
            dependencies = []
            priority = 0

            [[jobs]]
            id = 2
            status = "Created"
            infra = "Local"
            created_at = { secs_since_epoch = 1000000000, nanos_since_epoch = 0 }
            dependencies = []
            priority = 0
        "#;
        
        let df = DataFile::new(toml_str).unwrap();
        assert_eq!(df.jobs.unwrap().len(), 2);
    }

    #[test]
    fn run_test_job_settings() {
        test_job_settings();
    }
}

