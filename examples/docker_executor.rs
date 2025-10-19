use std::path::Path;

use silva::{
    components::docker::{executor::DockerExecutor, job::JobStatus, logs::LogLine},
    job_config::config,
};
use tokio::sync::mpsc;

/// Example of running a Docker job programmatically.
///
/// This example demonstrates:
/// 1. Loading job configurations from @job.toml files
/// 2. Creating a Docker executor with message channels
/// 3. Running multiple jobs sequentially
/// 4. Handling job status updates and logs via channels
/// 5. Using cancellation signals to stop running jobs
///
/// The example runs two jobs:
/// - job_1: Runs to completion without interruption
/// - job_2: Demonstrates cancellation by sending a cancel signal during execution
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
        let job_cfg = job_folder_path.join("@job.toml");
        println!("Loading job configuration from {job_cfg:?} ...");
        let config = match config::JobConfig::load_from_file(job_cfg) {
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
        match &config.container {
            config::Container::DockerImage(img) => {
                println!("  Container: Docker Image '{img}'");
            }
            config::Container::DockerFile(path) => {
                println!("  Container: Dockerfile at '{path}'");
            }
        }
        println!("  Pre-script: {}", config.scripts.pre);
        println!("  Run-script: {}", config.scripts.run);
        println!("  Post-script: {}\n", config.scripts.post);

        // Create Docker executor
        println!("Initializing Docker executor...");
        let (tx, mut rx) = mpsc::channel::<(usize, JobStatus, LogLine)>(32);
        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);

        tokio::spawn(async move {
            let executor = DockerExecutor::new(tx)
                .map_err(|e| {
                    eprintln!("✗ Failed to initialize Docker: {e}");
                    eprintln!("  Make sure Docker daemon is running");
                })
                .unwrap();
            println!("✓ Docker executor initialized\n");

            println!("Starting job execution...");
            println!("─────────────────────────────\n");
            executor
                .run_job(&job_folder_path, &config, &mut cancel_rx)
                .await
                .map_err(|e| {
                    eprintln!("✗ Job execution error: {e}");
                })
                .unwrap();
            println!("\n─────────────────────────────");
            println!("Job Execution Complete\n");
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
