# Silva TUI - Automate Workflows

A terminal interface for managing and running bioinformatics workflows.

## Installation

### One-line Install

**Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/YOUR_USERNAME/research-silva/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
iwr -useb https://raw.githubusercontent.com/YOUR_USERNAME/research-silva/main/install.ps1 | iex
```

The script will:
- Auto-detect your OS and architecture
- Download the latest release
- Install the binary to an appropriate location
- Add to PATH (Windows only)

### Manual Download

Download pre-built binaries from the [Releases](https://github.com/YOUR_USERNAME/research-silva/releases) page:
- Linux: x86_64, ARM64
- macOS: x86_64 (Intel), ARM64 (Apple Silicon)
- Windows: x86_64, ARM64

### Build from Source

```bash
git clone https://github.com/YOUR_USERNAME/research-silva.git
cd research-silva
cargo build --release
./target/release/research-silva
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

## Running Workflows

1. Navigate to the **Workflows** tab using `→`
2. Select a workflow with `↑` / `↓`
3. Press `Enter` to execute
4. Press `d` to view logs while running

## Requirements

- Docker (for containerized workflows)
