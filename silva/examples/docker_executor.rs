use std::path::Path;

use job_config::job::JobMeta;
use job_config::params::WorkflowParams;
use silva::components::{
    docker::{executor::DockerExecutor, job::JobStatus, logs::LogLine},
    workflow,
};
use tokio::sync::mpsc;

/// Example of running a Docker job programmatically.
///
/// This example demonstrates:
/// 1. Loading job configurations from .chiral/job.toml files
/// 2. Loading job parameters from params.toml (if available)
/// 3. Creating a Docker executor with message channels
/// 4. Running multiple jobs sequentially with parameter injection
/// 5. Handling job status updates and logs via channels
/// 6. Using cancellation signals to stop running jobs
///
/// The example runs two jobs:
/// - job_1: Runs to completion without interruption
/// - job_2: Demonstrates cancellation by sending a cancel signal during execution
///
/// Parameters are injected as environment variables with PARAM_ prefix.
///
/// To run this example:
/// Run: cargo run --example docker_executor
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Docker Job Example");
    println!("==================\n");

    for (job_path, try_cancel) in [
        ("./home/workflow_1/job_1", false),
        ("./home/workflow_1/job_2", true),
    ]
    .iter()
    {
        // Load job configuration
        let job_folder_path = Path::new(job_path).canonicalize().unwrap();
        let workflow_path = job_folder_path.parent().unwrap().to_owned();
        let job_name = job_folder_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        // Load from .chiral/job.toml
        let job_cfg = job_folder_path.join(".chiral/job.toml");

        println!("Loading job configuration from {job_cfg:?} ...");
        let config = match JobMeta::load_from_file(&job_cfg) {
            Ok(cfg) => {
                println!("✓ Configuration loaded successfully\n");
                cfg
            }
            Err(e) => {
                eprintln!("✗ Failed to load configuration: {e}");
                eprintln!("  Please check the file path");
                return Err(format!("Configuration error: {e}").into());
            }
        };

        // Display configuration
        println!("Job Configuration:");
        println!("  Name: {}", config.name);
        println!("  Description: {}", config.description);
        println!("  Container: Docker Image '{}'", config.container.image);
        println!("  GPU Enabled: {}", config.container.use_gpu);
        println!("  Pre-script: {}", config.scripts.pre);
        println!("  Run-script: {}", config.scripts.run);
        println!("  Post-script: {}\n", config.scripts.post);

        // Create Docker executor
        println!("Initializing Docker executor...");
        let (tx, mut rx) = mpsc::channel::<(usize, JobStatus, LogLine)>(32);
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);

        let workflow_params = WorkflowParams::new();
        let job = workflow::JobFolder::new(job_name.clone(), job_folder_path.clone());
        let job_params = job.load_params().ok().flatten().unwrap_or_default();

        tokio::spawn(async move {
            let executor = DockerExecutor::new(tx)
                .map_err(|e| {
                    eprintln!("✗ Failed to initialize Docker: {e}");
                    eprintln!("  Make sure Docker daemon is running");
                })
                .unwrap();
            println!("✓ Docker executor initialized\n");

            if !job_params.is_empty() {
                println!("Loaded {} parameter(s)", job_params.len());
                for (name, value) in &job_params {
                    println!("  {name} = {value}");
                }
                println!();
            }

            println!("Starting job execution...");
            println!("─────────────────────────────\n");

            let mut container_registry = std::collections::HashMap::new();
            let container_id = executor
                .run_job(
                    ("workflow_1", &workflow_path, &workflow_params),
                    (&job, &config, &job_params),
                    &mut container_registry,
                    &mut cancel_rx,
                )
                .await
                .map_err(|e| {
                    eprintln!("✗ Job execution error: {e}");
                })
                .unwrap();

            println!("\n─────────────────────────────");
            println!("Job Execution Complete\n");
            println!("Cleaning up container {container_id}...");
            executor.cleanup_containers(&[container_id]).await;
            println!("✓ Container cleaned up\n");
        });

        let mut cancel_sent = false;
        while let Some((idx, status, log_line)) = rx.recv().await {
            println!("Job {idx} status {status:?}: {log_line}");
            if *try_cancel && !cancel_sent && status == JobStatus::Running {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                println!("sending cancel signal");
                cancel_tx.send(()).await.unwrap();
                cancel_sent = true;
            }
        }
    }

    Ok(())
}
