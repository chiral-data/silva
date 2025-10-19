use tempfile::TempDir;
use std::fs;

use silva_tui::data_model::job::{Job, JobStatus, Manager, ResourceUsage};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_creation() {
        let job = Job::new(1);
        assert_eq!(job.id, 1);
        assert_eq!(job.status, JobStatus::Created);
        assert!(job.project_path.is_none());
        assert!(job.config_index.is_none());
        assert!(job.started_at.is_none());
        assert!(job.completed_at.is_none());
        assert!(job.duration.is_none());
        assert!(job.exit_code.is_none());
        assert!(job.error_message.is_none());
        assert!(job.resource_usage.is_none());
    }

    #[test]
    fn test_job_builder_pattern() {
        let job = Job::new(1)
            .with_project_path("/test/project".to_string())
            .with_config_index(2);
        
        assert_eq!(job.project_path, Some("/test/project".to_string()));
        assert_eq!(job.config_index, Some(2));
    }

    #[test]
    fn test_job_status_transitions() {
        let mut job = Job::new(1);
        
        // Created -> Running
        job.set_status(JobStatus::Running);
        assert_eq!(job.status, JobStatus::Running);
        assert!(job.started_at.is_some());
        assert!(job.completed_at.is_none());
        
        // Running -> Completed
        job.set_status(JobStatus::Completed);
        assert_eq!(job.status, JobStatus::Completed);
        assert!(job.started_at.is_some());
        assert!(job.completed_at.is_some());
        assert!(job.duration.is_some());
    }

    #[test]
    fn test_job_manager_creation() {
        let manager = Manager::new();
        assert!(manager.jobs.is_empty());
        assert_eq!(manager.next_job_id, 1);
    }

    #[test]
    fn test_manager_create_job() {
        let mut manager = Manager::new();
        
        let job_id = manager.create_job(Some("/test/project".to_string()), Some(1));
        assert_eq!(job_id, 1);
        assert_eq!(manager.next_job_id, 2);
        
        let job = manager.jobs.get(&job_id).unwrap();
        assert_eq!(job.id, job_id);
        assert_eq!(job.project_path, Some("/test/project".to_string()));
        assert_eq!(job.config_index, Some(1));
        assert_eq!(job.status, JobStatus::Created);
    }

    #[test]
    fn test_manager_update_job_status() {
        let mut manager = Manager::new();
        let job_id = manager.create_job(None, None);
        
        manager.update_job_status(job_id, JobStatus::Running).unwrap();
        let job = manager.jobs.get(&job_id).unwrap();
        assert_eq!(job.status, JobStatus::Running);
        
        manager.update_job_status(job_id, JobStatus::Completed).unwrap();
        let job = manager.jobs.get(&job_id).unwrap();
        assert_eq!(job.status, JobStatus::Completed);
    }

    #[test]
    fn test_manager_update_nonexistent_job() {
        let mut manager = Manager::new();
        let result = manager.update_job_status(999, JobStatus::Running);
        assert!(result.is_err());
    }

    #[test]
    fn test_manager_get_active_jobs() {
        let mut manager = Manager::new();
        
        let job1 = manager.create_job(None, None);
        let job2 = manager.create_job(None, None);
        let job3 = manager.create_job(None, None);
        
        manager.update_job_status(job1, JobStatus::Running).unwrap();
        manager.update_job_status(job2, JobStatus::Completed).unwrap();
        manager.update_job_status(job3, JobStatus::Queued).unwrap();
        
        let active_jobs = manager.get_active_jobs();
        assert_eq!(active_jobs.len(), 2); // Running and Queued
        
        let active_ids: Vec<usize> = active_jobs.iter().map(|j| j.id).collect();
        assert!(active_ids.contains(&job1));
        assert!(active_ids.contains(&job3));
        assert!(!active_ids.contains(&job2));
    }

    #[test]
    fn test_manager_get_completed_jobs() {
        let mut manager = Manager::new();
        
        let job1 = manager.create_job(None, None);
        let job2 = manager.create_job(None, None);
        let job3 = manager.create_job(None, None);
        
        manager.update_job_status(job1, JobStatus::Running).unwrap();
        manager.update_job_status(job2, JobStatus::Completed).unwrap();
        manager.update_job_status(job3, JobStatus::Failed).unwrap();
        
        let completed_jobs = manager.get_completed_jobs();
        assert_eq!(completed_jobs.len(), 2); // Completed and Failed
        
        let completed_ids: Vec<usize> = completed_jobs.iter().map(|j| j.id).collect();
        assert!(completed_ids.contains(&job2));
        assert!(completed_ids.contains(&job3));
        assert!(!completed_ids.contains(&job1));
    }

    #[test]
    fn test_manager_get_runnable_jobs() {
        let mut manager = Manager::new();
        
        let job1 = manager.create_job(None, None);
        let job2 = manager.create_job(None, None);
        let job3 = manager.create_job(None, None);
        let job4 = manager.create_job(None, None);
        
        manager.update_job_status(job1, JobStatus::Running).unwrap();
        manager.update_job_status(job2, JobStatus::Completed).unwrap();
        manager.update_job_status(job3, JobStatus::Failed).unwrap();
        // job4 stays in Created state
        
        let runnable_jobs = manager.get_runnable_jobs();
        assert_eq!(runnable_jobs.len(), 2); // Created and Failed
        
        let runnable_ids: Vec<usize> = runnable_jobs.iter().map(|j| j.id).collect();
        assert!(runnable_ids.contains(&job3));
        assert!(runnable_ids.contains(&job4));
        assert!(!runnable_ids.contains(&job1));
        assert!(!runnable_ids.contains(&job2));
    }

    #[test]
    fn test_manager_delete_job() {
        let mut manager = Manager::new();
        let job_id = manager.create_job(None, None);
        
        // Add some logs
        manager.add_log(job_id, "Test log".to_string());
        manager.add_log_tmp(job_id, "Test tmp log".to_string());
        
        assert!(manager.jobs.contains_key(&job_id));
        assert!(manager.logs.contains_key(&job_id));
        assert!(manager.logs_tmp.contains_key(&job_id));
        
        manager.delete_job(job_id).unwrap();
        
        assert!(!manager.jobs.contains_key(&job_id));
        assert!(!manager.logs.contains_key(&job_id));
        assert!(!manager.logs_tmp.contains_key(&job_id));
    }

    #[test]
    fn test_manager_delete_running_job() {
        let mut manager = Manager::new();
        let job_id = manager.create_job(None, None);
        manager.update_job_status(job_id, JobStatus::Running).unwrap();
        
        let result = manager.delete_job(job_id);
        assert!(result.is_err());
        assert!(manager.jobs.contains_key(&job_id));
    }

    #[test]
    fn test_manager_log_management() {
        let mut manager = Manager::new();
        let job_id = manager.create_job(None, None);
        
        // Add regular logs
        for i in 0..25 {
            manager.add_log(job_id, format!("Log {}", i));
        }
        
        let logs = manager.logs.get(&job_id).unwrap();
        assert_eq!(logs.len(), 20); // Should be capped at 20
        
        // Should contain the most recent 20 logs
        assert_eq!(logs.back().unwrap(), "Log 24");
        assert_eq!(logs.front().unwrap(), "Log 5");
        
        // Test temporary logs
        manager.add_log_tmp(job_id, "Temp log".to_string());
        assert_eq!(manager.logs_tmp.get(&job_id).unwrap(), "Temp log");
        
        manager.clear_log_tmp(&job_id);
        assert!(!manager.logs_tmp.contains_key(&job_id));
    }

    #[test]
    fn test_job_display_format() {
        let mut job = Job::new(1);
        job = job.with_project_path("/path/to/test_project".to_string());
        job = job.with_config_index(2);
        
        let display = format!("{}", job);
        assert!(display.contains("1")); // Job ID
        assert!(display.contains("Created")); // Status
        assert!(display.contains("@job_3.toml")); // Config
        assert!(display.contains("test_project")); // Project name
    }

    #[test]
    fn test_job_settings_single_config() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        
        // Create @job.toml
        let job_toml = r#"
