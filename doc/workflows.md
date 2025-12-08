# Workflows

## Usage Overview

The Silva workflow execution system allows you to define and run multi-step workflows using Docker containers. Each workflow consists of multiple jobs that execute sequentially, with each job running in its own Docker container.

## Directory Structure

### Workflow Home Directory

The workflow home directory is configurable via the `SILVA_WORKFLOW_HOME` environment variable. If not set, it defaults to `./home`.

```bash
export SILVA_WORKFLOW_HOME=/path/to/workflows
```

A collection of workflows can be found in [this repository](https://github.com/chiral-data/collab-workflows).

### Workflow and Job Structure

```
$SILVA_WORKFLOW_HOME/
├── workflow_1/
│   ├── .chiral/
│   │   └── workflow.toml       # Workflow metadata, dependencies, global params
│   ├── global_params.json      # Global parameter values
│   ├── job_1/
│   │   ├── .chiral/
│   │   │   └── job.toml        # Job configuration
│   │   ├── params.json         # Job parameter values
│   │   ├── run.sh
│   │   └── outputs/            # Output files collected after execution
│   ├── job_2/
│   │   ├── .chiral/
│   │   │   └── job.toml
│   │   └── run.sh
│   └── job_3/
│       ├── .chiral/
│       │   └── job.toml
│       └── run.sh
├── workflow_2/
│   └── ...
```

**Note**: Legacy `@job.toml` in the job root is still supported but `.chiral/job.toml` is preferred.

## Workflow Configuration

Workflows can have a `.chiral/workflow.toml` file that defines workflow-level metadata, job dependencies, and global parameters:

```toml
name = "My Workflow"
description = "A multi-step data processing workflow"

# Job dependencies (defines execution order)
[dependencies]
02_transform = ["01_extract"]
03_load = ["02_transform"]

# Global parameters (available to all jobs)
[params.input_source]
type = "string"
default = "production"
hint = "Data source environment"

[params.batch_size]
type = "integer"
default = 1000
hint = "Number of records per batch"
```

## Job Configuration

Each job requires a `.chiral/job.toml` configuration file that defines:

### Container Configuration

Specify a Docker image for the job:

```toml
[container]
image = "ubuntu:22.04"
use_gpu = false  # optional, defaults to false

[scripts]
run = "run.sh"
```

### Script Configuration

Scripts are optional and have default values:

- `pre`: Pre-execution script (default: `pre_run.sh`), optional
- `run`: Main execution script (default: `run.sh`)
- `post`: Post-execution script (default: `post_run.sh`), optional

**Note 1**: The job folder is mounted as `/workspace` inside the container, and scripts are executed from this directory.

**Note 2**: If pre-execution script and post-execution script are not specified, they will be ignored.

### Job Parameters

Jobs can define parameters that are injected as environment variables:

```toml
name = "Data Processor"
description = "Processes input data files"
inputs = ["*.csv"]
outputs = ["processed_*.csv", "report.json"]

[container]
image = "python:3.11-slim"

[scripts]
run = "process_data.sh"

# Parameter definitions
[params.batch_size]
type = "integer"
default = 100
hint = "Number of records per batch"

[params.output_format]
type = "enum"
default = "json"
hint = "Output file format"
enum_values = ["json", "csv", "parquet"]

[params.verbose]
type = "boolean"
default = false
hint = "Enable verbose logging"
```

**Supported Parameter Types:**
- `string`: Text values
- `integer`: Whole numbers
- `float`: Decimal numbers
- `boolean`: true/false values
- `enum`: Choice from predefined values (requires `enum_values` list)
- `file`: File path
- `directory`: Directory path
- `array`: List of values

Parameters are injected as environment variables with `PARAM_` prefix:
- `batch_size` → `PARAM_BATCH_SIZE`
- `output_format` → `PARAM_OUTPUT_FORMAT`

### Input/Output Data Flow

Jobs can specify input and output file patterns for automatic data transfer:

```toml
inputs = ["*.csv", "features/*.json"]
outputs = ["model.pkl", "metrics/*.txt"]
```

- `inputs`: Glob patterns for files to copy from dependency outputs
  - Files are copied from each dependency's `outputs/` folder before execution
  - If empty or omitted, **all** output files from dependencies are copied

- `outputs`: Glob patterns for files to collect after job execution
  - Matching files are copied to an `outputs/` folder in the job directory
  - Files become available to jobs that depend on this one

**Example Multi-Job Workflow with Dependencies:**

```
ml_pipeline/
├── .chiral/
│   └── workflow.toml     # Dependencies defined here
├── 01_data_prep/
│   ├── .chiral/
│   │   └── job.toml
│   └── prepare.sh        # Outputs: train.csv, test.csv
├── 02_feature_eng/
│   ├── .chiral/
│   │   └── job.toml
│   └── features.sh       # Inputs: *.csv, Outputs: features.json
└── 03_train_model/
    ├── .chiral/
    │   └── job.toml
    └── train.sh          # Inputs: features.json, Outputs: model.pkl
```

**.chiral/workflow.toml:**

```toml
name = "ML Pipeline"
description = "Train a machine learning model"

[dependencies]
02_feature_eng = ["01_data_prep"]
03_train_model = ["02_feature_eng"]
```

**01_data_prep/.chiral/job.toml:**

```toml
name = "Data Preparation"
outputs = ["train.csv", "test.csv"]

[container]
image = "python:3.11-slim"

[scripts]
run = "prepare.sh"
```

**02_feature_eng/.chiral/job.toml:**

```toml
name = "Feature Engineering"
inputs = ["*.csv"]
outputs = ["features.json"]

[container]
image = "python:3.11-slim"

[scripts]
run = "features.sh"
```

**03_train_model/.chiral/job.toml:**

```toml
name = "Model Training"
inputs = ["features.json"]
outputs = ["model.pkl", "metrics.txt"]

[container]
image = "python:3.11-slim"

[scripts]
run = "train.sh"
```

**How It Works:**

1. Job dependencies are defined in `workflow.toml` (not in individual job.toml files)
2. Jobs execute in dependency order (topological sort)
3. Before a job runs, input files from dependencies are copied to the job directory
4. After successful execution, output files are collected to the `outputs/` folder
5. The workflow displays execution order at startup: `01_data_prep → 02_feature_eng → 03_train_model`

## Creating Workflows

### 1. Create Workflow Directory

```bash
mkdir -p $SILVA_WORKFLOW_HOME/my_workflow
```

### 2. Create Job Directories

Job directories should be named in a way that ensures correct execution order (jobs are executed alphabetically by name):

```bash
mkdir -p $SILVA_HOME_DIR/my_workflow/01_preprocessing
mkdir -p $SILVA_HOME_DIR/my_workflow/02_analysis
mkdir -p $SILVA_HOME_DIR/my_workflow/03_reporting
```

### 3. Create Job Configurations

For each job, create a `.chiral/job.toml` file:

```bash
mkdir -p $SILVA_HOME_DIR/my_workflow/01_preprocessing/.chiral
cat > $SILVA_HOME_DIR/my_workflow/01_preprocessing/.chiral/job.toml << 'EOF'
name = "Preprocessing"
outputs = ["processed_data.csv"]

[container]
image = "python:3.11-slim"

[scripts]
run = "preprocess.sh"
EOF
```

### 4. Create Scripts

Create the required scripts (must be executable):

```bash
cat > $SILVA_HOME_DIR/my_workflow/01_preprocessing/preprocess.sh << 'EOF'
#!/bin/bash
set -e

echo "Starting preprocessing..."
python3 -c "print('Preprocessing complete!')"
EOF

chmod +x $SILVA_HOME_DIR/my_workflow/01_preprocessing/preprocess.sh
```

## Workflow Execution Behavior

### Sequential Execution

- Jobs execute in **dependency order** (topological sort) when dependencies are specified
- For workflows without dependencies, jobs execute in **alphabetical order** by folder name
- Each job runs to completion before the next job starts
- Job folder is mounted as `/workspace` in the container
- Scripts execute with `/workspace` as the working directory
- Input files from dependencies are copied to the job directory before execution
- Output files are collected to the `outputs/` folder after successful execution

### Script Execution Order

For each job:

1. **Pre-run script** (if specified)
2. **Main run script** (required)
3. **Post-run script** (if specified)

If any script returns a non-zero exit code, the job fails and the workflow stops.

### Failure Handling

- If a job fails, the workflow stops immediately
- Remaining jobs are not executed
- The failed job name is recorded in the execution result
- Logs up to the point of failure are retained

## Example Workflows

### Example 1: Data Processing Pipeline

```
data_pipeline/
├── .chiral/
│   └── workflow.toml
├── 01_extract/
│   ├── .chiral/
│   │   └── job.toml
│   └── extract.sh
├── 02_transform/
│   ├── .chiral/
│   │   └── job.toml
│   └── transform.py
└── 03_load/
    ├── .chiral/
    │   └── job.toml
    └── load.sh
```

**.chiral/workflow.toml**:

```toml
name = "Data Pipeline"

[dependencies]
02_transform = ["01_extract"]
03_load = ["02_transform"]
```

**01_extract/.chiral/job.toml**:

```toml
name = "Extract Data"
outputs = ["raw_data.csv"]

[container]
image = "alpine:latest"

[scripts]
run = "extract.sh"
```

**02_transform/.chiral/job.toml**:

```toml
name = "Transform Data"
inputs = ["*.csv"]
outputs = ["transformed_data.csv"]

[container]
image = "python:3.11-slim"

[scripts]
run = "transform.py"
```

### Example 2: Testing Pipeline

```
test_suite/
├── .chiral/
│   └── workflow.toml
├── job_1_unit_tests/
│   ├── .chiral/
│   │   └── job.toml
│   └── run_tests.sh
├── job_2_integration_tests/
│   ├── .chiral/
│   │   └── job.toml
│   └── run_tests.sh
└── job_3_e2e_tests/
    ├── .chiral/
    │   └── job.toml
    └── run_tests.sh
```

All jobs use the same configuration pattern:

```toml
name = "Unit Tests"

[container]
image = "node:20-alpine"

[scripts]
pre = "npm install"
run = "npm test"
```

## Troubleshooting

### Workflow Not Appearing in List

- Verify the workflow directory exists in `$SILVA_WORKFLOW_HOME`
- Press `r` to refresh the workflow list
- Check that jobs contain `.chiral/job.toml` files (or legacy `@job.toml`)

### Job Configuration Errors

- Verify `.chiral/job.toml` syntax is valid
- Ensure `[container]` section has an `image` field
- Check that script files exist and are executable

### Docker Execution Errors

- Verify Docker daemon is running
- Check that specified Docker images are available
- Review logs in the Docker popup for detailed error messages
- Ensure scripts have correct shebang (`#!/bin/bash`)

### Permission Issues

- Make sure all scripts are executable: `chmod +x script.sh`
- Verify Docker has permission to access mounted volumes

## Advanced Usage

### Sharing Data Between Jobs

**Recommended Approach:** Use dependencies in `workflow.toml` and `inputs`/`outputs` in job configs:

```toml
# .chiral/workflow.toml
[dependencies]
job_2 = ["job_1"]
```

```toml
# job_1/.chiral/job.toml
outputs = ["result.txt", "data.csv"]

[container]
image = "ubuntu:22.04"

[scripts]
run = "process.sh"
```

```toml
# job_2/.chiral/job.toml
inputs = ["*.txt", "*.csv"]  # or omit to copy all outputs

[container]
image = "ubuntu:22.04"

[scripts]
run = "analyze.sh"
```

Files from job_1's outputs are automatically copied to job_2's directory before execution.

**Legacy Approach:** Access other job folders via relative paths:

```bash
#!/bin/bash
# job_1/run.sh - Write output
echo "result data" > /workspace/output.txt

# job_2/run.sh - Read input
cat /workspace/../job_1/output.txt
```

**Note**: The dependency-based approach is preferred as it makes data flow explicit and handles file copying automatically.

### Using Environment Variables

Pass environment variables through Dockerfile:

```dockerfile
FROM ubuntu:22.04
ENV MY_VAR=value
```

Or set them in your script:

```bash
#!/bin/bash
export MY_VAR=value
./my_program
```

### Custom Docker Networks

Currently, each job runs in isolation. For jobs that need to communicate, use file-based data exchange through the workflow directory.

## Best Practices

1. **Name Jobs with Prefixes**: Use numeric prefixes (01*, 02*, 03\_) to ensure correct execution order
2. **Use Set -e**: Always start scripts with `set -e` to fail on errors
3. **Log Verbosely**: Add echo statements to track progress
4. **Test Individually**: Test each job independently before running the full workflow
5. **Keep Jobs Small**: Break complex workflows into smaller, focused jobs
6. **Document Dependencies**: Add README files explaining job purposes and dependencies
7. **Use Specific Tags**: Specify exact Docker image tags (e.g., `ubuntu:22.04` not `ubuntu:latest`)

## Configuration Reference

### Environment Variables

| Variable              | Default  | Description                  |
| --------------------- | -------- | ---------------------------- |
| `SILVA_WORKFLOW_HOME` | `./home` | Workflow home directory path |

### File Names

| File                    | Required | Description                               |
| ----------------------- | -------- | ----------------------------------------- |
| `.chiral/workflow.toml` | Optional | Workflow metadata, dependencies, params   |
| `.chiral/job.toml`      | Yes      | Job configuration file                    |
| `global_params.json`    | Optional | Global parameter values (workflow root)   |
| `params.json`           | Optional | Job parameter values (job directory)      |
| `run.sh`                | Default  | Main execution script (configurable)      |
| `pre_run.sh`            | Default  | Pre-execution script (configurable)       |
| `post_run.sh`           | Default  | Post-execution script (configurable)      |
| `outputs/`              | Auto     | Output files collected after execution    |

### Exit Codes

| Code     | Meaning                  |
| -------- | ------------------------ |
| 0        | Success                  |
| Non-zero | Failure (workflow stops) |

## Support

For issues or questions:

- Check the logs in the Docker popup (press `d`)
- Review test files for examples
- See source code in `src/components/workflow/` and `src/components/docker/`
