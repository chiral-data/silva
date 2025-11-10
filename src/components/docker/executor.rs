//! # Docker Integration Usage Guide
//!
//! This project now includes comprehensive Docker support via the Bollard library, allowing you to build images, run containers, and execute jobs with sequential script execution.
//!
//! ## Features
//!
//! ### Core Functionality
//! - **Image Building**: Build Docker images from Dockerfiles
//! - **Image Pulling**: Pull Docker images from registries
//! - **Container Management**: Create, start, and manage containers
//! - **Sequential Script Execution**: Run pre_run.sh → run.sh → post_run.sh in order
//! - **Log Streaming**: Real-time log capture with stdout/stderr separation
//! - **UI Integration**: Visual log viewer with scrolling and status display
//!
//! ### Job Configuration
//!
//! Jobs are configured via `@job.toml` file:
//!
//! ```toml
//! [container]
//! # Option 1: Use a Docker image from a registry
//! docker_image = "ubuntu:22.04"
//!
//! # Option 2: Build from a Dockerfile (uncomment to use)
//! # dockerfile = "./Dockerfile"
//!
//! [scripts]
//! # All fields are optional with defaults shown
//! pre = "pre_run.sh"    # default: "pre_run.sh"
//! run = "run.sh"        # default: "run.sh"
//! post = "post_run.sh"  # default: "post_run.sh"
//! ```
//!
//! ## Programmatic Usage
//!
//! ### UI Keyboard Shortcuts
//!
//! When using the terminal UI:
//!
//! - **`d`** - Toggle Docker logs popup
//! - **`↑`** - Scroll logs up
//! - **`↓`** - Scroll logs down
//! - **`b`** - Jump to bottom of logs
//! - **`q`** - Quit application
//!
//! ## Architecture
//!
//! ### Module Structure
//!
//! src/docker
//!  - mod.rs          # Module exports
//!  - error.rs        # Docker-specific errors
//!  - executor.rs     # Core Docker operations
//!  - logs.rs         # Log buffer management
//!  - state.rs        # Job state tracking
//!
//! ### Docker Executor
//!
//! The `DockerExecutor` handles:
//! - Docker client initialization
//! - Image building from Dockerfile
//! - Image pulling from registries
//! - Container lifecycle management
//! - Script execution with output capture
//!
//! ### Log Management
//!
//! Logs are stored in a circular buffer (default 10,000 lines) with:
//! - Timestamps for each line
//! - Source identification (stdout/stderr)
//! - Efficient rotation when buffer fills
//!
//! ### State Management
//!
//! Job execution tracks:
//! - Current status (Idle, Building, Running scripts, etc.)
//! - Start/end timestamps
//! - Container ID
//! - Error messages
//! - Log buffer
//!
//! ## Script Execution
//!
//! Scripts are executed sequentially inside the container:
//!
//! 1. **Pre-run script** (`pre_run.sh`) - Setup/preparation
//! 2. **Main script** (`run.sh`) - Primary computation
//! 3. **Post-run script** (`post_run.sh`) - Cleanup/finalization
//!
//! Execution stops immediately if any script fails (returns non-zero exit code).
//!
//! ## Error Handling
//!
//! The system provides detailed error types:
//! - `BollardError` - Docker API errors
//! - `ImageBuildFailed` - Image build failures
//! - `ContainerCreateFailed` - Container creation errors
//! - `ScriptExecutionFailed` - Script execution failures with exit codes
//! - `LogStreamError` - Log streaming issues
//!
//! ## Testing
//!
//! Run tests with:
//! ```bash
//! cargo test docker
//! ```
//!
//! Current test coverage:
//! - Log buffer operations (push, rotate, tail)
//! - Job state transitions
//! - Status tracking
//! - Error handling
//!
//! ## Requirements
//!
//! - Docker daemon running locally
//! - Bollard 0.17+
//! - Tokio async runtime
//! - Scripts must be available in container (mount volume or include in image)
//!
//! ## Future Enhancements
//!
//! Potential improvements:
//! - Volume mounting for script injection
//! - Environment variable passing
//! - Network configuration
//! - Resource limits (CPU/memory)
//! - Multiple container orchestration
//! - Background job execution
//! - Job queue management

use bollard::Docker;
use bollard::container::{Config, LogOutput, RemoveContainerOptions};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::{BuildImageOptions, CreateImageOptions};
use futures_util::stream::StreamExt;
use std::collections::HashMap;
use std::default::Default;
use std::path::Path;
use tokio::sync::mpsc;

