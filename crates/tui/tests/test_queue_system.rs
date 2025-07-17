use silva_tui::data_model::job::{JobStatus, Manager};

#[test]
fn test_job_queue_basic_operations() {
    let mut manager = Manager::new();
    
    // Create some jobs
    let job1 = manager.create_job(Some("project1".to_string()), Some(0));
    let job2 = manager.create_job(Some("project2".to_string()), Some(0));
    let job3 = manager.create_job(Some("project3".to_string()), Some(0));
    
    // Initially no jobs are queued
    assert_eq!(manager.get_queued_jobs().len(), 0);
    assert_eq!(manager.get_running_jobs().len(), 0);
    
    // Queue the jobs
    manager.queue_job(job1).unwrap();
    manager.queue_job(job2).unwrap();
    manager.queue_job(job3).unwrap();
    
    // Check queue status
    assert_eq!(manager.get_queued_jobs().len(), 3);
    assert_eq!(manager.get_running_jobs().len(), 0);
    
    // Verify jobs are in queued state
    assert_eq!(manager.jobs[&job1].status, JobStatus::Queued);
    assert_eq!(manager.jobs[&job2].status, JobStatus::Queued);
    assert_eq!(manager.jobs[&job3].status, JobStatus::Queued);
}

#[test]
fn test_job_queue_processing() {
    let mut manager = Manager::new();
    manager.set_max_concurrent_jobs(2); // Allow max 2 concurrent jobs
    
    // Create and queue jobs
    let job1 = manager.create_job(Some("project1".to_string()), Some(0));
    let job2 = manager.create_job(Some("project2".to_string()), Some(0));
    let job3 = manager.create_job(Some("project3".to_string()), Some(0));
    
    manager.queue_job(job1).unwrap();
    manager.queue_job(job2).unwrap();
    manager.queue_job(job3).unwrap();
    
    // Process the queue
    let started_jobs = manager.process_queue();
    
    // Should start max 2 jobs due to concurrency limit
    assert_eq!(started_jobs.len(), 2);
    assert_eq!(manager.get_running_jobs().len(), 2);
    assert_eq!(manager.get_queued_jobs().len(), 1);
    
    // The started jobs should be running
    assert!(started_jobs.contains(&job1));
    assert!(started_jobs.contains(&job2));
    assert_eq!(manager.jobs[&job1].status, JobStatus::Running);
    assert_eq!(manager.jobs[&job2].status, JobStatus::Running);
    assert_eq!(manager.jobs[&job3].status, JobStatus::Queued);
}

#[test]
fn test_job_queue_completion_and_next_start() {
    let mut manager = Manager::new();
    manager.set_max_concurrent_jobs(1); // Only 1 job at a time
    
    // Create and queue jobs
    let job1 = manager.create_job(Some("project1".to_string()), Some(0));
    let job2 = manager.create_job(Some("project2".to_string()), Some(0));
    
    manager.queue_job(job1).unwrap();
    manager.queue_job(job2).unwrap();
    
    // Process queue - should start job1
    let started_jobs = manager.process_queue();
    assert_eq!(started_jobs.len(), 1);
    assert!(started_jobs.contains(&job1));
    assert_eq!(manager.jobs[&job1].status, JobStatus::Running);
    assert_eq!(manager.jobs[&job2].status, JobStatus::Queued);
    
    // Complete job1
    manager.update_job_status(job1, JobStatus::Completed).unwrap();
    
    // Process queue again - should start job2
    let started_jobs = manager.process_queue();
    assert_eq!(started_jobs.len(), 1);
    assert!(started_jobs.contains(&job2));
    assert_eq!(manager.jobs[&job2].status, JobStatus::Running);
}

#[test]
fn test_job_queue_cancellation() {
    let mut manager = Manager::new();
    
    // Create and queue jobs
    let job1 = manager.create_job(Some("project1".to_string()), Some(0));
    let job2 = manager.create_job(Some("project2".to_string()), Some(0));
    
    manager.queue_job(job1).unwrap();
    manager.queue_job(job2).unwrap();
    
    // Cancel job1
    manager.cancel_queued_job(job1).unwrap();
    
    // Check that job1 is cancelled and removed from queue
    assert_eq!(manager.jobs[&job1].status, JobStatus::Cancelled);
    assert_eq!(manager.get_queued_jobs().len(), 1);
    assert!(manager.get_queued_jobs().iter().any(|job| job.id == job2));
}

#[test]
fn test_job_queue_status() {
    let mut manager = Manager::new();
    manager.set_max_concurrent_jobs(2);
    
    // Create jobs
    let job1 = manager.create_job(Some("project1".to_string()), Some(0));
    let job2 = manager.create_job(Some("project2".to_string()), Some(0));
    let job3 = manager.create_job(Some("project3".to_string()), Some(0));
    
    // Queue jobs
    manager.queue_job(job1).unwrap();
    manager.queue_job(job2).unwrap();
    manager.queue_job(job3).unwrap();
    
    // Check initial queue status
    let (queued, running, available) = manager.get_queue_status();
    assert_eq!(queued, 3);
    assert_eq!(running, 0);
    assert_eq!(available, 2);
    
    // Process queue
    manager.process_queue();
    
    // Check queue status after processing
    let (queued, running, available) = manager.get_queue_status();
    assert_eq!(queued, 1);
    assert_eq!(running, 2);
    assert_eq!(available, 0);
}