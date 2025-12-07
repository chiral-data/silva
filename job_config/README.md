# job_config

Configuration parser for Silva workflow jobs with TOML support.

## Overview

`job_config` provides a simple and type-safe way to parse job configuration files for workflow systems. It supports Docker containers, custom scripts, GPU usage, job dependencies, file input/output specifications, and parameter definitions.

## Features

- **Container Configuration**: Support for both Docker images and Dockerfiles
- **Custom Scripts**: Configure pre-run, run, and post-run scripts
- **GPU Support**: Optional GPU usage flag
- **Job Dependencies**: Specify dependencies on other jobs with `depends_on`
- **File I/O**: Define input and output file patterns with glob support
- **Parameter Definitions**: Define typed parameters with defaults and validation
- **TOML Format**: Easy-to-read and write configuration format

## Modules

- `job` - Unified `JobMeta` struct combining job configuration and metadata (TOML)
- `workflow` - Workflow-level metadata with `WorkflowMetadata` struct (TOML)
- `params` - JSON-based parameter storage with `JobParams` and `WorkflowParams` types

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
job_config = "0.3.3"
```

## Usage

### Basic Example

```rust
use job_config::job::JobMeta;

// Load job metadata from a TOML file
let meta = JobMeta::load_from_file("job.toml")?;

println!("Job: {} - {}", meta.name, meta.description);

// Access container configuration
match &meta.container {
    job_config::job::Container::DockerImage(image) => {
        println!("Using Docker image: {}", image);
    }
    job_config::job::Container::DockerFile(path) => {
        println!("Using Dockerfile: {}", path);
    }
}

// Generate default parameters
let defaults = meta.generate_default_params();
```

### TOML Configuration Format

#### Minimal Configuration

```toml
name = "My Job"
description = "A simple job"

[container]
image = "ubuntu:22.04"
```

#### Full Configuration with Parameters

```toml
name = "Training Job"
description = "Train a machine learning model"

[container]
image = "python:3.11"
use_gpu = true

[scripts]
pre = "setup.sh"
run = "train.sh"
post = "cleanup.sh"

# Job dependencies
depends_on = ["prepare_data", "download_model"]

# Input files from dependency outputs (supports glob patterns)
inputs = ["*.csv", "models/*.pt"]

# Output files to collect (supports glob patterns)
outputs = ["results/*.json", "trained_model.pt"]

# Parameter definitions
[params.learning_rate]
type = "float"
default = 0.001
hint = "Learning rate for training"

[params.epochs]
type = "integer"
default = 100
hint = "Number of training epochs"

[params.model_type]
type = "enum"
default = "resnet"
hint = "Model architecture to use"
enum_values = ["resnet", "vgg", "transformer"]
```

## Configuration Reference

### Container Section (Required)

Specify the Docker image and optional GPU support:

```toml
[container]
image = "ubuntu:22.04"
use_gpu = false  # Default: false, set to true for GPU support
```

### Scripts Section (Optional)

Customize script names (defaults shown):

```toml
[scripts]
pre = "pre_run.sh"   # Default: "pre_run.sh"
run = "run.sh"       # Default: "run.sh"
post = "post_run.sh" # Default: "post_run.sh"
```

### Job Dependencies (Optional)

```toml
depends_on = ["job1", "job2"]
```

### Input Files (Optional)

Specify which files to copy from dependency job outputs:

```toml
# Copy specific patterns
inputs = ["*.csv", "data/*.json"]

# If omitted or empty, all output files from dependencies are copied
inputs = []
```

### Output Files (Optional)

Specify which files to collect after job completion:

```toml
outputs = ["results/*.json", "*.csv", "models/"]
```

## API Documentation

### `Container`

Container configuration:

```rust
pub struct Container {
    pub image: String,     // Docker image URL
    pub use_gpu: bool,     // Enable GPU support (default: false)
}
```

### `JobMeta`

Main configuration structure:

```rust
pub struct JobMeta {
    pub name: String,
    pub description: String,
    pub container: Container,
    pub scripts: Scripts,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub depends_on: Vec<String>,
    pub params: HashMap<String, ParamDefinition>,
}
```

### Methods

- `JobMeta::load_from_file(path)` - Load configuration from a TOML file
- `JobMeta::save_to_file(path)` - Save configuration to a TOML file
- `JobMeta::validate_params(params)` - Validate JSON parameters against TOML definitions
- `JobMeta::generate_default_params()` - Generate default parameter values as JSON

### Parameter Storage

Parameter *definitions* are stored in TOML format (in `job.toml`), while parameter *values* are stored in JSON format:

- Job parameters: `params.json`
- Workflow parameters: `global_params.json`

```rust
use job_config::params::{JobParams, load_job_params, save_job_params};

// Load job parameters from JSON
let params = load_job_params("params.json")?;

// Save job parameters to JSON
save_job_params("params.json", &params)?;
```

## Error Handling

```rust
use job_config::job::{JobMeta, JobError};

match JobMeta::load_from_file("job.toml") {
    Ok(meta) => {
        // Use meta
    }
    Err(JobError::FileNotFound(path)) => {
        eprintln!("Config file not found: {}", path);
    }
    Err(JobError::InvalidToml(err)) => {
        eprintln!("Failed to parse config: {}", err);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## License

MIT License - see LICENSE file for details.

## Repository

https://github.com/chiral-data/silva
