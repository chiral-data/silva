# Silva Workflow Format 1.0

This is the normative description of the format: what a workflow folder must contain and what silva does with it. To learn the format by building a workflow, start with the [tutorial](tutorial.md).

Each rule below can be checked against the JSON Schema or against a named function in silva's source. Where `silva run` and the TUI behave differently, `silva run` is the reference, and the difference is stated with the issue that owns it. Behaviour marked **unspecified** may change without a format version bump.

## Fields are defined by the schema

The keys of `workflow.toml` and `job.toml` — their types, defaults, and which are required — are defined by two JSON Schemas generated from the structs silva parses with:

- [`schema/workflow.schema.json`](../schema/workflow.schema.json), also printed by `silva schema workflow`
- [`schema/job.schema.json`](../schema/job.schema.json), also printed by `silva schema job`

Each release publishes both as assets. This document does not repeat their field lists. Three schema facts that surprise people:

- `description` is required in both files, not only `name`.
- A `[params.<name>]` block requires all of `type`, `default` and `hint`. `default` accepts any value and is not checked against `type`.
- Unknown keys are ignored, not rejected. A misspelt key is silently dropped.

`schema_version` (in `workflow.toml`) is the format version as `"MAJOR.MINOR"`, independent of silva's own version. Absent means `1.0`. `silva validate` accepts a newer minor version with a note — features newer than the silva build are ignored — and rejects a newer major version and anything malformed (`check_schema_version`). `silva run` does not check it.

## Layout

```
<workflow>/
├── .chiral/workflow.toml
├── global_params.json      # optional
├── input_files/            # required when any job has no dependencies
└── <job>/
    ├── .chiral/job.toml
    ├── params.json         # optional
    └── run.sh              # and any other files the job needs
```

