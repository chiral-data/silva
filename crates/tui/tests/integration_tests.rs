use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;
use std::fs;

use silva_tui::data_model;
use silva_tui::ui::pages::job;
use silva_tui::ui::states::States;

#[cfg(test)]
mod integration_tests {
    use super::*;

    fn create_test_store() -> data_model::Store {
        let temp_dir = TempDir::new().unwrap();
        let test_data_dir = temp_dir.path().join("silva_test");
        fs::create_dir_all(&test_data_dir).unwrap();
        
        std::env::set_var("SILVA_DATA_DIR", test_data_dir.to_str().unwrap());
        
        data_model::Store::default()
    }

    fn create_test_project(temp_dir: &TempDir, name: &str, job_configs: Vec<&str>) -> std::path::PathBuf {
        let project_dir = temp_dir.path().join(name);
        fs::create_dir_all(&project_dir).unwrap();
        
        if job_configs.len() == 1 {
            fs::write(project_dir.join("@job.toml"), job_configs[0]).unwrap();
        } else {
            for (i, config) in job_configs.iter().enumerate() {
                fs::write(project_dir.join(format!("@job_{}.toml", i + 1)), config).unwrap();
            }
        }
        
        // Create a README.md to make it a valid project
        fs::write(project_dir.join("README.md"), "# Test Project").unwrap();
        
        project_dir
    }

    #[test]
    fn test_single_job_config_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Create a project with single job config
        let single_job_config = r#"
[files]
inputs = ["input.txt"]
outputs = ["output.txt"]
scripts = ["run.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run.sh"
use_gpu = false
"#;
        
        let project_dir = create_test_project(&temp_dir, "single_job_project", vec![single_job_config]);
        
        // Create project and set as selected
        let project = data_model::project::Project::new(project_dir.clone());
        let project_mgr = data_model::project::Manager::new().unwrap();
        store.project_sel = Some((project, project_mgr));
        
        // Simulate creating a new job
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        let job_id = job_mgr_lock.create_job(Some(project_dir.display().to_string()), Some(0));
        drop(job_mgr_lock);
        
        // Verify job was created correctly
        let job_mgr = store.job_mgr.lock().unwrap();
        let job = job_mgr.jobs.get(&job_id).unwrap();
        assert_eq!(job.id, job_id);
        assert_eq!(job.status, data_model::job::JobStatus::Created);
        assert_eq!(job.project_path, Some(project_dir.display().to_string()));
        assert_eq!(job.config_index, Some(0));
    }

    #[test]
    fn test_multiple_job_config_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Create a project with multiple job configs
        let job_config1 = r#"
[files]
inputs = ["input1.txt"]
outputs = ["output1.txt"]
scripts = ["run1.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run1.sh"
use_gpu = false
"#;
        
        let job_config2 = r#"
[files]
inputs = ["input2.txt"]
outputs = ["output2.txt"]
scripts = ["run2.sh"]

[dok]
base_image = "python:3.9"
plan = "v100-32gb"
http_port = 8080
"#;
        
        let project_dir = create_test_project(&temp_dir, "multi_job_project", vec![job_config1, job_config2]);
        
        // Create project and set as selected
        let project = data_model::project::Project::new(project_dir.clone());
        let project_mgr = data_model::project::Manager::new().unwrap();
        store.project_sel = Some((project, project_mgr));
        
        // Test that we can get multiple job settings
        let settings_vec = data_model::job::Job::get_settings_vec(&project_dir).unwrap();
        assert_eq!(settings_vec.len(), 2);
        
        // Create jobs for each configuration
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        
        let job1_id = job_mgr_lock.create_job(Some(project_dir.display().to_string()), Some(0));
        let job2_id = job_mgr_lock.create_job(Some(project_dir.display().to_string()), Some(1));
        
        drop(job_mgr_lock);
        
        // Verify jobs were created correctly
        let job_mgr = store.job_mgr.lock().unwrap();
        
        let job1 = job_mgr.jobs.get(&job1_id).unwrap();
        assert_eq!(job1.config_index, Some(0));
        
        let job2 = job_mgr.jobs.get(&job2_id).unwrap();
        assert_eq!(job2.config_index, Some(1));
    }

