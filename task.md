# Tasks: configuration unification

## Steps

- [x] merge the struct "JobConfig" and the struct "NodeMetadata" from "job_config/config.rs" to a new struct "JobMeta", save the new struct to a sperate file "job_config/job.rs", use TOML.
  1. Created job_config/src/job.rs with the new JobMeta struct that merges:
     - JobConfig fields: container, scripts, use_gpu, inputs, outputs, depends_on
     - NodeMetadata fields: name, description, params
     - All using TOML format with toml::Value for parameter values
  2. Updated lib.rs to export the new job module
  3. Updated CHANGELOG.md with the change
  4. Updated job_config/README.md with new documentation
- [x] update callers
  - silva/src/components/workflow/job.rs - Changed load_config() to load_meta(), uses JobMeta
  - silva/src/components/workflow/params_editor.rs - Uses JobMeta and toml::Value
  - silva/src/components/docker/executor.rs - Uses JobMeta, updated param value matching to toml::Value
  - silva/src/components/docker/state.rs - Updated load_config() calls to load_meta()
  - silva/examples/docker_executor.rs - Uses JobMeta
- [x] move the JobParams and WorkflowParams (if any) from job_config/src/job.rs into a seperated file job_config/src/params.rs, please be aware that ParamDefinition uses toml; WorkflowParams and JobParams will use json. File paths changes will be: params.json → params.json, node.json → integrated into job.toml, global_params.json → global_params.json.
  - job_config/src/params.rs - New module for JSON-based parameter storage with:
    - JobParams type alias (HashMap<String, serde_json::Value>)
    - WorkflowParams type alias (HashMap<String, serde_json::Value>)
    - ParamsError enum for error handling
    - load_job_params() / save_job_params() - JSON file I/O for job params
    - load_workflow_params() / save_workflow_params() - JSON file I/O for workflow params
    - toml_to_json() / json_to_toml() - Value conversion utilities
  - Design:
    - Parameter definitions (ParamDefinition) use TOML format (stored in job.toml)
    - Parameter values (JobParams, WorkflowParams) use JSON format (stored in params.json, global_params.json)
- [x] continue to update callers: update the following files according to this design
  - job_config/src/job.rs - Removed JobParams, updated validate_params() and generate_default_params() to work with
    JSON values
  - job_config/src/workflow.rs - Updated to use params from params.rs
  - job_config/src/lib.rs - Added params module export
  - silva/src/components/workflow/job.rs - Updated to use params.json (JSON format)
  - silva/src/components/workflow/manager.rs - Updated to use global_params.json (JSON format)
  - silva/src/components/workflow/params_editor.rs - Updated to use JSON values
  - silva/src/components/workflow/global_params_editor.rs - Updated to use JSON values
  - silva/src/components/docker/executor.rs - Updated to use JSON params
  - silva/examples/docker_executor.rs - Updated imports
  - CHANGELOG.md - Updated with configuration unification changes
  - job_config/README.md - Updated documentation
- [x] update "JobMeta"
  - [x] move "use_gpu" into "container"; remove the option docker file, only use docker image.
    - Changes to Container:
      - Changed from an enum (DockerImage/DockerFile) to a struct with image and use_gpu fields
      - Added Container::new() and Container::with_gpu() constructor methods
      - Removed Dockerfile support (only Docker images are now supported)
    - Changes to JobMeta:
      - Removed use_gpu field (now part of Container)
  - [x] update callers and doc
    - job_config/src/job.rs - Simplified Container struct, updated JobMeta, updated tests
    - silva/src/components/docker/executor.rs - Updated to use new Container structure
    - silva/examples/docker_executor.rs - Updated to use new Container structure
    - CHANGELOG.md - Added documentation for the changes
    - job_config/README.md - Updated documentation with new TOML format
- [] add more tests.
- [] merge params_editor and global_params_editor.rs

- [] move the struct "WorkflowMetadata" from "job_config/config.rs" to a sperate file "job_config/workflow.rs", change it from json to toml.
  - this step is partially done with the status:
    - Created job_config/src/workflow.rs with WorkflowMetadata struct using TOML format
    - Removed WorkflowMetadata and related functions from config.rs
    - Updated lib.rs to export the workflow module
    - Updated imports in silva/src/components/workflow/ files
    - However, the code doesn't compile yet because:
      1. WorkflowParams now uses toml::Value instead of serde_json::Value
      2. Several places in the codebase expect serde_json::Value for parameter handling
      3. The generate_default_params() and validate_params() methods need updating
    - The main design decision needed: how to handle the conversion between serde_json::Value (used by ParamDefinition for defaults/validation) and toml::Value (used by WorkflowParams for TOML file storage).

## Rules for each step

- Some steps will be done manally and they will be marked and highlighted.
- [x] means it has been done.
- After the completion of each step, update "CHANGELOG.md" and the "doc/\*.md" and also "job_config/README.md"