[files]
inputs = ["input1.txt", "input2.txt"]
outputs = ["output.txt"]
scripts = ["run.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run.sh"
use_gpu = false
"#;
        
        fs::write(project_dir.join("@job.toml"), job_toml).unwrap();
        
        let settings_vec = Job::get_settings_vec(project_dir).unwrap();
        assert_eq!(settings_vec.len(), 1);
        
        let settings = &settings_vec[0];
        assert_eq!(settings.files.inputs.len(), 2);
        assert_eq!(settings.files.outputs.len(), 1);
        assert_eq!(settings.files.scripts.len(), 1);
        assert!(settings.infra_local.is_some());
        assert!(settings.dok.is_none());
    }

    #[test]
    fn test_job_settings_multiple_configs() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        
        // Create @job_1.toml
        let job1_toml = r#"
[files]
inputs = ["input1.txt"]
outputs = ["output1.txt"]
scripts = ["run1.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run1.sh"
use_gpu = false
"#;
        
        // Create @job_2.toml
        let job2_toml = r#"
[files]
inputs = ["input2.txt"]
outputs = ["output2.txt"]
scripts = ["run2.sh"]

[dok]
base_image = "python:3.9"
plan = "v100-32gb"
http_port = 8080
"#;
        
        fs::write(project_dir.join("@job_1.toml"), job1_toml).unwrap();
        fs::write(project_dir.join("@job_2.toml"), job2_toml).unwrap();
        
        let settings_vec = Job::get_settings_vec(project_dir).unwrap();
        assert_eq!(settings_vec.len(), 2);
        
        let settings1 = &settings_vec[0];
        assert_eq!(settings1.files.inputs.len(), 1);
        assert_eq!(settings1.files.inputs[0], "input1.txt");
        assert!(settings1.infra_local.is_some());
        assert!(settings1.dok.is_none());
        
        let settings2 = &settings_vec[1];
        assert_eq!(settings2.files.inputs.len(), 1);
        assert_eq!(settings2.files.inputs[0], "input2.txt");
        assert!(settings2.infra_local.is_none());
        assert!(settings2.dok.is_some());
    }

    #[test]
    fn test_job_settings_no_config() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        
        let result = Job::get_settings_vec(project_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_job_settings_with_gaps() {
        let temp_dir = TempDir::new().unwrap();
        let project_dir = temp_dir.path();
        
        // Create @job_1.toml and @job_3.toml (skip @job_2.toml)
        let job_toml = r#"
[files]
inputs = ["input.txt"]
outputs = ["output.txt"]
scripts = ["run.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run.sh"
use_gpu = false
"#;
        
        fs::write(project_dir.join("@job_1.toml"), job_toml).unwrap();
        fs::write(project_dir.join("@job_3.toml"), job_toml).unwrap();
        
        let settings_vec = Job::get_settings_vec(project_dir).unwrap();
        assert_eq!(settings_vec.len(), 1); // Should stop at the first gap
    }

    #[test]
    fn test_job_predicates() {
        let mut job = Job::new(1);
        
        // Initial state
        assert!(!job.is_running());
        assert!(!job.is_completed());
        assert!(job.can_run());
        
        // Running state
        job.set_status(JobStatus::Running);
        assert!(job.is_running());
        assert!(!job.is_completed());
        assert!(!job.can_run());
        
        // Completed state
        job.set_status(JobStatus::Completed);
        assert!(!job.is_running());
        assert!(job.is_completed());
        assert!(!job.can_run());
        
        // Failed state (can be rerun)
        job.set_status(JobStatus::Failed);
        assert!(!job.is_running());
        assert!(job.is_completed());
        assert!(job.can_run());
    }

    #[test]
    fn test_job_exit_code() {
        let mut job = Job::new(1);
        
        job.set_exit_code(0);
        assert_eq!(job.exit_code, Some(0));
        assert_eq!(job.status, JobStatus::Completed);
        
        let mut job2 = Job::new(2);
        job2.set_exit_code(1);
        assert_eq!(job2.exit_code, Some(1));
        assert_eq!(job2.status, JobStatus::Failed);
    }

    #[test]
    fn test_job_error() {
        let mut job = Job::new(1);
        
        job.set_error("Test error".to_string());
        assert_eq!(job.error_message, Some("Test error".to_string()));
        assert_eq!(job.status, JobStatus::Failed);
    }

    #[test]
    fn test_job_resource_usage() {
        let mut job = Job::new(1);
        let usage = ResourceUsage {
            cpu_usage: Some(50.0),
            memory_usage: Some(1024),
            disk_usage: Some(512),
        };
        
        job.set_resource_usage(usage.clone());
        assert!(job.resource_usage.is_some());
        let stored_usage = job.resource_usage.as_ref().unwrap();
        assert_eq!(stored_usage.cpu_usage, Some(50.0));
        assert_eq!(stored_usage.memory_usage, Some(1024));
        assert_eq!(stored_usage.disk_usage, Some(512));
    }

    #[test]
    fn test_manager_next_job_id() {
        let mut manager = Manager::new();
        
        assert_eq!(manager.next_job_id, 1);
        
        let job1_id = manager.create_job(None, None);
        assert_eq!(job1_id, 1);
        assert_eq!(manager.next_job_id, 2);
        
        let job2_id = manager.create_job(None, None);
        assert_eq!(job2_id, 2);
        assert_eq!(manager.next_job_id, 3);
    }
}