    #[test]
    fn test_job_execution_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Create a simple job config
        let job_config = r#"
[files]
inputs = ["input.txt"]
outputs = ["output.txt"]
scripts = ["run.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run.sh"
use_gpu = false
"#;
        
        let project_dir = create_test_project(&temp_dir, "test_project", vec![job_config]);
        
        // Create project and set as selected
        let project = data_model::project::Project::new(project_dir.clone());
        let project_mgr = data_model::project::Manager::new().unwrap();
        store.project_sel = Some((project, project_mgr));
        
        // Create a job
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        let job_id = job_mgr_lock.create_job(Some(project_dir.display().to_string()), Some(0));
        drop(job_mgr_lock);
        
        // Set job as selected
        states.job_states.set_selected_job_id(Some(job_id));
        
        // Test job status updates
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        
        // Update to running
        job_mgr_lock.update_job_status(job_id, data_model::job::JobStatus::Running).unwrap();
        let job = job_mgr_lock.jobs.get(&job_id).unwrap();
        assert_eq!(job.status, data_model::job::JobStatus::Running);
        assert!(job.started_at.is_some());
        
        // Update to completed
        job_mgr_lock.update_job_status(job_id, data_model::job::JobStatus::Completed).unwrap();
        let job = job_mgr_lock.jobs.get(&job_id).unwrap();
        assert_eq!(job.status, data_model::job::JobStatus::Completed);
        assert!(job.completed_at.is_some());
        assert!(job.duration.is_some());
    }

    #[test]
    fn test_job_cancellation_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Create a job
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        let job_id = job_mgr_lock.create_job(None, None);
        job_mgr_lock.update_job_status(job_id, data_model::job::JobStatus::Running).unwrap();
        drop(job_mgr_lock);
        
        // Set job as selected
        states.job_states.set_selected_job_id(Some(job_id));
        
        // Test cancellation
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        job_mgr_lock.update_job_status(job_id, data_model::job::JobStatus::Cancelled).unwrap();
        
