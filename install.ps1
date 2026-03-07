param (
    [switch]$Reconfigure,
    [switch]$Yes   # Non-interactive: accept all defaults (useful in CI)
)

$ErrorActionPreference = "Stop"

# ─────────────────────────────────────────────────────────────────────────────
# Klein — Windows installer (PowerShell)
#
# Usage:
#   irm https://raw.githubusercontent.com/<owner>/Klein/main/install.ps1 | iex
#   .\install.ps1              # interactive
#   .\install.ps1 -Yes         # non-interactive (CI-friendly)
#   .\install.ps1 -Reconfigure # re-run configuration only
#
# Environment variable override:
#   $env:REPO = "owner/repo"   # defaults to upstream or auto-detected from git remote
# ─────────────────────────────────────────────────────────────────────────────

# ── Repository detection ─────────────────────────────────────────────────────
function Get-Repo {
    $scriptDir = Split-Path -Parent $MyInvocation.ScriptName
    if ($scriptDir -and (Test-Path "$scriptDir\.git" -ErrorAction SilentlyContinue)) {
        try {
            $remoteUrl = git -C $scriptDir remote get-url origin 2>$null
            if ($remoteUrl -match 'github\.com[:/]([^/]+/[^/]+?)(\.git)?$') {
                return $Matches[1]
            }
        } catch {}
    }
    return "Adarsh-codesOP/Klein"
}
$Repo = if ($env:REPO) { $env:REPO } else { Get-Repo }
$RepoOwner = $Repo.Split("/")[0]
$RepoName  = $Repo.Split("/")[1]

# ── Paths ────────────────────────────────────────────────────────────────────
$AppDir     = "$env:LOCALAPPDATA\Klein"
$BinDir     = $AppDir
$ConfigPath = "$AppDir\config.toml"
$BinName    = "klein.exe"
$BinPath    = "$BinDir\$BinName"

# ── Colours ──────────────────────────────────────────────────────────────────
$Cyan = "Cyan"; $White = "White"; $Green = "Green"
$Yellow = "Yellow"; $DarkGray = "DarkGray"; $Red = "Red"

Write-Host "" -ForegroundColor $Cyan
Write-Host "oooo   oooo ooooo       ooooooooooo ooooo oooo   oooo " -ForegroundColor $Cyan
Write-Host " 888  o88    888         888    88   888   8888o  88  " -ForegroundColor $Cyan
Write-Host " 888888      888         888ooo8     888   88 888o88  " -ForegroundColor $Cyan
Write-Host " 888  88o    888      o  888    oo   888   88   8888  " -ForegroundColor $Cyan
Write-Host "o888o o888o o888ooooo88 o888ooo8888 o888o o88o    88  " -ForegroundColor $Cyan
Write-Host "                                                      " -ForegroundColor $Cyan
Write-Host "A professional terminal text editor with IDE-like features.`n" -ForegroundColor $White

# ── Helpers ───────────────────────────────────────────────────────────────────

function Get-LatestVersion {
    $apiUrl = "https://api.github.com/repos/$Repo/releases/latest"
    try {
        $resp = Invoke-RestMethod -Uri $apiUrl -ErrorAction Stop
        return $resp.tag_name
    } catch {
        Write-Host "Warning: could not fetch release info from GitHub." -ForegroundColor $Yellow
        return $null
    }
}

function Get-TargetTriple {
    # Determine Rust target triple for the current Windows architecture.
    # Asset naming: klein-<version>-<triple>.zip
    $arch = $env:PROCESSOR_ARCHITECTURE
    switch ($arch) {
        "AMD64"  { return "x86_64-pc-windows-msvc" }
        "ARM64"  { return "aarch64-pc-windows-msvc" }
        default  { return "x86_64-pc-windows-msvc" }  # safe default
    }
}

# ── Installation methods ──────────────────────────────────────────────────────

function Install-ViaMise {
    if (-not (Get-Command mise -ErrorAction SilentlyContinue)) { return $false }
    Write-Host "Trying mise (github backend)…" -ForegroundColor $Yellow
    try {
        mise use -g "github:$Repo"
        Write-Host "✔ Installed via mise!" -ForegroundColor $Green
        return $true
    } catch {
        return $false
    }
}

