use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tempfile::TempDir;
use std::fs;

use silva_tui::data_model::job::{Job, JobStatus, Manager, ResourceUsage};
use silva_tui::data_model::job::settings::Settings;

#[cfg(test)]
mod job_tests {
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
    fn test_job_with_project_path() {
        let job = Job::new(1).with_project_path("/test/project".to_string());
        assert_eq!(job.project_path, Some("/test/project".to_string()));
    }

    #[test]
    fn test_job_with_config_index() {
        let job = Job::new(1).with_config_index(2);
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
    fn test_job_status_failed() {
        let mut job = Job::new(1);
        job.set_status(JobStatus::Running);
        
        job.set_status(JobStatus::Failed);
        assert_eq!(job.status, JobStatus::Failed);
        assert!(job.duration.is_some());
    }

    #[test]
    fn test_job_status_cancelled() {
        let mut job = Job::new(1);
        job.set_status(JobStatus::Running);
        
        job.set_status(JobStatus::Cancelled);
        assert_eq!(job.status, JobStatus::Cancelled);
        assert!(job.duration.is_some());
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
        assert_eq!(job.resource_usage, Some(usage));
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
    fn test_job_display() {
        let mut job = Job::new(1);
        job = job.with_project_path("/test/project".to_string());
        job = job.with_config_index(0);
        
        let display = format!("{}", job);
        assert!(display.contains("1")); // Job ID
        assert!(display.contains("Created")); // Status
        assert!(display.contains("@job_1.toml")); // Config
        assert!(display.contains("project")); // Project name
    }
}

#[cfg(test)]
mod manager_tests {
    use super::*;

    #[test]
    fn test_manager_creation() {
        let manager = Manager::new();
        assert!(manager.jobs.is_empty());
        assert_eq!(manager.next_job_id, 1);
    }

    #[test]
    fn test_create_job() {
        let mut manager = Manager::new();
        
        let job_id = manager.create_job(Some("/test/project".to_string()), Some(1));
        assert_eq!(job_id, 1);
        assert_eq!(manager.next_job_id, 2);
        
        let job = manager.jobs.get(&job_id).unwrap();
        assert_eq!(job.id, job_id);
        assert_eq!(job.project_path, Some("/test/project".to_string()));
        assert_eq!(job.config_index, Some(1));
    }

    #[test]
    fn test_update_job_status() {
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
    fn test_update_nonexistent_job() {
        let mut manager = Manager::new();
        let result = manager.update_job_status(999, JobStatus::Running);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_active_jobs() {
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
    fn test_get_completed_jobs() {
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
    fn test_get_runnable_jobs() {
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
    fn test_delete_job() {
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
    fn test_delete_running_job() {
        let mut manager = Manager::new();
        let job_id = manager.create_job(None, None);
        manager.update_job_status(job_id, JobStatus::Running).unwrap();
        
        let result = manager.delete_job(job_id);
        assert!(result.is_err());
        assert!(manager.jobs.contains_key(&job_id));
    }

    #[test]
    fn test_cleanup_old_jobs() {
        let mut manager = Manager::new();
        
        // Create 5 jobs and complete them
        let mut job_ids = Vec::new();
        for _ in 0..5 {
            let job_id = manager.create_job(None, None);
            manager.update_job_status(job_id, JobStatus::Completed).unwrap();
            job_ids.push(job_id);
        }
        
        assert_eq!(manager.jobs.len(), 5);
        
        // Keep only 3 most recent completed jobs
        manager.cleanup_old_jobs(3).unwrap();
        
        assert_eq!(manager.jobs.len(), 3);
        
        // The 2 oldest jobs should be removed
        assert!(!manager.jobs.contains_key(&job_ids[0]));
        assert!(!manager.jobs.contains_key(&job_ids[1]));
        assert!(manager.jobs.contains_key(&job_ids[2]));
        assert!(manager.jobs.contains_key(&job_ids[3]));
        assert!(manager.jobs.contains_key(&job_ids[4]));
    }

    #[test]
    fn test_log_management() {
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
}

#[cfg(test)]
mod persistence_tests {
    use super::*;

    #[test]
    fn test_save_and_load_empty_manager() {
        let temp_dir = TempDir::new().unwrap();
        let test_data_dir = temp_dir.path().join("silva_test");
        fs::create_dir_all(&test_data_dir).unwrap();
        
        // Override the data directory temporarily
        std::env::set_var("SILVA_DATA_DIR", test_data_dir.to_str().unwrap());
        
        let manager = Manager::new();
        manager.save().unwrap();
        
        let loaded_manager = Manager::load().unwrap();
        assert!(loaded_manager.jobs.is_empty());
        assert_eq!(loaded_manager.next_job_id, 1);
    }

    #[test]
    fn test_save_and_load_with_jobs() {
        let temp_dir = TempDir::new().unwrap();
        let test_data_dir = temp_dir.path().join("silva_test");
        fs::create_dir_all(&test_data_dir).unwrap();
        
        std::env::set_var("SILVA_DATA_DIR", test_data_dir.to_str().unwrap());
        
        let mut manager = Manager::new();
        let job1_id = manager.create_job(Some("/project1".to_string()), Some(0));
        let job2_id = manager.create_job(Some("/project2".to_string()), Some(1));
        
        manager.update_job_status(job1_id, JobStatus::Running).unwrap();
        manager.update_job_status(job2_id, JobStatus::Completed).unwrap();
        
        manager.save().unwrap();
        
        let loaded_manager = Manager::load().unwrap();
        assert_eq!(loaded_manager.jobs.len(), 2);
        assert_eq!(loaded_manager.next_job_id, 3);
        
        let job1 = loaded_manager.jobs.get(&job1_id).unwrap();
        assert_eq!(job1.status, JobStatus::Running);
        assert_eq!(job1.project_path, Some("/project1".to_string()));
        assert_eq!(job1.config_index, Some(0));
        
        let job2 = loaded_manager.jobs.get(&job2_id).unwrap();
        assert_eq!(job2.status, JobStatus::Completed);
        assert_eq!(job2.project_path, Some("/project2".to_string()));
        assert_eq!(job2.config_index, Some(1));
    }

    #[test]
    fn test_auto_save_on_job_creation() {
        let temp_dir = TempDir::new().unwrap();
        let test_data_dir = temp_dir.path().join("silva_test");
        fs::create_dir_all(&test_data_dir).unwrap();
        
        std::env::set_var("SILVA_DATA_DIR", test_data_dir.to_str().unwrap());
        
        let mut manager = Manager::new();
        manager.create_job(None, None);
        
        // Load a new manager - should have the job
        let loaded_manager = Manager::load().unwrap();
        assert_eq!(loaded_manager.jobs.len(), 1);
    }

    #[test]
    fn test_auto_save_on_status_update() {
        let temp_dir = TempDir::new().unwrap();
        let test_data_dir = temp_dir.path().join("silva_test");
        fs::create_dir_all(&test_data_dir).unwrap();
        
        std::env::set_var("SILVA_DATA_DIR", test_data_dir.to_str().unwrap());
        
        let mut manager = Manager::new();
        let job_id = manager.create_job(None, None);
        manager.update_job_status(job_id, JobStatus::Running).unwrap();
        
        let loaded_manager = Manager::load().unwrap();
        let job = loaded_manager.jobs.get(&job_id).unwrap();
        assert_eq!(job.status, JobStatus::Running);
    }
}

#[cfg(test)]
mod job_settings_tests {
    use super::*;

    #[test]
    fn test_get_single_job_settings() {
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
    fn test_get_multiple_job_settings() {
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
    fn test_no_job_settings() {
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
}