use crate::components::workflow;
use crate::job_config::config::{Container, JobConfig};

use super::error::DockerError;
use super::job::JobStatus;
use super::logs::{LogBuffer, LogLine, LogSource};

/// Result of a Docker job execution.
#[derive(Debug)]
pub struct JobResult {
    pub container_id: Option<String>,
    pub status: JobStatus,
    pub logs: LogBuffer,
}

impl Default for JobResult {
    fn default() -> Self {
        Self {
            container_id: None,
            status: JobStatus::Idle,
            logs: LogBuffer::default(),
        }
    }
}

impl JobResult {
    /// Acquires resources from another JobResult, transferring ownership of container ID and logs.
    ///
    /// # Arguments
    ///
    /// * `source` - The source JobResult to acquire resources from (will be emptied)
    ///
    /// # Behavior
    ///
    /// - Takes the container_id from source if present
    /// - Appends all logs from source to self
    /// - Resets status to Idle
    pub fn acquire(&mut self, source: &mut JobResult) {
        if let Some(id) = source.container_id.take() {
            self.container_id = Some(id);
        }
        self.status = JobStatus::Idle;
        self.logs.append(&mut source.logs);
    }
}

/// Docker executor for building images and running jobs.
pub struct DockerExecutor {
    client: Docker,
    tx: mpsc::Sender<(usize, JobStatus, LogLine)>,
    job_idx: usize,
}

impl DockerExecutor {
    /// Creates a new Docker executor.
    ///
    /// Connects to Docker using the default connection method (typically Unix socket or Windows named pipe).
    ///
    /// # Arguments
    ///
    /// * `tx` - Message channel sender for streaming job status and logs
    ///
    /// # Returns
    ///
    /// * `Ok(DockerExecutor)` - Successfully connected to Docker daemon
    /// * `Err(DockerError)` - Failed to connect to Docker (daemon may not be running)
    pub fn new(tx: mpsc::Sender<(usize, JobStatus, LogLine)>) -> Result<Self, DockerError> {
        let client = Docker::connect_with_local_defaults()?;
        Ok(Self {
            client,
            tx,
            job_idx: 0,
        })
    }

    /// Sets the current job index for message tagging.
    ///
    /// # Arguments
    ///
    /// * `new_job_idx` - The job index to use for subsequent messages sent via the channel
    ///
    /// # Usage
    ///
    /// This is useful when running multiple jobs with the same executor instance,
    /// allowing the receiver to distinguish which job each message belongs to.
    pub fn set_job_idx(&mut self, new_job_idx: usize) {
        self.job_idx = new_job_idx;
    }

    /// Sends a message via the channel with the current job index.
    ///
    /// # Arguments
    ///
    /// * `status` - The job status
    /// * `log_line` - The log line to send
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Message sent successfully
    /// * `Err(DockerError)` - Channel send error
    async fn tx_send(&self, status: JobStatus, log_line: LogLine) -> Result<(), DockerError> {
        self.tx
            .send((self.job_idx, status, log_line))
            .await
            .map_err(|e| {
                DockerError::ChannelSendMessageError(format!(
                    "Message channel receiver dropped: {e}"
                ))
            })
    }