- **The workflow** is a folder. Its `.chiral/workflow.toml` is required by `silva validate`. `silva run` and the TUI fall back to an empty one when it is missing or does not parse, so a broken file silently runs as "no dependencies" (the TUI side is in [#117](https://github.com/chiral-data/silva/issues/117)).
- **A job** is any direct child folder that contains a regular file `.chiral/job.toml` (`JobScanner::scan_jobs`). Only one level is scanned. A job's name is its folder name. The legacy `@job.toml` is not read.
- **`input_files/`** at the workflow root holds the data for jobs that have no dependencies. If any such job exists, the folder must exist, even if empty (`check_input_files_folder`). With no `workflow.toml`, every job counts as having no dependencies.
- **`global_params.json`** and a job's **`params.json`** are flat JSON objects of parameter values (see [Parameters](#parameters)).

In TUI mode, workflows are the folders directly inside `$SILVA_WORKFLOW_HOME`, which defaults to `./home`.

## Order

Defined by `topological_sort_jobs`:

- A job runs after every job listed for it in `[dependencies]`.
- A cycle is an error, and so is a dependency naming a job that does not exist.
- A `[dependencies]` key that is not a job is ignored by `silva run` and reported by `silva validate` (`check_dependency_names`).
- The order among jobs that are ready at the same time is **unspecified**. In particular, a workflow without `[dependencies]` does not run in folder-name order ([#128](https://github.com/chiral-data/silva/issues/128)).

Jobs run one at a time.

## Execution

`silva run` copies the workflow into a fresh folder, `silva-<timestamp>-…` in the system temp directory, and runs the copy. The source folder is never written to. The copy is kept after the run and its path is printed. When a job succeeds, its folder is moved to `@complete/<job>/` in the copy (`move_job_to_complete`).

For each job (`DockerExecutor::run_job`):

- One container is started per image and reused by every job with that image. The copy's root is mounted at `/workspace`, and the container runs as the user and group that own the workflow folder.
- Scripts run in this order: `pre`, `run`, `post`, with defaults `pre_run.sh`, `run.sh` and `post_run.sh`. Each value is a **file name** relative to the job folder, not a command.
- A script is fed to `/bin/bash` with working directory `/workspace/<job>` (`exec_script`). It needs no execute bit, its shebang is ignored, and CRLF line endings are stripped. The image must provide `/bin/bash`.
- A missing `pre` or `post` file is skipped. A missing `run` file is **not checked**: the step does nothing and counts as a success ([#117](https://github.com/chiral-data/silva/issues/117)).
- The first script that exits non-zero fails the job, skips the job's remaining scripts and its output collection, and stops the workflow. Jobs not yet run are reported as skipped.

Two conventions are enforced before anything runs, by `silva run`, the TUI and `silva validate`:

- No install commands in scripts: `pip`/`pip3`/`apt-get`/`apt`/`conda`/`npm` followed by `install`, or `apk add`, on a non-comment line (`check_install_commands`). Dependencies belong in the image.
- No `../` on a non-comment script line (`check_cross_node_references`). Jobs exchange data through `inputs` and `outputs` only.

## Data flow

**Into a job, before it runs** (`copy_input_files_from_dependencies`):

- Files come from the `outputs/` folder of each job **directly** listed for it in `[dependencies]`. Dependencies of dependencies contribute nothing.
- **An empty or missing `inputs` copies everything** in each dependency's `outputs/`. Many existing workflows rely on this.
- Otherwise each `inputs` entry is a glob, and an entry of a dependency's `outputs/` is copied when its **name** matches one. Matching is on the name only, so a pattern containing `/` never matches ([#130](https://github.com/chiral-data/silva/issues/130)). A matched folder is copied whole.
- When two dependencies provide the same name, the one listed first in `[dependencies]` wins, and the other is skipped with a warning.
- Files land in the job's `inputs/` folder.
- A pattern that matches nothing is ignored silently, and the job still runs. `silva validate` does not catch it yet ([#117](https://github.com/chiral-data/silva/issues/117)).

Jobs with no dependencies instead receive the contents of the workflow's `input_files/`, also in their `inputs/` folder (`copy_input_files_to_dependency_free_jobs`).

In the TUI, inputs land in the job folder itself rather than in `inputs/`, and `input_files/` is not copied ([#129](https://github.com/chiral-data/silva/issues/129)).

**Out of a job, after it succeeds** (`collect_output_files`):

- `outputs` is optional. Each entry is a bash glob evaluated in the job folder, and every match is copied into the job's `outputs/` folder. Paths are flattened: `results/a.json` becomes `outputs/a.json`, and same-named files overwrite each other ([#130](https://github.com/chiral-data/silva/issues/130)).
- An empty `outputs` collects nothing, but a script may write into `outputs/` directly. Whatever is in `outputs/` when the job ends is what its dependents receive.
- A failed collection is a warning and does not fail the job.

## Parameters

A parameter is declared as `[params.<name>]` in `workflow.toml` (shared by all jobs) or in a `job.toml` (that job only). Its value reaches every script as the environment variable `PARAM_<NAME>`, where `<NAME>` is the name upper-cased and otherwise unchanged. Strings are passed as they are. Numbers and booleans are written out, and arrays and objects as JSON.

Values come from `global_params.json` at the workflow root and from each job's `params.json`. Every key in `global_params.json` must be declared in `workflow.toml`, and every key in a `params.json` in that job's `job.toml`. `silva validate` reports undeclared keys as `Unknown parameter`. `silva run` does not check them.

The container environment is built in this order, later entries overriding earlier ones (`DockerExecutor::run_job`):

1. `global_params.json`
2. the job's own values
3. host variables named in `env_passthrough`
4. `-e`/`--env` values

Declared defaults are only partly used at run time today ([#118](https://github.com/chiral-data/silva/issues/118)):

- A workflow-level `default` is never used. A value missing from `global_params.json` produces no `PARAM_` variable.
- For a job, `silva run` uses the declared defaults only when `params.json` is absent. A `params.json` that names some keys gets no defaults for the others.
- The TUI uses no defaults.

## Environment

**`env_passthrough`** in `workflow.toml` lists host environment variables (set in the shell running `silva`) to forward into every job's container. This lets a workflow take API keys without writing them into `global_params.json`. A listed variable that is not set is skipped silently.

**`-e KEY=VALUE`** (`silva run` only) injects a variable, unprefixed, into every job for that run. It is not limited by `env_passthrough`. A malformed entry, with no `=`, fails before any container starts.

```bash
silva run workflows/my-workflow -e RUN_MODE=use_gpu -e FOO=bar
```

**GPU.** If the image declares CUDA or ROCm environment variables and the host has a matching runtime (NVIDIA Container Toolkit, or AMD `/dev/kfd`), GPU access is enabled. Otherwise the container runs on CPU.

### `RUN_MODE=use_dok` bundling (`silva run` only)

Some workflows dispatch to a remote GPU cloud (Sakura's 高火力 DOK managed-container API) instead of computing in the container. They do it through a `run_dok.sh` script that the job's `run.sh` calls when `RUN_MODE=use_dok`. DOK's task API has no bind-mount or upload endpoint — it only accepts `image`, `command` and `environment` — so `run_dok.sh` needs a presigned URL (`DOK_BUNDLE_URL`) to a tar.gz of the job's folder (its scripts and its merged `inputs/`), which it cannot produce itself.

When silva sees `RUN_MODE=use_dok` for a job, after merging `PARAM_*`, `env_passthrough` and `-e`, it prepares this automatically:

1. It tars the job's folder, excluding `outputs/`.
2. It submits a small prep task to DOK that deposits the payload as a DOK artifact.
3. It injects the resulting presigned `DOK_BUNDLE_URL` into the job's environment before launching it.

This needs silva's own `SAKURA_ACCESS_TOKEN` and `SAKURA_ACCESS_TOKEN_SECRET` from silva's host environment. These are separate from whatever is forwarded into the container for `run_dok.sh`:

```bash
export SAKURA_ACCESS_TOKEN=...
export SAKURA_ACCESS_TOKEN_SECRET=...
silva run workflows/my-workflow \
  -e RUN_MODE=use_dok \
  -e DOK_PLAN=v100-32gb \
  -e SAKURA_ACCESS_TOKEN=$SAKURA_ACCESS_TOKEN \
  -e SAKURA_ACCESS_TOKEN_SECRET=$SAKURA_ACCESS_TOKEN_SECRET
```

## What `silva validate` checks beyond the schema

`silva validate <workflow>` (`validate_workflow`) needs no Docker and no network. It reports every finding rather than stopping at the first:

1. The path is a folder, and `.chiral/workflow.toml` exists and parses. If either fails, validation stops here.
2. `schema_version` is acceptable (`check_schema_version`).
3. At least one job exists, and every `job.toml` parses.
4. Every `[dependencies]` key and value names a job (`check_dependency_names`). If so, the graph has no cycle.
5. `global_params.json` and each `params.json` parse and declare no unknown keys.
6. The install-command, `../` and `input_files/` conventions above.

It does not check that script files exist, that `inputs` can be satisfied by upstream `outputs` ([#117](https://github.com/chiral-data/silva/issues/117)), that images exist, that defaults match their types ([#118](https://github.com/chiral-data/silva/issues/118)), or that every declared param has a value.

## Exit codes

| Command | Code | Meaning |
| --- | --- | --- |
| `silva run` | `0` | every job succeeded |
| | `1` | a job failed, a check failed, no jobs were found, a `-e` value was malformed, or the run was cancelled with one Ctrl-C |
| | `130` | cancelled with a second Ctrl-C |
| `silva validate` | `0` | no findings |
| | `1` | at least one finding |
