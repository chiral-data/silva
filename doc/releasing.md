# Release Guide

This document explains how to create a new release for Silva TUI.

## Prerequisites

1. Ensure all changes are committed and pushed to the main branch
2. All CI checks should be passing
3. Update the version in `Cargo.toml`

## Creating a Release

### 1. Update Version

Edit `Cargo.toml`:

```toml
[package]
name = "silva"
version = "1.0.0"  # Update this version
edition = "2024"
```

### 2. Commit and Tag

```bash
# Commit the version change
git add Cargo.toml
git commit -m "Bump version to 1.0.0"

# Create a version tag
git tag v1.0.0

# Push both commit and tag
git push origin main
git push origin v1.0.0
```

### 3. Automated Build Process

Once you push the tag, GitHub Actions will automatically:

1. **Create a GitHub Release** with the tag name
2. **Build binaries** for all supported platforms:
   - Linux x86_64
   - Linux ARM64
   - macOS x86_64 (Intel)
   - macOS ARM64 (Apple Silicon)
   - Windows x86_64
   - Windows ARM64

3. **Package binaries**:
   - Linux/macOS: `.tar.gz` archives
   - Windows: `.zip` archives

4. **Upload to GitHub Releases** as release assets

### 4. Verify the Release

1. Go to `https://github.com/chiral-data/silva/releases`
2. Check that the release was created with version tag
3. Verify all 6 platform binaries are attached
4. Test the installation script:
   ```bash
   curl -fsSL https://raw.githubusercontent.com/chiral-data/silva/main/install.sh | sh
   ```

## Installation Scripts

### Unix (Linux/macOS)

The `install.sh` script:

- Auto-detects OS (Linux/macOS) and architecture (x86_64/ARM64)
- Downloads the appropriate release binary
- Installs to `/usr/local/bin` (if writable) or `~/.local/bin`
- Makes the binary executable
- Provides PATH instructions if needed

**Usage:**

```bash
curl -fsSL https://raw.githubusercontent.com/chiral-data/silva/main/install.sh | sh
```

### Windows

The `install.ps1` script:

- Auto-detects architecture (x86_64/ARM64)
- Downloads the appropriate release binary
- Installs to `%LOCALAPPDATA%\Programs\Silva`
- Adds installation directory to user PATH
- Requires PowerShell

**Usage:**

```powershell
iwr -useb https://raw.githubusercontent.com/YOUR_USERNAME/research-silva/main/install.ps1 | iex
```

## Manual Release (if needed)

If you need to create a release manually:

1. Build for a specific target:

   ```bash
   cargo build --release --target x86_64-unknown-linux-gnu
   ```

2. Package the binary:

   ```bash
   cd target/x86_64-unknown-linux-gnu/release
   tar czf silva-linux-x86_64.tar.gz silva
   ```

3. Create a GitHub release manually and upload the archive

## Troubleshooting

### Release workflow fails

- Check GitHub Actions logs at `https://github.com/chiral-data/silva/actions`
- Ensure `GITHUB_TOKEN` has proper permissions (should be automatic)
- Verify Cargo.toml syntax is valid

### Binary doesn't work on target platform

- Check cross-compilation setup in `.github/workflows/release.yml`
- Test locally with `cargo build --target <target-triple>`
- Verify dependencies are compatible with target platform

### Installation script fails

- Check that the release exists and has the expected asset names
- Verify URL in installation script matches your repository
- Test download URL manually:
  ```bash
  curl -I https://github.com/chiral-data/silva/releases/download/v1.0.0/silva-linux-x86_64.tar.gz
  ```

## Best Practices

1. **Test before tagging**: Build and test locally before creating a release
2. **Semantic versioning**: Use MAJOR.MINOR.PATCH (e.g., 1.0.0, 1.1.0, 2.0.0)
3. **Changelog**: Update CHANGELOG.md with notable changes (optional but recommended)
4. **Release notes**: Add release notes on GitHub after automated release completes
5. **Testing**: Test installation script after each release on multiple platforms