    /// Builds a Docker image from a Dockerfile.
    ///
    /// # Arguments
    ///
    /// * `dockerfile_path` - Path to the Dockerfile
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Image ID of the built image
    /// * `Err(DockerError)` - Build error
    pub async fn build_image(
        &self,
        image_name: &str,
        dockerfile_path: &Path,
    ) -> Result<String, DockerError> {
        // update job entry
        let log_line = LogLine::new(
            LogSource::Stdout,
            format!("Building image from: {dockerfile_path:?}"),
        );
        self.tx_send(JobStatus::BuildingImage, log_line).await?;

        // docker builds image
        let path = Path::new(dockerfile_path);
        let context_path = path
            .parent()
            .ok_or_else(|| DockerError::ImageBuildFailed("Invalid Dockerfile path".to_string()))?;

        // Create tar archive of the build context
        let tar_file = self.create_tar_archive(context_path)?;
        let image_tag = format!("{image_name}:latest");

        let build_options = BuildImageOptions {
            dockerfile: path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Dockerfile"),
            t: &image_tag,
            rm: true,
            ..Default::default()
        };

        let mut stream = self
            .client
            .build_image(build_options, None, Some(tar_file.into()));

        let mut image_id = String::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => {
                    if let Some(id) = output.stream
                        && id.contains("Successfully built")
                    {
                        image_id = id
                            .split_whitespace()
                            .last()
                            .unwrap_or("")
                            .trim()
                            .to_string();
                    }
                    if let Some(error) = output.error {
                        return Err(DockerError::ImageBuildFailed(error));
                    }
                }
                Err(e) => return Err(DockerError::ImageBuildFailed(e.to_string())),
            }
        }

        let log_line = LogLine::new(
            LogSource::Stdout,
            format!("Building image complete with image id: {image_id}"),
        );
        self.tx_send(JobStatus::BuildingImage, log_line).await?;

        Ok(image_tag)
    }

    /// Pulls a Docker image from a registry.
    ///
    /// # Arguments
    ///
    /// * `image_url` - Docker image URL (e.g., "ubuntu:22.04")
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Image pulled successfully
    /// * `Err(DockerError)` - Pull error
    pub async fn pull_image(&self, image_url: &str) -> Result<(), DockerError> {
        // update job entry
        let log_line = LogLine::new(LogSource::Stdout, format!("Pulling image: {image_url}"));
        self.tx_send(JobStatus::PullingImage, log_line).await?;

        // docker pulls image
        let options = Some(CreateImageOptions {
            from_image: image_url,
            ..Default::default()
        });

        let mut stream = self.client.create_image(options, None, None);

        while let Some(result) = stream.next().await {
            match result {
                Ok(_) => {}
                Err(e) => return Err(DockerError::ImageBuildFailed(e.to_string())),
            }
        }

        Ok(())
    }

    /// Runs a job based on the provided configuration.
    ///
    /// # Arguments
    ///
    /// * `job_folder` - Path to the job folder (mounted as /workspace in container)
    /// * `config` - Job configuration specifying container image/Dockerfile and scripts
    /// * `cancel_rx` - Channel receiver for cancellation signals
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - Container ID of the job container (for cleanup later)
    /// * `Err(DockerError)` - Fatal execution error
    ///
    /// # Behavior
    ///
    /// 1. Pulls/builds the Docker image specified in config
    /// 2. Creates and starts a container with job_folder mounted
    /// 3. Executes pre-run, run, and post-run scripts sequentially
    /// 4. Returns the container ID (container is NOT stopped/removed)
    /// 5. Can be interrupted via cancel_rx channel
    ///
    /// Job status and logs are streamed via the message channel throughout execution.
    ///
    /// **Note**: The container is left running. Call `cleanup_containers()` after all jobs complete.
    pub async fn run_job(
        &self,
        workflow_name: &str,
        workflow_folder: &Path, // tmp workflow folder
        job: &workflow::Job,
        config: &JobConfig,
        container_registry: &mut std::collections::HashMap<String, String>,
        cancel_rx: &mut mpsc::Receiver<()>,
    ) -> Result<String, DockerError> {
        let work_dir = "/workspace";

        let image_name = match &config.container {
            Container::DockerImage(url) => {
                self.pull_image(url).await?;
                url.clone()
            }
            Container::DockerFile(path) => {
                let job_folder = workflow_folder.join(&job.name);
                let docker_file_path = job_folder.join(path);
                let image_name = format!("{workflow_name}_{}", job.name);
                match self
                    .client
                    .inspect_image(format!("{image_name}:latest").as_str())
                    .await
                {
                    Ok(_) => {
                        // If inspect_image succeeds, the image exists
                        let log_line = LogLine::new(
                            LogSource::Stdout,
                            format!(
                                "docker image {image_name} exists, skip building, remove the image to rebuild ..."
                            ),
                        );
                        self.tx_send(JobStatus::BuildingImage, log_line).await?;
                        format!("{image_name}:latest")
                    }
                    Err(bollard::errors::Error::DockerResponseServerError {
                        status_code: 404,
                        ..
                    }) => {
                        // A 404 status code from the Docker API means "no such image"
                        self.build_image(&image_name, &docker_file_path).await?
                    }
                    Err(e) => {
                        // Handle other errors (e.g., connection, authentication)
                        return Err(DockerError::ImageBuildFailed(format!(
                            "inspect image {image_name} error: {e}"
                        )));
                    }
                }
            }
        };

        // Check if we already have a container for this image
        let container_id = if let Some(existing_id) = container_registry.get(&image_name) {
            let log_line = LogLine::new(
                LogSource::Stdout,
                format!("Reusing existing container {existing_id} for image {image_name}"),
            );
            self.tx_send(JobStatus::Running, log_line).await?;
            existing_id.clone()
        } else {
            // Create new container
        let log_line = LogLine::new(
            LogSource::Stdout,
            format!("Creating container with image: {image_name}"),
        );
        self.tx_send(JobStatus::CreatingContainer, log_line).await?;

        // Build host config based on GPU requirement
        let mut host_config = if config.use_gpu {
            let log_line = LogLine::new(
                LogSource::Stdout,
                "GPU support enabled for this container".to_string(),
            );
            self.tx_send(JobStatus::CreatingContainer, log_line).await?;

            bollard::models::HostConfig {
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                device_requests: Some(vec![bollard::models::DeviceRequest {
                    driver: Some("".into()),
                    count: Some(-1),
                    device_ids: None,
                    capabilities: Some(vec![vec!["gpu".into()]]),
                    options: Some(HashMap::new()),
                }]),
                ..Default::default()
            }
        } else {
            bollard::models::HostConfig {
                extra_hosts: Some(vec!["host.docker.internal:host-gateway".into()]),
                ..Default::default()
            }
        };
        let workflow_folder_str = workflow_folder.to_str().unwrap();
        let volume_binds = vec![format!("{workflow_folder_str}:{work_dir}")];
        host_config.binds = Some(volume_binds);
        let container_config = Config {
            image: Some(image_name.clone()),
            tty: Some(true),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            host_config: Some(host_config),
            working_dir: Some(work_dir.to_string()),
            ..Default::default()
        };

        let container = self
            .client
            .create_container::<String, String>(None, container_config)
            .await
            .map_err(|e| DockerError::ContainerCreateFailed(e.to_string()))?;
        let log_line = LogLine::new(
            LogSource::Stdout,
            format!(
                "Container created: {}, binding {workflow_folder_str} to {work_dir}",
                container.id
            ),
        );
        self.tx_send(JobStatus::CreatingContainer, log_line).await?;

        // Start container
        self.client
            .start_container::<String>(&container.id, None)
            .await
            .map_err(|e| DockerError::ContainerStartFailed(e.to_string()))?;

        let log_line = LogLine::new(
            LogSource::Stdout,
            format!("Waiting for Container {} running ... ", container.id),
        );
        self.tx_send(JobStatus::CreatingContainer, log_line).await?;

        // Wait for container to be running (timeout: 30 seconds)
        self.wait_for_container_running(&container.id, 30).await?;

        let log_line = LogLine::new(
            LogSource::Stdout,
            format!("Now container {} is running ... ", container.id),
        );
        self.tx_send(
            JobStatus::ContainerRunning(container.id.to_string()),
            log_line,
        )
        .await?;

        let log_line = LogLine::new(LogSource::Stdout, "Container started and ready".to_string());
        self.tx_send(JobStatus::Running, log_line).await?;

            // Register the new container in the registry
            container_registry.insert(image_name.clone(), container.id.clone());
            container.id
        };

        // Execute scripts sequentially
        let scripts = vec![
            ("pre_run.sh", &config.scripts.pre),
            ("run.sh", &config.scripts.run),
            ("post_run.sh", &config.scripts.post),
        ];

        let mut all_scripts_succeeded = true;
        for (name, script) in scripts {
            if ["pre_run.sh", "post_run.sh"].contains(&name)
                && !workflow_folder.join(&job.name).join(script).exists()
            {
                let log_line = LogLine::new(
                    LogSource::Stdout,
                    format!("Script {script} not found ... skip"),
                );
                self.tx_send(JobStatus::Running, log_line).await?;
                continue;
            }

            let log_line = LogLine::new(LogSource::Stdout, format!("Executing script: {script}"));
            self.tx_send(JobStatus::Running, log_line).await?;

            let job_workdir = Path::new(work_dir).join(&job.name);
            match self
                .exec_script(
                    &container_id,
                    job_workdir.to_str().unwrap(),
                    script,
                    cancel_rx,
                )
                .await
            {
                Ok(exit_code) => {
                    if exit_code != 0 {
                        let log_line = LogLine::new(
                            LogSource::Stderr,
                            format!("Script {script} failed with exit code {exit_code}"),
                        );
                        self.tx_send(JobStatus::Failed, log_line).await?;
                        all_scripts_succeeded = false;
                        break;
                    } else {
                        let log_line = LogLine::new(
                            LogSource::Stdout,
                            format!("Script {script} completed successfully"),
                        );
                        self.tx_send(JobStatus::Completed, log_line).await?;
                    }
                }
                Err(e) => {
                    let log_line = LogLine::new(LogSource::Stderr, format!("Error: {e}"));
                    self.tx_send(JobStatus::Failed, log_line).await?;
                    all_scripts_succeeded = false;
                    break;
                }
            }
        }

        // Collect output files if all scripts succeeded
        if all_scripts_succeeded && !config.outputs.is_empty() {
            let log_line = LogLine::new(
                LogSource::Stdout,
                "Collecting output files...".to_string(),
            );
            self.tx_send(JobStatus::Running, log_line).await?;

            let job_workdir = Path::new(work_dir).join(&job.name);
            match self
                .collect_output_files(&container_id, job_workdir.to_str().unwrap(), &config.outputs, cancel_rx)
                .await
            {
                Ok(file_count) => {
                    let log_line = LogLine::new(
                        LogSource::Stdout,
                        format!("Collected {file_count} output file(s) to outputs/ folder"),
                    );
                    self.tx_send(JobStatus::Completed, log_line).await?;
                }
                Err(e) => {
                    let log_line = LogLine::new(
                        LogSource::Stderr,
                        format!("Warning: Failed to collect output files: {e}"),
                    );
                    self.tx_send(JobStatus::Running, log_line).await?;
                }
            }
        }

        // Return container ID for cleanup later
        let log_line = LogLine::new(
            LogSource::Stdout,
            format!("Job completed, container {container_id} will be cleaned up after workflow finishes"),
        );
        self.tx_send(JobStatus::Completed, log_line).await?;

        Ok(container_id)
    }

    /// Cleans up (stops and removes) multiple containers.
    ///
    /// # Arguments
    ///
    /// * `container_ids` - List of container IDs to clean up
    ///
    /// # Behavior
    ///
    /// - Stops each container
    /// - Removes each container with force option
    /// - Logs cleanup progress
    /// - Continues even if some containers fail to stop/remove
    pub async fn cleanup_containers(&self, container_ids: &[String]) {
        if container_ids.is_empty() {
            return;
        }

        let log_line = LogLine::new(
            LogSource::Stdout,
            format!("Cleaning up {} container(s)...", container_ids.len()),
        );
        let _ = self.tx_send(JobStatus::Completed, log_line).await;

        for container_id in container_ids {
            // Stop container
            match self.client.stop_container(container_id, None).await {
                Ok(_) => {
                    let log_line = LogLine::new(
                        LogSource::Stdout,
                        format!("Stopped container {container_id}"),
                    );
                    let _ = self.tx_send(JobStatus::Completed, log_line).await;
                }
                Err(e) => {
                    let log_line = LogLine::new(
                        LogSource::Stderr,
                        format!("Failed to stop container {container_id}: {e}"),
                    );
                    let _ = self.tx_send(JobStatus::Completed, log_line).await;
                }
            }

            // Remove container
            let remove_options = RemoveContainerOptions {
                force: true,
                ..Default::default()
            };
            match self.client.remove_container(container_id, Some(remove_options)).await {
                Ok(_) => {
                    let log_line = LogLine::new(
                        LogSource::Stdout,
                        format!("Removed container {container_id}"),
                    );
                    let _ = self.tx_send(JobStatus::Completed, log_line).await;
                }
                Err(e) => {
                    let log_line = LogLine::new(
                        LogSource::Stderr,
                        format!("Failed to remove container {container_id}: {e}"),
                    );
                    let _ = self.tx_send(JobStatus::Completed, log_line).await;
                }
            }
        }

        let log_line = LogLine::new(
            LogSource::Stdout,
            "All containers cleaned up".to_string(),
        );
        let _ = self.tx_send(JobStatus::Completed, log_line).await;
    }

    /// Executes a script inside a running container.
    async fn exec_script(
        &self,
        container_id: &str,
        job_work_dir: &str,
        script: &str,
        cancel_rx: &mut mpsc::Receiver<()>,
    ) -> Result<i64, DockerError> {
        let exec_config = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(vec!["/bin/bash", "-c", script]),
            working_dir: Some(job_work_dir),
            ..Default::default()
        };

        let exec = self.client.create_exec(container_id, exec_config).await?;
        let log_line = LogLine::new(
            LogSource::Stdout,
            format!(
                "Docker exec {} with container {container_id} created",
                exec.id
            ),
        );
        self.tx_send(JobStatus::Running, log_line).await?;

        match self.client.start_exec(&exec.id, None).await? {
            StartExecResults::Attached { mut output, .. } => {
                loop {
                    tokio::select! {
                        result = output.next() => {
                            match result {
                                Some(Ok(LogOutput::StdOut { message })) => {
                                    let content = String::from_utf8_lossy(&message)
                                        .trim_end_matches("\n").to_string();
                                    let log_line = LogLine::new(LogSource::Stdout, content);
                                    self.tx_send(JobStatus::Running, log_line).await?;
                                }
                                Some(Ok(LogOutput::StdErr { message })) => {
                                    let content = String::from_utf8_lossy(&message)
                                        .trim_end_matches("\n").to_string();
                                    let log_line = LogLine::new(LogSource::Stderr, content);
                                    self.tx_send(JobStatus::Running, log_line).await?;
                                }
                                Some(Err(e)) => {
                                    return Err(DockerError::LogStreamError(e.to_string()));
                                }
                                None => {
                                    // stream ended
                                    break;
                                }
                                _ => {}
                            }
                        }
                        _ = cancel_rx.recv() => {
                            break;
                        }
                    }
                }
            }
            StartExecResults::Detached => {
                return Err(DockerError::LogStreamError(
                    "Exec started in detached mode".to_string(),
                ));
            }
        }

        // Get exit code
        let inspect = self.client.inspect_exec(&exec.id).await?;
        let exit_code = inspect.exit_code.unwrap_or(1);

        Ok(exit_code)
    }

    /// Collects output files from the container based on glob patterns.
    ///
    /// Creates an outputs/ directory in the job folder and copies matching files.
    /// Since the job folder is bind-mounted, files will automatically appear on the host.
    ///
    /// # Arguments
    ///
    /// * `container_id` - The running container ID
    /// * `job_work_dir` - Working directory inside the container (e.g., "/workspace/job1")
    /// * `output_patterns` - List of glob patterns to match files (e.g., ["*.csv", "results/*.json"])
    /// * `cancel_rx` - Channel receiver for cancellation signals
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - Number of files collected
    /// * `Err(DockerError)` - Collection error
    async fn collect_output_files(
        &self,
        container_id: &str,
        job_work_dir: &str,
        output_patterns: &[String],
        cancel_rx: &mut mpsc::Receiver<()>,
    ) -> Result<usize, DockerError> {
        // Build a bash script that creates outputs/ dir and copies matching files
        // Note: We don't use 'set -e' to make it more forgiving when patterns don't match
        let mut script = String::from("mkdir -p outputs\n");
        script.push_str("file_count=0\n");
        script.push_str("echo 'Collecting output files...'\n");

        for pattern in output_patterns {
            // Use shopt -s nullglob to handle cases where pattern doesn't match anything
            script.push_str("shopt -s nullglob\n");
            script.push_str(&format!("matched_files=({pattern})\n"));
            script.push_str("shopt -u nullglob\n");
            script.push_str("if [ ${#matched_files[@]} -eq 0 ]; then\n");
            script.push_str(&format!("  echo 'Pattern \"{pattern}\" matched no files'\n"));
            script.push_str("else\n");
            script.push_str("  for file in \"${matched_files[@]}\"; do\n");
            script.push_str("    if [ -f \"$file\" ]; then\n");
            script.push_str("      if cp -v \"$file\" outputs/ 2>&1; then\n");
            script.push_str("        file_count=$((file_count + 1))\n");
            script.push_str("      else\n");
            script.push_str("        echo \"Warning: Failed to copy file: $file\"\n");
            script.push_str("      fi\n");
            script.push_str("    elif [ -d \"$file\" ]; then\n");
            script.push_str("      if cp -rv \"$file\" outputs/ 2>&1; then\n");
            script.push_str("        file_count=$((file_count + 1))\n");
            script.push_str("      else\n");
            script.push_str("        echo \"Warning: Failed to copy directory: $file\"\n");
            script.push_str("      fi\n");
            script.push_str("    fi\n");
            script.push_str("  done\n");
            script.push_str("fi\n");
        }

        script.push_str("echo \"Total files collected: $file_count\"\n");
        script.push_str("exit 0\n");

        let exec_config = CreateExecOptions {
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            cmd: Some(vec!["/bin/bash", "-c", &script]),
            working_dir: Some(job_work_dir),
            ..Default::default()
        };

        let exec = self.client.create_exec(container_id, exec_config).await?;

        let mut file_count = 0;
        let mut last_line = String::new();
        match self.client.start_exec(&exec.id, None).await? {
            StartExecResults::Attached { mut output, .. } => {
                loop {
                    tokio::select! {
                        result = output.next() => {
                            match result {
                                Some(Ok(LogOutput::StdOut { message })) => {
                                    let content = String::from_utf8_lossy(&message)
                                        .trim_end_matches("\n").to_string();

                                    // Save the last line to parse file count at the end
                                    if !content.is_empty() {
                                        last_line = content.clone();
                                        let log_line = LogLine::new(LogSource::Stdout, content);
                                        self.tx_send(JobStatus::Running, log_line).await?;
                                    }
                                }
                                Some(Ok(LogOutput::StdErr { message })) => {
                                    let content = String::from_utf8_lossy(&message)
                                        .trim_end_matches("\n").to_string();
                                    if !content.is_empty() {
                                        let log_line = LogLine::new(LogSource::Stderr, content);
                                        self.tx_send(JobStatus::Running, log_line).await?;
                                    }
                                }
                                Some(Err(e)) => {
                                    return Err(DockerError::LogStreamError(e.to_string()));
                                }
                                None => {
                                    break;
                                }
                                _ => {}
                            }
                        }
                        _ = cancel_rx.recv() => {
                            break;
                        }
                    }
                }
            }
            StartExecResults::Detached => {
                return Err(DockerError::LogStreamError(
                    "Exec started in detached mode".to_string(),
                ));
            }
        }

        // Parse the file count from the last line
        if let Some(count) = last_line.rsplit_once(':').and_then(|(_, num)| num.trim().parse().ok()) {
            file_count = count;
        } else {
            let log_line = LogLine::new(LogSource::Stderr, format!("parse file count from -{last_line}- error"));
            self.tx_send(JobStatus::Running, log_line).await?;
        }

        // Get exit code to ensure the script ran successfully
        let inspect = self.client.inspect_exec(&exec.id).await?;
        let exit_code = inspect.exit_code.unwrap_or(1);

        if exit_code != 0 {
            return Err(DockerError::ScriptExecutionFailed {
                script: "output_collection".to_string(),
                exit_code,
            });
        }

        Ok(file_count)
    }

    /// Creates a tar archive from a directory for Docker build context.
    fn create_tar_archive(&self, path: &Path) -> Result<Vec<u8>, DockerError> {
        let mut tar = tar::Builder::new(Vec::new());
        tar.append_dir_all(".", path)?;
        let data = tar
            .into_inner()
            .map_err(|e| DockerError::IoError(std::io::Error::other(e)))?;
        Ok(data)
    }

    /// Waits for a container to reach running state.
    ///
    /// # Arguments
    ///
    /// * `container_id` - The container ID to monitor
    /// * `timeout_secs` - Maximum seconds to wait for the container to start
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Container is running
    /// * `Err(DockerError)` - Container failed to start or timeout reached
    async fn wait_for_container_running(
        &self,
        container_id: &str,
        timeout_secs: u64,
    ) -> Result<(), DockerError> {
        use tokio::time::{Duration, sleep};

        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        loop {
            let inspect = self.client.inspect_container(container_id, None).await?;

            if let Some(state) = inspect.state {
                if state.running == Some(true) {
                    return Ok(());
                }
                if state.dead == Some(true) || state.oom_killed == Some(true) {
                    return Err(DockerError::ContainerStartFailed(
                        "Container died during startup".to_string(),
                    ));
                }
            }

            if start.elapsed() > timeout {
                return Err(DockerError::ContainerStartFailed(format!(
                    "Container did not start within {timeout_secs} seconds",
                )));
            }

            sleep(Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_executor_creation() {
        // This test will fail if Docker is not available
        let (tx, _rx) = mpsc::channel::<(usize, JobStatus, LogLine)>(32);
        let result = DockerExecutor::new(tx);
        // We don't assert success here as Docker may not be available in test environment
        match result {
            Ok(_) => println!("Docker connection successful"),
            Err(e) => println!("Docker connection failed (expected in CI): {e}"),
        }
    }
}