function Install-ViaGitHubRelease {
    $version = Get-LatestVersion
    if (-not $version) {
        Write-Host "No GitHub release found, skipping." -ForegroundColor $Yellow
        return $false
    }

    $triple  = Get-TargetTriple
    $archive = "klein-$version-$triple.zip"
    $url     = "https://github.com/$Repo/releases/download/$version/$archive"

    Write-Host "Downloading $archive ($version)…" -ForegroundColor $Yellow
    Write-Host "URL: $url" -ForegroundColor $DarkGray

    $tmpDir = Join-Path $env:TEMP "klein-install-$([System.Guid]::NewGuid().ToString('N').Substring(0,8))"
    New-Item -ItemType Directory -Path $tmpDir | Out-Null

    try {
        Invoke-WebRequest -Uri $url -OutFile "$tmpDir\$archive" -ErrorAction Stop

        Expand-Archive -Path "$tmpDir\$archive" -DestinationPath $tmpDir -Force

        $extracted = Get-ChildItem -Path $tmpDir -Filter "klein.exe" -Recurse | Select-Object -First 1
        if ($extracted) {
            New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
            Copy-Item -Path $extracted.FullName -Destination $BinPath -Force
            Write-Host "✔ Installed $version to $BinPath" -ForegroundColor $Green
            return $true
        }

        # Fallback: legacy plain-exe naming (klein-windows-x86_64.exe)
        $legacyUrl = "https://github.com/$Repo/releases/download/$version/klein-windows-x86_64.exe"
        Write-Host "Archive binary not found, trying legacy .exe…" -ForegroundColor $Yellow
        Invoke-WebRequest -Uri $legacyUrl -OutFile $BinPath -ErrorAction Stop
        Write-Host "✔ Installed $version (legacy) to $BinPath" -ForegroundColor $Green
        return $true

    } catch {
        Write-Host "Download failed: $_" -ForegroundColor $Yellow
        return $false
    } finally {
        Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Install-FromSource {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "Rust is not installed. Install from https://rustup.rs/" -ForegroundColor $Red
        return $false
    }
    Write-Host "Building from source (this may take a few minutes)…" -ForegroundColor $Yellow
    try {
        cargo install klein
        return $true
    } catch {
        try {
            cargo install --path .
            return $true
        } catch {
            return $false
        }
    }
}

# ── Configuration ─────────────────────────────────────────────────────────────

function Invoke-Configuration {
    New-Item -ItemType Directory -Path $AppDir -Force | Out-Null
    Write-Host "`n╭────────────┤ Configuration ├────────────╮" -ForegroundColor $Cyan

    if ($Yes) {
        $workspace = $env:USERPROFILE
        $shell = "auto"
    } else {
        $gitBashExists = (Test-Path "C:\Program Files\Git\bin\bash.exe") -or
                         (Test-Path "$env:LOCALAPPDATA\Programs\Git\bin\bash.exe")
        if (-not $gitBashExists) {
            Write-Host "Git Bash not found — install from https://gitforwindows.org/ for best experience." -ForegroundColor $Yellow
        }

        $workspace = Read-Host "Default workspace path (blank = $env:USERPROFILE)"
        if ([string]::IsNullOrWhiteSpace($workspace)) { $workspace = $env:USERPROFILE }

        if (-not (Test-Path $workspace)) {
            $create = Read-Host "Path '$workspace' does not exist. Create it? (y/N)"
            if ($create -eq 'y') { New-Item -ItemType Directory -Path $workspace | Out-Null }
        }
        $shell = if ($gitBashExists) { "bash" } else { "auto" }
    }

    @"
# Klein TIDE Configuration
default_workspace = "$workspace"
shell = "$shell"
"@ | Out-File -FilePath $ConfigPath -Encoding utf8
    Write-Host "Configuration saved to $ConfigPath" -ForegroundColor $Green
}

# ── PATH setup ────────────────────────────────────────────────────────────────

function Add-ToPath {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notmatch [regex]::Escape($BinDir)) {
        [Environment]::SetEnvironmentVariable("Path", "$userPath;$BinDir", "User")
        Write-Host "Added $BinDir to User PATH." -ForegroundColor $Green
        Write-Host "Restart your terminal to use 'klein' globally." -ForegroundColor $Yellow
    }
}

# ── Main ──────────────────────────────────────────────────────────────────────

Write-Host "Starting installation (repo: $Repo)…" -ForegroundColor $Yellow
New-Item -ItemType Directory -Path $AppDir  -Force | Out-Null
New-Item -ItemType Directory -Path $BinDir  -Force | Out-Null

if ($Reconfigure) {
    Invoke-Configuration
    Write-Host "`n✔ Reconfiguration complete!" -ForegroundColor $Green
    exit 0
}

Write-Host "`n╭────────────┤ Installation ├────────────╮" -ForegroundColor $Cyan

$installed = $false

# 1. mise github backend
if (-not $installed) { $installed = Install-ViaMise }
# 2. GitHub Release archive download
if (-not $installed) { $installed = Install-ViaGitHubRelease }
# 3. Source build fallback
if (-not $installed) { $installed = Install-FromSource }

if (-not $installed) {
    Write-Host "All installation methods failed." -ForegroundColor $Red
    exit 1
}

Add-ToPath
Invoke-Configuration

Write-Host "`n✔ Installation & Configuration Complete!" -ForegroundColor $Green
Write-Host "  Run 'klein' to start the editor." -ForegroundColor $Cyan
Write-Host "  Use '-Reconfigure' to update your settings." -ForegroundColor $Cyan
