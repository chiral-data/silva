# Getting Started

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

## FAQ

- Q: the emojis do not show correctly under Windows.
  - A: we recommend using Windows Terminal instead of PowerShell. To install Windows Terminal, run `winget install Microsoft.WindowsTerminal`.
