# Silva TUI Installation Script for Windows
# Usage: iwr -useb https://raw.githubusercontent.com/chiral-data/silva/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

# Repository information
$REPO = "chiral-data/silva"
$BINARY_NAME = "silva"

# Colors
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

function Write-Success { Write-ColorOutput Green $args }
function Write-Info { Write-ColorOutput Cyan $args }
function Write-Warning { Write-ColorOutput Yellow $args }
function Write-Error { Write-ColorOutput Red $args }

# Detect architecture
function Get-Architecture {
    $arch = [System.Environment]::GetEnvironmentVariable("PROCESSOR_ARCHITECTURE")
    switch ($arch) {
        "AMD64" { return "x86_64" }
        "ARM64" { return "aarch64" }
        default {
            Write-Error "Error: Unsupported architecture: $arch"
            exit 1
        }
    }
}

# Get latest release
function Get-LatestRelease {
    Write-Info "Fetching latest release..."

    try {
        $response = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
        $version = $response.tag_name

        if (-not $version) {
            Write-Error "Error: Could not fetch latest release"
            exit 1
        }

        Write-Success "Latest version: $version"
        return $version
    }
    catch {
        Write-Error "Error: Failed to fetch latest release"
        Write-Error $_.Exception.Message
        exit 1
    }
}

# Download and install
function Install-Silva {
    Write-Success "========================================"
    Write-Success "  Silva TUI Installation"
    Write-Success "========================================"
    Write-Output ""

    $arch = Get-Architecture
    Write-Info "Detected architecture: $arch"
    Write-Output ""

    $version = Get-LatestRelease

    $assetName = "${BINARY_NAME}-windows-${arch}.exe.zip"
    $downloadUrl = "https://github.com/$REPO/releases/download/$version/$assetName"

    Write-Info "Download URL: $downloadUrl"
    Write-Output ""

    # Create temp directory
    $tempDir = Join-Path $env:TEMP "silva-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $tempDir | Out-Null

    try {
        # Download
        $zipPath = Join-Path $tempDir $assetName
        Write-Info "Downloading..."
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath
        Write-Success "Download completed"
        Write-Output ""

        # Extract
        Write-Info "Extracting..."
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

        $binaryPath = Join-Path $tempDir "${BINARY_NAME}.exe"

        if (-not (Test-Path $binaryPath)) {
            Write-Error "Error: Binary not found after extraction"
            exit 1
        }

        # Determine installation directory
        $installDir = Join-Path $env:LOCALAPPDATA "Programs\Silva"

        if (-not (Test-Path $installDir)) {
            New-Item -ItemType Directory -Path $installDir | Out-Null
        }

        $installedPath = Join-Path $installDir "${BINARY_NAME}.exe"

        Write-Info "Installing to $installedPath..."
        Copy-Item $binaryPath $installedPath -Force
        Write-Success "Installation completed"
        Write-Output ""

        # Add to PATH if not already there
        $userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")

        if ($userPath -notlike "*$installDir*") {
            Write-Info "Adding to PATH..."
            [System.Environment]::SetEnvironmentVariable(
                "Path",
                "$userPath;$installDir",
                "User"
            )
            Write-Success "Added to PATH"
            Write-Warning "Please restart your terminal for PATH changes to take effect"
        } else {
            Write-Success "Already in PATH"
        }

        Write-Output ""
        Write-Success "========================================"
        Write-Success "  Installation Complete!"
        Write-Success "========================================"
        Write-Output ""
        Write-Info "Run the application with:"
        Write-Info "    silva"
        Write-Output ""
        Write-Warning "Note: You may need to restart your terminal for PATH changes to take effect"
    }
    finally {
        # Cleanup
        Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# Run installation
Install-Silva
