# Quick Start: How to Release Klein

## Fastest Way: Using GitHub Web Interface

1. **Update version in Cargo.toml** (IMPORTANT!)
   ```toml
   # Cargo.toml
   [package]
   name = "klein"
   version = "0.2.0"  # Update this to match your release
   ```
   
   ```bash
   git add Cargo.toml
   git commit -m "chore: prepare v0.2.0 release"
   git push origin main
   ```

2. **Go to GitHub → Releases → Draft a new release**

3. **Fill in the details:**
   - **Tag version:** Must match Cargo.toml version (e.g., `v0.2.0` if Cargo.toml is `version = "0.2.0"`)
   - **Release title:** e.g., "Klein v0.2.0 - Major Update"
   - **Description:** Write release notes (what changed, new features, etc.)
   - **Check "Set as latest release"** (unless it's a pre-release)

4. **Click "Publish release"**

5. **Wait 5-15 minutes** while:
   - 🔨 Binaries build for all 3 platforms
   - 📦 Binaries upload to the release
   - 📝 `Cargo.toml` auto-updates on main branch
   - 🔗 Install scripts auto-update (if marked stable)

6. **Verify everything is ready:**
   - Go back to the release page
   - Scroll down to see attached binaries (`.exe`, Linux builds)
   - Check that a commit was pushed updating `Cargo.toml` (unless tag == `stable`)
   - They should appear automatically once builds complete

## Updating the "Stable" Release

The `stable` tag always points to the recommended version for users installing with the default scripts.

### To make the latest release "stable":

**Option A: GitHub Web Interface**
1. Go to your new release
2. Click the ⭐ icon next to the release (or use "Set as latest release")
3. The workflow automatically updates `install.ps1` and `install.sh`

**Option B: Using GitHub Actions (Manual)**
1. Go to **Actions** tab
2. Click **"Build and Release Binaries"**
3. Click **"Run workflow"** button
4. Enter `stable` as the version
5. Click **"Run workflow"**

## View Build Progress

1. **Go to Actions tab** on GitHub
2. **Click the latest workflow run**
3. **Click "build"** to see build details
4. Monitor the job output

## Release Checklist

- [ ] All tests pass locally
- [ ] **Version number updated in `Cargo.toml`** ⬅️ DO THIS FIRST!
- [ ] `CHANGELOG.md` or release notes updated
- [ ] Commit and push all changes to `main`
- [ ] Create GitHub release with matching tag (e.g., tag `v0.2.0` if Cargo.toml is `0.2.0`)
- [ ] Workflow automatically:
  - ✅ Builds binaries for all platforms
  - ✅ Uploads to the release
  - ✅ Updates install scripts (if marked stable)
- [ ] Wait for binaries to build (5-15 min)
- [ ] Download and test one binary locally
- [ ] Mark as stable if it's the recommended version
- [ ] Announce release on social media/Discord/etc.

## Common Release Procedures

### Release a New Major Version
```bash
# 1. Update version in Cargo.toml
# [package]
# version = "1.0.0"

# 2. Test locally
cargo build --release
cargo test

# 3. Commit and push
git add Cargo.toml
git commit -m "chore: prepare v1.0.0 release"
git push origin main

# 4. Create release on GitHub with tag v1.0.0
# The workflow automatically:
# ✅ Builds all binaries
# ✅ Updates Cargo.toml back on main (if not "stable" tag)
# ✅ Updates install scripts (if "stable" tag)
```

### Update Stable Binaries Only (No Version Change)
```bash
# Use GitHub Actions to republish stable without creating a new version tag
# Actions tab → Run workflow → enter "stable" as version
# This SKIPS the version update in Cargo.toml (since it's just the stable pointer)
```

### Emergency Hotfix
```bash
# 1. Update Cargo.toml to v0.2.1
# 2. Commit and push
git add Cargo.toml
git commit -m "chore: prepare v0.2.1 hotfix"
git push origin main

# 3. Create release with tag v0.2.1 on GitHub
git tag v0.2.1
git push origin v0.2.1

# Or create release directly on GitHub
```

## Troubleshooting

### "Release tag already exists"
You can delete and recreate or increment the version number (e.g., `v0.2.1` instead of `v0.2.0`)

### Binaries never appeared after 20 minutes
1. Check **Actions** tab for failed builds
2. Click the failed job for error logs
3. Common issues:
   - Invalid Rust code (syntax errors)
   - Missing dependencies
   - Insufficient permissions in GitHub Token

### Install scripts still point to old release
This is automatic only for the `stable` tag. To fix:
1. Make sure you tagged it as `stable` (not `v0.2.0`)
2. Or manually run the workflow with `stable` version

## Pre-Release Versions

For beta/alpha releases:

1. Update `Cargo.toml` to `version = "0.2.0-beta.1"`
2. Create tag: `v0.2.0-beta.1`
3. In GitHub Release form:
   - Check **"This is a pre-release"** ✓
   - Use the same version: `v0.2.0-beta.1`
4. Publish

Users won't auto-update to pre-releases.

## How Automatic Version Bumping Works

When you publish a release, the GitHub Actions workflow:

1. **Extracts version from tag**
   - Tag: `v0.2.0` → Version: `0.2.0`

2. **Updates `Cargo.toml`**
   ```toml
   [package]
   version = "0.2.0"  # Auto-updated!
   ```

3. **Commits back to main** with message:
   ```
   chore: bump version to v0.2.0
   ```

**Exception:** If the tag is `stable`, version bump is skipped (since `stable` is a pointer, not a versioned release).

This ensures `Cargo.toml` always matches your latest release tag!

## Files Generated by Workflow

After a successful release, these files are created:

| File | Platform |
|------|----------|
| `klein-windows-x86_64.exe` | Windows 64-bit |
| `klein-linux-x86_64` | Linux 64-bit |
| `klein-linux-aarch64` | Linux ARM64 |

Users can download from: `https://github.com/Adarsh-codesOP/Klein/releases/download/vX.Y.Z/filename`

The `stable` release always points to the recommended version.

---

**Need help?** Check [CI_CD_GUIDE.md](CI_CD_GUIDE.md) for detailed workflow documentation.
