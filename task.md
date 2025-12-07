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
- [] move the JobParams and WorkflowParams (if any) from job_config/src/job.rs into a seperated file job_config/src/params.rs, please be aware that ParamDefinition uses toml; WorkflowParams and JobParams will use json. File paths changes will be: params.json → params.json, node.json → integrated into job.toml, global_params.json → global_params.json.
  - [] continue to update callers: update the following files according to this design
    1. silva/src/components/workflow/global_params_editor.rs
    2. silva/src/components/workflow/manager.rs
       from job.rs
    3. silva/src/components/workflow/state.rs
- [] update "JobMeata"
  - [] move "use_gpu" into "container"
  - [] remove the option docker file, only use docker image.

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

- some steps will be done by me manally and I will mark those steps.
- [x] means it has been done.
- after the completion of each step, update "CHANGELOG.md" and the "doc/\*.md" and also "job_config/README.md"
