# Tutorial: your first workflow

This walks from an empty directory to a two-job workflow that runs. It explains what each file is for as you write it. The rules themselves — what is required, what silva does with each key — are in the [format spec](format.md); this page links to it rather than repeating it.

You need `silva` ([installation](get_started.md)) and a running Docker daemon.

## What we are building

```
hello-silva/
├── .chiral/
│   └── workflow.toml       # the jobs' order and the shared params
├── global_params.json      # values for those params
├── input_files/
│   └── names.txt           # data for the first job
├── 01-count/
│   ├── .chiral/
│   │   └── job.toml        # image and outputs of this job
│   └── run.sh
└── 02-report/
    ├── .chiral/
    │   └── job.toml
    └── run.sh
```

`01-count` counts the lines of `names.txt`. `02-report` turns that count into a one-line report. The second job depends on the first, so it runs after it and receives its outputs.

## 1. The workflow

```bash
mkdir -p hello-silva/.chiral hello-silva/input_files
cd hello-silva
printf 'Ada\nGrace\nKatherine\n' > input_files/names.txt
```

Create `.chiral/workflow.toml`:

```toml
# hello-silva/.chiral/workflow.toml
schema_version = "1.0"
name = "Hello Silva"
description = "Count the names in a file, then report the count"

[dependencies]
02-report = ["01-count"]

[params.greeting]
type = "string"
default = "Hello"
hint = "The word the report starts with"
```

- `[dependencies]` maps a job to the jobs it waits for. A job's name is its folder name.
- `[params.greeting]` declares a parameter every job can read. `type`, `default` and `hint` are all required.
- Scalar keys (`schema_version`, `name`, `description`) must come before the first `[table]`. That is TOML, not silva: anything after a table header belongs to that table.

A run uses the declared `default` unless a file gives a value. To set it for this workflow, use `global_params.json`:

```bash
echo '{ "greeting": "Hello" }' > global_params.json
```

`input_files/` holds the data for the jobs that have no dependencies — here, `01-count`. silva copies its contents into those jobs before they run.

## 2. The first job

```bash
mkdir -p 01-count/.chiral
```

Create `01-count/.chiral/job.toml`:

```toml
# hello-silva/01-count/.chiral/job.toml
name = "Count"
description = "Count the names in the seed file"
outputs = ["count.txt"]

[container]
image = "ubuntu:24.04"
```

- `name`, `description` and `[container]` with an `image` are required.
- `outputs` lists the files to keep. After the job succeeds, silva copies each match into the job's `outputs/` folder, which is what later jobs receive.
- There is no `[scripts]` table, so silva runs `run.sh`.

Create `01-count/run.sh`:

```bash
# hello-silva/01-count/run.sh
set -e
wc -l < inputs/names.txt > count.txt
echo "Counted $(cat count.txt) names"
```

`inputs/names.txt` is there because silva copied `input_files/` into the job's `inputs/` folder. The script is fed to `/bin/bash`, so it needs no `chmod +x` and no shebang — but the image must have bash, which is why this uses `ubuntu` and not `alpine`.

## 3. The second job

```bash
mkdir -p 02-report/.chiral
```

Create `02-report/.chiral/job.toml`:

```toml
# hello-silva/02-report/.chiral/job.toml
name = "Report"
description = "Write a one-line report from the count"
outputs = ["report.txt"]

[container]
image = "ubuntu:24.04"
```

There is no `inputs` key. **An empty or missing `inputs` means "everything"**: silva copies every file in each dependency's `outputs/` into this job's `inputs/`. To take only some of them, list patterns, such as `inputs = ["*.txt"]`.

Create `02-report/run.sh`:

```bash
# hello-silva/02-report/run.sh
set -e
count=$(cat inputs/count.txt)
echo "$PARAM_GREETING: $count names" > report.txt
cat report.txt
```

The `greeting` param arrives as the environment variable `PARAM_GREETING`: the name upper-cased, with a `PARAM_` prefix.

The count is read into a variable first, not inside the `echo`. `set -e` stops the script when an assignment's command fails, but not when a command inside another command's arguments fails, so `echo "$(cat missing)"` would succeed and write a broken report.

## 4. Check it, then run it

```bash
cd ..
silva validate hello-silva
```

```
Hello Silva: 2 job(s) ok
Execution order: 01-count -> 02-report
```

`validate` needs no Docker and no network. It checks what the [JSON Schema](format.md#fields-are-defined-by-the-schema) cannot: that dependency names are real jobs, that there is no cycle, that the params files match their declarations, and that scripts follow the run conventions. Run it before every run and in CI.

```bash
silva run hello-silva
```

Each job's log is printed as it runs. `02-report` prints `Hello: 3 names`, and the run ends with:

```
Workflow completed successfully

Output folder: /tmp/silva-...
```

silva runs a **copy** of the workflow, so `hello-silva/` itself is never written to. In the output folder, each finished job has been moved under `@complete/`:

```bash
cat /tmp/silva-.../@complete/02-report/outputs/report.txt
```

## Where to go next

- **More inputs.** A job with two dependencies receives the outputs of both. If both produce a file with the same name, the first dependency listed wins.
- **Job-only params.** Declare `[params.x]` in a `job.toml` and give its value in that job's `params.json`. Any param `params.json` leaves out takes its declared default.
- **Other scripts.** `[scripts]` takes `pre`, `run` and `post` file names. A missing `pre` or `post` file is skipped.
- **Real examples.** [collab-workflows](https://github.com/chiral-data/collab-workflows) has more than thirty, from a three-job pocket analysis (workflow-007) to multi-stage docking pipelines.

## Troubleshooting

- **The workflow is not listed in the TUI.** It must be a folder directly inside `$SILVA_WORKFLOW_HOME` (default `./home`), and each job must have `.chiral/job.toml`. Press `r` to refresh.
- **`validate` reports a job that will not parse.** The usual cause is a missing `description`, or a scalar key placed after a `[table]`. `silva schema job` prints what is allowed.
- **`Unknown parameter`.** Every key in `global_params.json` must be declared under `[params]` in `workflow.toml`, and every key in a `params.json` under that job's `[params]`.
- **`No 'input_files/' folder found`.** Any job without dependencies needs the workflow's `input_files/` folder to exist, even if it is empty.
- **Install commands are rejected.** A script may not run `pip install`, `apt-get install` and the like; build them into the image instead.
- **`../` is rejected.** A script may not reach into another job's folder. Pass files through `outputs` and `inputs`.
- **Docker errors.** Check the daemon is running and the image tag exists. In the TUI, press `d` for the job's logs.

## Good habits

- Number job folders (`01-`, `02-`), so the layout reads in order. The run order comes only from `[dependencies]`.
- Start scripts with `set -e`, so a failing command fails the job.
- Pin image tags (`ubuntu:24.04`, not `ubuntu:latest`).
- Keep jobs small, and put values shared by several jobs in `global_params.json`.
