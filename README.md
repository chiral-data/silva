# Silva TUI - Automate Workflows

A terminal interface for managing and running workflows.

## Documentation

- [Getting Started](doc/get_started.md) - Requirements, installation, FAQ
- [Key Bindings and Navigation](doc/navigation.md)
- [Tutorial](doc/tutorial.md) - Your first workflow, from an empty folder to a run
- [Workflow Format](doc/format.md) - The normative spec: layout, order, data flow, params
- [Releasing](doc/releasing.md) - For developers: release process

## Requirements

- Docker (for containerized workflows)

## Installation

### One-line Install

**Linux/macOS:**

```bash
curl -fsSL https://raw.githubusercontent.com/chiral-data/silva/main/install.sh | sh
```

**Windows:**

```Windows Terminal(recommended) or powershell
iwr -useb https://raw.githubusercontent.com/chiral-data/silva/main/install.ps1 | iex
```

The script will:

- Auto-detect your OS and architecture
- Download the latest release
- Install the binary to an appropriate location
- Add to PATH (Windows only)

Re-running it upgrades an existing install, including one whose binary is
currently running: the new binary is staged next to the old one and renamed into
place, so the running process is undisturbed and the next invocation is the new
version.

### Updating

Silva keeps itself up to date and never asks. A patch release is downloaded in
the background, verified, and put in place while the current version carries on
running — it becomes the running version the next time you start silva, which
then says so once:

```
Now running silva v0.5.12, updated from v0.5.11.
```

Nothing changes underneath a workflow: the version that starts a run is the
version that finishes it, whatever gets installed while it runs.

What is and is not installed automatically:

| Situation | Behaviour |
| --- | --- |
| A patch release (`0.5.11` → `0.5.12`) | Verified and installed, active next start |
| A minor or major release (`0.5.11` → `0.6.0`) | Reported, not installed — behaviour is allowed to change across that boundary |
| The release publishes no SHA-256 to verify against | Reported, not installed |
| Silva's own binary is not writable (a system or packaged install) | Reported, not installed |

Every download is checked against the `.sha256` published alongside the release
asset, and a mismatch is discarded rather than installed.

`silva --rollback` restores the binary the last update replaced — it is kept
next to the new one, so this needs no network.

To turn it down or off:

| Setting | Effect |
| --- | --- |
| `SILVA_UPDATE=auto` | The default: check, verify, install |
| `SILVA_UPDATE=notify` | Check and report, install nothing |
| `SILVA_UPDATE=off`, `--no-update` | No check, no network request at all |
| `SILVA_NO_UPDATE_CHECK`, `NO_UPDATE`, `CI` | Same as `off`, unless `SILVA_UPDATE` says otherwise |

The check itself is cached for a day, so it costs roughly one request a day
rather than one per invocation — and none at all when switched off, which
matters when a workflow's network traffic is being accounted for.

### Manual Download

Download pre-built binaries from the [Releases](https://github.com/chiral-data/silva/releases) page:

- Linux: x86_64, ARM64 (WIP)
- macOS: x86_64 (Intel), ARM64 (Apple Silicon)
- Windows: x86_64, ARM64

### Build from Source

```bash
git clone https://github.com/chiral-data/silva.git
cd silva
cargo build --release
./target/release/silva
```

## Navigation

### Switching Tabs

- `←` / `→` or `h` `l` - Switch between Applications, Workflows, and Settings
- `i` - Toggle help popup
- `q` - Quit

### Applications Tab

Browse available bioinformatics applications:

- `↑` `↓` or `j` `k` - Navigate list
- `Enter` or `d` - View details
- `Esc` or `d` - Close details

### Workflows Tab

Run and manage workflows:

- `↑` `↓` or `j` `k` - Select workflow
- `Enter` - Execute workflow
- `d` - View/Close job logs

### Settings Tab

Configure health checks:

- `r` - Refresh health checking status

## Machine-Readable Runs

`silva run <WORKFLOW_PATH> --json` emits the run as newline-delimited JSON on
stdout instead of human-formatted text — one object per line, flushed as it
happens, so a script, a CI job or another tool can follow a run without
scraping prose:

```json
{"event":"workflow","status":"started","workflow":"Protein Pocket Analysis","jobs":["01-download","02-pocket"],"at":"..."}
{"event":"job","job":"01-download","index":0,"status":"pulling_image","at":"..."}
{"event":"log","job":"01-download","index":0,"stream":"stdout","text":"Downloading 3 PDB file(s)","at":"..."}
{"event":"job","job":"01-download","index":0,"status":"completed","error":null,"at":"..."}
{"event":"job","job":"02-pocket","index":1,"status":"failed","error":"Script 'run.sh' failed with exit code 1","at":"..."}
{"event":"workflow","status":"failed","error":"Workflow failed","outputDir":"/tmp/silva-...","at":"..."}
```

Four event types:

| `event` | |
| --- | --- |
| `workflow` | `started`, then `completed` or `failed` with the output folder |
| `job` | phase changes (`pulling_image`, `building_image`, `creating_container`, `running`), then exactly one terminal `completed` or `failed`; jobs an aborted run never reached are reported as `skipped` rather than omitted |
| `log` | one container output line, attributed to its job and `stream` |
| `note` | silva's own diagnostics, which would otherwise be bare text on the stream |

Every line on stdout is a JSON object, including diagnostics and warnings, so a
consumer never has to skip unparseable lines — an automatic update reports
itself as a `note` event like any other diagnostic, and may arrive after the
terminal `workflow` event. The process exit code is unchanged: `0` on success,
`1` on failure.

## Checking a Workflow

`silva validate <WORKFLOW_PATH>` checks a workflow folder without running it —
no Docker daemon, no containers, no network:

```bash
$ silva validate ./workflow-007
Protein Pocket Analysis: 3 job(s) ok
Execution order: 01-download -> 02-pocket -> 03-visualize
```

It parses `workflow.toml` and every `job.toml`, resolves the dependency graph,
validates `global_params.json` and each `params.json` against their `[params]`
definitions, and applies the same conventions a run applies (no install commands
in scripts, no cross-node `../` references, every `run` script present,
`input_files/` present when dependency-free jobs exist), and checks that every
job's `inputs` can be fed by its dependencies' `outputs`.

The exit code is `0` when the folder is sound and non-zero when it is not, so it
works as a CI gate on a repository of workflows. `--json` emits the report as
structured data:

```bash
$ silva validate --json ./broken
{
  "valid": false,
  "workflow": "Broken",
  "jobs": ["01-download", "02-pocket"],
  "order": [],
  "findings": [
    {
      "kind": "dependency",
      "file": ".chiral/workflow.toml",
      "job": "02-pocket",
      "message": "Job '02-pocket' depends on '01-fetch', which is not a job folder here."
    }
  ]
}
```

`kind` is one of `workflow`, `job`, `dependency`, `ports`, `params`, `script`
or `inputs`.

## Workflows

A workflow is a folder of jobs. Each job runs its scripts in a Docker container, and `.chiral/workflow.toml` says which jobs wait for which. Files pass from one job to the next through the jobs' `outputs` and `inputs`.

- **Writing one:** follow the [tutorial](doc/tutorial.md). It builds a two-job workflow and runs it.
- **The rules:** [doc/format.md](doc/format.md) is the spec. Its keys come from the JSON Schemas in [`schema/`](schema/), which `silva schema workflow` and `silva schema job` also print.
- **Examples:** [collab-workflows](https://github.com/chiral-data/collab-workflows) has more than thirty real workflows.
