# Tasks: configuration unification

## Steps

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