        let job = job_mgr_lock.jobs.get(&job_id).unwrap();
        assert_eq!(job.status, data_model::job::JobStatus::Cancelled);
        assert!(job.completed_at.is_some());
        assert!(job.duration.is_some());
    }

    #[test]
    fn test_job_persistence_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let test_data_dir = temp_dir.path().join("silva_test");
        fs::create_dir_all(&test_data_dir).unwrap();
        
        std::env::set_var("SILVA_DATA_DIR", test_data_dir.to_str().unwrap());
        
        // Create and save jobs
        {
            let store = data_model::Store::default();
            let job_mgr = store.job_mgr.clone();
            let mut job_mgr_lock = job_mgr.lock().unwrap();
            
            let job1_id = job_mgr_lock.create_job(Some("/project1".to_string()), Some(0));
            let job2_id = job_mgr_lock.create_job(Some("/project2".to_string()), Some(1));
            
            job_mgr_lock.update_job_status(job1_id, data_model::job::JobStatus::Running).unwrap();
            job_mgr_lock.update_job_status(job2_id, data_model::job::JobStatus::Completed).unwrap();
            
            job_mgr_lock.add_log(job1_id, "Test log 1".to_string());
            job_mgr_lock.add_log(job2_id, "Test log 2".to_string());
            
            drop(job_mgr_lock);
        }
        
        // Load and verify jobs
        {
            let store = data_model::Store::default();
            let job_mgr = store.job_mgr.lock().unwrap();
            
            assert_eq!(job_mgr.jobs.len(), 2);
            
            let job1 = job_mgr.jobs.get(&1).unwrap();
            assert_eq!(job1.status, data_model::job::JobStatus::Running);
            assert_eq!(job1.project_path, Some("/project1".to_string()));
            assert_eq!(job1.config_index, Some(0));
            
            let job2 = job_mgr.jobs.get(&2).unwrap();
            assert_eq!(job2.status, data_model::job::JobStatus::Completed);
            assert_eq!(job2.project_path, Some("/project2".to_string()));
            assert_eq!(job2.config_index, Some(1));
            
            // Logs are not persisted, but that's expected behavior
            assert!(job_mgr.logs.is_empty());
        }
    }

    #[test]
    fn test_job_display_formatting() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        
        // Create a job with full information
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        let job_id = job_mgr_lock.create_job(
            Some("/path/to/test_project".to_string()), 
            Some(2)
        );
        
        job_mgr_lock.update_job_status(job_id, data_model::job::JobStatus::Running).unwrap();
        
        let job = job_mgr_lock.jobs.get(&job_id).unwrap();
        let display = format!("{}", job);
        
        // Check that display includes expected information
        assert!(display.contains(&job_id.to_string()));
        assert!(display.contains("Running"));
        assert!(display.contains("@job_3.toml"));
        assert!(display.contains("test_project"));
        
        drop(job_mgr_lock);
    }

    #[test]
    fn test_job_manager_state_consistency() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        
        // Create multiple jobs
        let job1_id = job_mgr_lock.create_job(None, None);
        let job2_id = job_mgr_lock.create_job(None, None);
        let job3_id = job_mgr_lock.create_job(None, None);
        
        // Test next_job_id increments correctly
        assert_eq!(job_mgr_lock.next_job_id, 4);
        
        // Test various status updates
        job_mgr_lock.update_job_status(job1_id, data_model::job::JobStatus::Running).unwrap();
        job_mgr_lock.update_job_status(job2_id, data_model::job::JobStatus::Completed).unwrap();
        job_mgr_lock.update_job_status(job3_id, data_model::job::JobStatus::Failed).unwrap();
        
        // Test filtering methods
        let active_jobs = job_mgr_lock.get_active_jobs();
        assert_eq!(active_jobs.len(), 1); // Only running job
        
        let completed_jobs = job_mgr_lock.get_completed_jobs();
        assert_eq!(completed_jobs.len(), 2); // Completed and failed
        
        let runnable_jobs = job_mgr_lock.get_runnable_jobs();
        assert_eq!(runnable_jobs.len(), 1); // Only failed job can be rerun
        
        drop(job_mgr_lock);
    }

    #[test]
    fn test_config_selection_logic() {
        let temp_dir = TempDir::new().unwrap();
        
        // Test single config scenario
        let single_config = r#"
[files]
inputs = ["input.txt"]
outputs = ["output.txt"]
scripts = ["run.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run.sh"
use_gpu = false
"#;
        
        let single_project_dir = create_test_project(&temp_dir, "single_project", vec![single_config]);
        let single_settings = data_model::job::Job::get_settings_vec(&single_project_dir).unwrap();
        assert_eq!(single_settings.len(), 1);
        
        // Test multiple config scenario
        let config1 = r#"
[files]
inputs = ["input1.txt"]
outputs = ["output1.txt"]
scripts = ["run1.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run1.sh"
use_gpu = false
"#;
        
        let config2 = r#"
[files]
inputs = ["input2.txt"]
outputs = ["output2.txt"]
scripts = ["run2.sh"]

[local]
docker_image = "python:3.9"
script = "run2.sh"
use_gpu = true
"#;
        
        let multi_project_dir = create_test_project(&temp_dir, "multi_project", vec![config1, config2]);
        let multi_settings = data_model::job::Job::get_settings_vec(&multi_project_dir).unwrap();
        assert_eq!(multi_settings.len(), 2);
        
        // Verify configs are different
        assert_ne!(multi_settings[0].files.inputs, multi_settings[1].files.inputs);
        assert_ne!(multi_settings[0].files.outputs, multi_settings[1].files.outputs);
        
        let local1 = multi_settings[0].infra_local.as_ref().unwrap();
        let local2 = multi_settings[1].infra_local.as_ref().unwrap();
        assert_ne!(local1.docker_image, local2.docker_image);
        assert_ne!(local1.use_gpu, local2.use_gpu);
    }

    #[test]
    fn test_concurrent_job_management() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        
        let job_mgr = store.job_mgr.clone();
        
        // Simulate concurrent job creation
        let handles: Vec<_> = (0..10).map(|i| {
            let job_mgr = job_mgr.clone();
            std::thread::spawn(move || {
                let mut job_mgr_lock = job_mgr.lock().unwrap();
                let job_id = job_mgr_lock.create_job(
                    Some(format!("/project_{}", i)), 
                    Some(i % 3)
                );
                job_mgr_lock.update_job_status(job_id, data_model::job::JobStatus::Running).unwrap();
                job_id
            })
        }).collect();
        
        let job_ids: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        
        // Verify all jobs were created
        let job_mgr_lock = job_mgr.lock().unwrap();
        assert_eq!(job_mgr_lock.jobs.len(), 10);
        
        for job_id in job_ids {
            let job = job_mgr_lock.jobs.get(&job_id).unwrap();
            assert_eq!(job.status, data_model::job::JobStatus::Running);
        }
        
        drop(job_mgr_lock);
    }

    #[test]
    fn test_backwards_compatibility() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create an old-style project with just @job.toml
        let old_project_dir = temp_dir.path().join("old_project");
        fs::create_dir_all(&old_project_dir).unwrap();
        
        let old_config = r#"
[files]
inputs = ["input.txt"]
outputs = ["output.txt"]
scripts = ["run.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run.sh"
use_gpu = false
"#;
        
        fs::write(old_project_dir.join("@job.toml"), old_config).unwrap();
        fs::write(old_project_dir.join("README.md"), "# Old Project").unwrap();
        
        // Should still work with new job management system
        let settings_vec = data_model::job::Job::get_settings_vec(&old_project_dir).unwrap();
        assert_eq!(settings_vec.len(), 1);
        
        // Create a job using the old-style config
        let mut store = create_test_store();
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        
        let job_id = job_mgr_lock.create_job(
            Some(old_project_dir.display().to_string()), 
            Some(0)
        );
        
        let job = job_mgr_lock.jobs.get(&job_id).unwrap();
        assert_eq!(job.config_index, Some(0));
        
        drop(job_mgr_lock);
    }
}