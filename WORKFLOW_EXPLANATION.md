# Klein Workflow Explanation & Publishing Guide

## 📋 Project Overview

**Klein** is a Terminal IDE (TIDE) - a professional terminal-based text editor with IDE-like features, written in Rust.

### Current Status
- **Current Version:** 0.2.3 (from Cargo.toml)
- **Repository:** https://github.com/Adarsh-codesOP/Klein
- **Platforms Supported:** Windows x86_64, Linux x86_64, Linux ARM64

---

## 🔄 Complete Workflow

### Step 1: Development Workflow

```bash
# Clone the repository (if not already done)
git clone https://github.com/Adarsh-codesOP/Klein.git
cd Klein

# Make your code changes
# ... edit files ...

# Test locally
cargo run

# Build for testing
cargo build --release
```

### Step 2: Update Version (IMPORTANT!)

Before releasing, update the version in `Cargo.toml`:

```toml
[package]
name = "klein"
version = "0.2.4"  # Increment this!
edition = "2021"
```

### Step 3: Commit Changes

```bash
git add .
git commit -m "feat: your feature description"
git push origin main
```

### Step 4: Create Release on GitHub

**Method A: GitHub Web Interface**
1. Go to https://github.com/Adarsh-codesOP/Klein/releases
2. Click "Draft a new release"
3. Enter tag version: `v0.2.4` (must match Cargo.toml version with 'v' prefix)
4. Enter title: "Klein v0.2.4"
5. Add release notes
6. Click "Publish release"

**Method B: Git CLI**
```bash
git tag v0.2.4
git push origin v0.2.4
```

**Method C: GitHub CLI**
```bash
gh release create v0.2.4 --title "Klein v0.2.4" --notes "Your release notes"
```

---

## ⚙️ What Happens Automatically

After you publish a release, the GitHub Actions workflow (`.github/workflows/release.yml`) automatically:

1. **Builds Binaries** for all 3 platforms:
   - `klein-windows-x86_64.exe` (Windows)
   - `klein-linux-x86_64` (Linux x86_64)
   - `klein-linux-aarch64` (Linux ARM64)

2. **Uploads** binaries to the release

3. **Updates Cargo.toml** on main branch (extracts version from tag)

4. **Updates Install Scripts** (only if tagged as `stable`)

---

## 🏷️ Release Types

### Regular Release (e.g., v0.2.4)
```bash
# Create tag
git tag v0.2.4
git push origin v0.2.4
```
- Builds binaries
- Updates Cargo.toml version
- Does NOT update install scripts

### Stable Release
```bash
# Tag as 'stable'
git tag stable
git push origin stable
```
- Builds binaries
- Updates install.ps1 and install.sh to point to this release
- Users will get this version when running install scripts

---

## 🔧 Manual Actions (If Errors Occur)

### Fix 1: Version Not Updated in Cargo.toml
The workflow should auto-update, but if it fails:
```bash
# Manually update
sed -i 's/version = "0.2.3"/version = "0.2.4"/' Cargo.toml
git add Cargo.toml
git commit -m "chore: bump version to v0.2.4"
git push
```

### Fix 2: Install Scripts Not Pointing to Latest Release
Run the workflow manually:
1. Go to GitHub Actions → "Build and Release Binaries"
2. Click "Run workflow"
3. Enter "stable" as version
4. Click "Run workflow"

### Fix 3: Workflow Failed
1. Go to Actions tab → Click failed workflow
2. Click on failed job to see error logs
3. Common fixes:
   - Syntax errors → Fix code and push again
   - Dependency issues → Check Cargo.toml dependencies
   - Token issues → Ensure GITHUB_TOKEN has correct permissions

---

## ✅ Pre-Release Checklist

Before publishing:
- [ ] All tests pass locally (`cargo test`)
- [ ] Version updated in Cargo.toml
- [ ] Changes committed and pushed to main
- [ ] Release notes prepared

After publishing:
- [ ] Wait 5-15 minutes for builds
- [ ] Check Actions tab for build status
- [ ] Verify binaries are attached to release
- [ ] Download and test at least one binary
- [ ] Mark as "stable" if it's the recommended version

---

## 📦 Distribution

### Users Can Install Using:

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/Adarsh-CodesOP/Klein/main/install.ps1 | iex
```

**Linux (Bash):**
```bash
curl -sSL https://raw.githubusercontent.com/Adarsh-CodesOP/Klein/main/install.sh | bash
```

### Or Download Directly:
- Windows: https://github.com/Adarsh-codesOP/Klein/releases/download/v0.2.4/klein-windows-x86_64.exe
- Linux: https://github.com/Adarsh-codesOP/Klein/releases/download/v0.2.4/klein-linux-x86_64

