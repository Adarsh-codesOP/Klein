# GitHub Actions CI/CD Workflow - Klein Release Pipeline

This document explains the automated build and release workflow for Klein.

## Overview

The GitHub Actions workflow (`.github/workflows/release.yml`) automatically builds and releases Klein binaries for multiple platforms whenever a new release is published.

## Triggered Events

The workflow is triggered in two ways:

### 1. **Automatic Release Trigger**
- When you publish a new release on GitHub (via the Releases page)
- Builds happen automatically and binaries are attached to the release

### 2. **Manual Workflow Dispatch**
- You can manually trigger the workflow from GitHub's Actions tab
- Specify a version tag (e.g., `v1.0.0`, or `stable` to update the stable binaries)

## Build Targets

The workflow builds Klein for the following platforms:

| Platform | Binary Name | Target Triple |
|----------|-------------|----------------|
| Windows x86_64 | `klein-windows-x86_64.exe` | `x86_64-pc-windows-msvc` |
| Linux x86_64 | `klein-linux-x86_64` | `x86_64-unknown-linux-gnu` |
| Linux ARM64 | `klein-linux-aarch64` | `aarch64-unknown-linux-gnu` |

## Workflow Steps

### 1. **Build Job** (`build`)
- Runs on multiple OS environments (Windows, Ubuntu)
- Uses a matrix strategy to parallelize builds
- For ARM64 Linux builds, uses the `cross` tool for cross-compilation
- Strips debug symbols from Linux binaries to reduce file size
- Uploads each binary as a release asset

### 2. **Update Install Scripts Job** (`update-install-scripts`)
- Triggered only for `stable` releases
- Updates `install.ps1` and `install.sh` to point to the stable release
- Automatically commits and pushes changes to the repository

### 3. **Update Version Job** (`update-version`)
- Triggered on actual release events (not for `stable` tag)
- Extracts version from git tag (e.g., `v0.2.0` → `0.2.0`)
- Updates `Cargo.toml` with the new version
- Automatically commits and pushes changes to main branch
- Ensures `Cargo.toml` always matches the release version

### 4. **Publish Release Notes Job** (`publish-release`)
- Triggered on actual release events
- Creates a formatted release summary with download links
- Updates the release notes with available binaries and installation instructions

## How to Create a Release

### The Complete Process

1. **Update `Cargo.toml` with the new version** (IMPORTANT!)
   ```toml
   [package]
   version = "0.2.0"  # Update this to match your release tag
   ```

2. **Commit and push the version change**
   ```bash
   git add Cargo.toml
   git commit -m "chore: prepare v0.2.0 release"
   git push origin main
   ```

3. **Create a GitHub release with a matching tag**
   - Tag: `v0.2.0` (must match Cargo.toml `0.2.0`)
   - This triggers the entire workflow

4. **The workflow handles the rest automatically:**
   - ✅ Builds binaries for Windows, Linux x64, Linux ARM64
   - ✅ Uploads binaries to the release
   - ✅ Updates `Cargo.toml` back to main (extracts version from tag)
   - ✅ Updates install scripts (if tagged as `stable`)

### Method 1: GitHub Web Interface
1. Go to your GitHub repository
2. Click **Releases** → **Draft a new release**
3. Create a new tag (e.g., `v0.2.0`)
4. Add release title and description
5. Click **Publish release**
6. The workflow automatically starts!

### Method 2: Git Commands
```bash
# Tag the current commit
git tag v0.2.0
git push origin v0.2.0

# Or create an annotated tag with a message
git tag -a v0.2.0 -m "Release version 0.2.0"
git push origin v0.2.0
```

### Method 3: GitHub CLI
```bash
gh release create v0.2.0 --title "Klein v0.2.0" --notes "Release notes here"
```

## Update Stable Release

To update the `stable` release tag with new binaries:

1. **Using GitHub Web Interface:**
   - Delete the old `stable` release/tag
   - Create new release with tag name `stable`

2. **Or manually via Actions:**
   - Go to **Actions** → **Build and Release Binaries**
   - Click **Run workflow**
   - Enter `stable` as the version
   - Click **Run workflow**

The workflow will automatically update `install.ps1` and `install.sh` to point to the new stable release.

## Install Scripts Behavior

The updated install scripts now:

### Windows (`install.ps1`)
```powershell
# Downloads: klein-windows-x86_64.exe from stable release
irm https://raw.githubusercontent.com/Adarsh-codesOP/Klein/main/install.ps1 | iex
```

### Linux (`install.sh`)
```bash
# Detects OS and architecture
# Downloads appropriate binary (x86_64 or aarch64)
curl -sSL https://raw.githubusercontent.com/Adarsh-codesOP/Klein/main/install.sh | bash
```

Both scripts:
- Automatically fall back to building from source if binary download fails
- Add Klein to PATH automatically
- Support reconfiguration with `-Reconfigure` or `--reconfigure` flags

## Manual Build and Release

If you prefer not to use GitHub Actions, you can manually build and create releases:

```bash
# Build for Windows
cargo build --release --target x86_64-pc-windows-msvc

# Build for Linux x86_64
cargo build --release --target x86_64-unknown-linux-gnu

# Build for Linux ARM64 (requires cross)
cargo install cross
cross build --release --target aarch64-unknown-linux-gnu
```

Then manually upload the binaries to a GitHub release.

## Troubleshooting

### Workflow Failed to Trigger
- Ensure you're publishing the release from the repository's main branch
- Check that `.github/workflows/release.yml` file exists and is valid YAML

### Build Fails for a Specific Target
- Check the workflow logs in GitHub Actions
- Ensure all dependencies are available for the target platform
- Cross-platform builds may require additional setup

### Binaries Not Uploaded to Release
- Verify the `GITHUB_TOKEN` has sufficient permissions
- Check that the release was published (not saved as draft)

### Install Scripts Not Updated
- Ensure the release tag is exactly `stable`
- Check that the workflow job `update-install-scripts` ran successfully
- Verify GitHub Actions has write access to your repository

## Configuration

To customize the workflow, edit `.github/workflows/release.yml`:

### Add More Targets
Add new entries to the `matrix.include` section:
```yaml
- os: macos-latest
  target: aarch64-apple-darwin
  artifact_name: klein
  asset_name: klein-macos-aarch64
```

### Change Release Asset Names
Modify the `asset_name` field in the matrix to change how binaries are named.

### Skip Certain Steps
Remove job sections (e.g., `update-install-scripts`) if not needed.

## Status Badges

You can add a status badge to your README:

```markdown
[![Build Status](https://github.com/Adarsh-codesOP/Klein/actions/workflows/release.yml/badge.svg)](https://github.com/Adarsh-codesOP/Klein/actions/workflows/release.yml)
```

## Security Considerations

- The workflow uses `secrets.GITHUB_TOKEN` which is automatically provided by GitHub
- Token permissions are limited to the current repository
- Binary builds happen on GitHub-hosted runners (trusted environment)
- Consider using code signing for production releases

## Next Steps

1. Commit `.github/workflows/release.yml` to your repository
2. Update `install.ps1` and `install.sh` (already done)
3. Create a release tag on GitHub
4. Watch the workflow run in the Actions tab
5. Verify binaries are attached to the release
6. Test the install scripts point to the correct URLs

---

For more information about GitHub Actions, visit: https://docs.github.com/en/actions
