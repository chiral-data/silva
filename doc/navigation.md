# Key Bindings and Navigation

## Switching Tabs

- `←` / `→` or `h` `l` - Switch between Applications, Workflows, and Settings
- `i` - Toggle help popup
- `q` - Quit

## Applications Tab

Browse available bioinformatics applications:

- `↑` `↓` or `j` `k` - Navigate list
- `Enter` or `d` - View details
- `Esc` or `d` - Close details

## Workflows Tab

Run and manage workflows:

- `↑` `↓` or `j` `k` - Select workflow
- `Enter` - Execute workflow
- `d` - View/Close job logs

## Settings Tab

Configure health checks:

- `r` - Refresh health checking status

## Running Workflows

1. Navigate to the **Workflows** tab using `→`
2. Select a workflow with `↑` / `↓`
3. Press `Enter` to execute (Docker logs popup opens automatically)
4. Press `d` to view logs while running

### Monitor Progress

The Docker logs popup shows real-time execution logs with:

- Status section displaying workflow name and execution status
- Job progress section with visual indicators:
  - **✓** (green) - Completed job
  - **⟳** (yellow) - Currently running job
  - **⬜** (gray) - Pending job
- Progress counter showing (current/total) jobs

## Keyboard Shortcuts Summary

| Key       | Action                         |
| --------- | ------------------------------ |
| `Enter`   | Launch selected workflow       |
| `↑` / `↓` | Navigate workflows/scroll logs |
| `d`       | Toggle Docker logs popup       |
| `b`       | Scroll logs to bottom          |
| `r`       | Refresh workflow list          |
| `i`       | Toggle help popup              |
| `q`       | Quit application               |
