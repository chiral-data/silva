# How to Create a Silva Workflow from Scratch

## Workflow Structure Overview

A Silva workflow ([example](https://github.com/chiral-data/collab-workflows/tree/main/workflows/workflow-003)) consists of:

- Workflow configuration (.chiral/workflow.json) - defines workflow metadata and global parameters
- Global parameters (global_params.json) - default values automatically applied to all jobs
- Job directories - each containing job configuration, scripts, and optional job-specific parameters

## Step-by-Step Instructions

### 1. Create the Workflow Directory Structure

```bash
mkdir -p my-workflow/.chiral
cd my-workflow
```

### 2. Create Workflow Configuration

Create .chiral/workflow.json:

```json
{
  "name": "my-workflow",
  "description": "Brief description of what this workflow does",
  "params": {
    "param1": ["string", "default_value", "Description of parameter 1"],
    "param2": ["string", "value2", "Description of parameter 2"]
  }
}
```

Parameter format: [type, default_value, description]

### 3. Create Global Parameters File

Create global_params.json - these parameters are automatically applied to all jobs in the workflow:

```json
{
  "param1": "default_value",
  "param2": "value2"
}
```

### 4. Create Job Directories

Jobs execute in dependency order (or alphabetically if no dependencies). Use numbered prefixes for clarity:

```bash
mkdir -p 1-first-job/.chiral
mkdir -p 2-second-job/.chiral
mkdir -p 3-third-job/.chiral
```

### 5. Configure Each Job

For each job directory, create two configuration files:

#### a.Job Metadata: .chiral/node.json

```json
{
  "name": "1 First Job",
  "description": "Description of what this job does",
  "params": {}
}
```

#### b. Job Configuration: .chiral/job.toml

```toml
# Optional: specify dependencies

depends_on = ["previous-job-name"]

# Optional: input patterns (files to copy from dependencies)

inputs = ["*.pdb", "data/*.csv"]

# Optional: output patterns (files to preserve after execution)

outputs = ["*.sdf", "results"]

[container]
docker_image = "your-docker-image:tag"

[scripts]

# Scripts are optional and have these defaults:

# run = "run.sh"

# pre = "pre_run.sh"

# post = "post_run.sh"
```

#### Common Job.toml Patterns

Simple standalone job:

```toml
outputs = ["results.txt"]

[container]
docker_image = "python:3.11-slim"

Job with dependency:
depends_on = ["1-preprocessing"]
inputs = ["*.csv"]
outputs = ["model.pkl"]

[container]
docker_image = "python:3.11-slim"

Job with multiple dependencies:
depends_on = ["2-prep", "3-extract"]
inputs = ["*.pdb", "*.sdf"]
outputs = ["docking_results"]

[container]
docker_image = "your-image:tag"
```

Key points:

- There must be a `run.sh` script while `pre_run.sh` and `post_run.sh` are optional.
- inputs: Glob patterns for files to copy from dependency outputs (omit to copy all)
- outputs: Glob patterns for files to collect after job execution
- Must specify either docker_image

### 6. Create Job Scripts

Create `run.sh` in each job directory:

```bash
#!/bin/bash
set -e

# Your job logic here

python3 your_script.py

Make scripts executable:
```

And add execution permission:

```
chmod +x \*/run.sh
```

### 7. Access Parameters in Scripts

Global parameters from `global_params.json` are automatically available as environment variables in all jobs with the `PARAM_` prefix:

Python example:

```python
import os

pdb_id = os.getenv("PARAM_PDB_ID")
ligand_name = os.getenv("PARAM_LIGAND_NAME")

Bash example:
echo "PDB ID: $PARAM_PDB_ID"
echo "Ligand: $PARAM_LIGAND_NAME"
```

### 8. Job-Specific Parameters

Create `params.json` in a job directory to add parameters for that specific job only:

```json
{
  "job_specific_param": "value_for_this_job_only",
  "param1": "override_global_value"
}
```

Important:

- Global parameters apply to all jobs automatically
- Job-specific params.json only affects that individual job

## Complete Example Structure

```
my-workflow/
  │
  ├── .chiral/
  │   └── workflow.json          # Workflow metadata and parameter definitions
  │
  ├── global_params.json         # Parameters applied to ALL jobs
  │
  ├── 1-download-data/
  │   ├── .chiral/
  │   │   ├── node.json         # Job metadata
  │   │   └── job.toml          # Job configuration
  │   ├── run.sh                # Main script
  │   ├── download.py           # Python script
  │   └── params.json           # Optional: parameters for this job ONLY
  │
  ├── 2-process-data/
  │   ├── .chiral/
  │   │   ├── node.json
  │   │   └── job.toml
  │   └── run.sh
  │
  └── 3-generate-report/
      ├── .chiral/
      │   ├── node.json
      │   └── job.toml
      └── run.sh
```

## Parameter Scope Example

### global_params.json:

```json
{
"pdb_id": "4OHU", # Available in ALL jobs as $PARAM_PDB_ID
"ligand_name": "2TK" # Available in ALL jobs as $PARAM_LIGAND_NAME
}
```

### 1-download-data/params.json:

```json
{
"download_format": "pdb" # Only available in job 1 as $PARAM_DOWNLOAD_FORMAT
}
```

### Workflow-003 Dependency Chain Example

```
1-download-pdb
↓ (outputs: _.pdb)
2-protein-preparation
↓ (inputs: _.pdb, outputs: _.pdb)
├→ 3-ligand-extraction
│ ↓ (inputs: _.pdb, outputs: _.sdf)
│ 4-ligand-modification
│    ↓ (inputs: _.sdf, outputs: ligand_library)
└──→ 5-in-silico-screening
     ↓ (inputs: \*.pdb + ligand_library, outputs: docking_results)
     6-report
     (inputs: docking_results, outputs: results)
```

## Running Your Workflow

1. Place the workflow directory in $SILVA_WORKFLOW_HOME
2. Launch Silva TUI
3. Navigate to the Workflows tab
4. Select your workflow and press Enter

## Best Practices

1. Use descriptive job names with numbered prefixes (1-, 2-, 3-)
2. Always start scripts with set -e to fail on errors
3. Specify exact Docker image tags (e.g., python:3.11-slim not python:latest)
4. Use outputs patterns to preserve important results
5. Declare dependencies explicitly in job.toml for data flow clarity
6. Add logging with echo statements to track progress
7. Put common parameters in global_params.json to avoid repetition across jobs
