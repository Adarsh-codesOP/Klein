param (
    [switch]$Reconfigure,
    [switch]$Yes   # Non-interactive: accept all defaults (useful in CI)
)

$ErrorActionPreference = "Stop"

# ── Application identity ─────────────────────────────────────────────────────
# Single place to update if the project or binary is ever renamed.
# Can also be overridden via environment variables (e.g. in CI).
$AppName = if ($env:APP_NAME) { $env:APP_NAME } else { "Klein" }
$BinaryName = if ($env:BINARY_NAME) { $env:BINARY_NAME } else { "klein" }

# ─────────────────────────────────────────────────────────────────────────────
# Klein — cross-platform installer (PowerShell Core 7+)
#
# Supports: Windows, Linux, macOS
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

# ── Platform detection ───────────────────────────────────────────────────────
$OnWindows = $IsWindows -or ($PSVersionTable.PSEdition -eq "Desktop")
$OnMacOS = [bool]$IsMacOS
$OnLinux = [bool]$IsLinux

# ── Repository detection ─────────────────────────────────────────────────────
function Get-Repo {
    $scriptDir = $null

    if ($MyInvocation.MyCommand.Path) {
        $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    }

    if ($scriptDir) {
        $gitDir = Join-Path $scriptDir ".git"
        if (Test-Path $gitDir -ErrorAction SilentlyContinue) {
            try {
                $remoteUrl = git -C $scriptDir remote get-url origin 2>$null
                if ($remoteUrl -match 'github\.com[:/]([^/]+/[^/]+?)(\.git)?$') {
                    return $Matches[1]
                }
            }
            catch {}
        }
    }

    return "Adarsh-CodesOP/Klein"
}
$Repo = if ($env:REPO) { $env:REPO } else { Get-Repo }
$RepoOwner = $Repo.Split("/")[0]
$RepoName = $Repo.Split("/")[1]

# ── Paths (platform-aware) ───────────────────────────────────────────────────
if ($OnWindows) {
    $AppDir = "$env:LOCALAPPDATA\$AppName"
    $BinDir = "$AppDir\bin"   # cargo install --root uses <root>/bin/ — keep consistent
    $ConfigPath = "$AppDir\config.toml"
    $BinName = "$BinaryName.exe"
    $TmpRoot = $env:TEMP
    $HomeDir = $env:USERPROFILE
}
else {
    $HomeDir = $env:HOME
    $AppDir = "$HomeDir/.local/share/$BinaryName"
    $BinDir = "$HomeDir/.local/bin"
    $ConfigDir = if ($OnMacOS) { "$HomeDir/Library/Application Support/$BinaryName" } else { "$HomeDir/.config/$BinaryName" }
    $ConfigPath = "$ConfigDir/config.toml"
    $BinName = $BinaryName
    $TmpRoot = if ($env:TMPDIR) { $env:TMPDIR } else { "/tmp" }
}
$BinPath = Join-Path $BinDir $BinName

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
    }
    catch {
        Write-Host "Warning: could not fetch release info from GitHub." -ForegroundColor $Yellow
        return $null
    }
}

function Get-TargetTriple {
    # Determine Rust target triple for the current platform.
    # Asset naming: klein-<version>-<triple>.<zip|tar.gz>
    if ($OnWindows) {
        $arch = $env:PROCESSOR_ARCHITECTURE
        switch ($arch) {
            "AMD64" { return "x86_64-pc-windows-msvc" }
            "ARM64" { return "aarch64-pc-windows-msvc" }
            default { return "x86_64-pc-windows-msvc" }
        }
    }
    elseif ($OnMacOS) {
        $arch = (uname -m).Trim()
        switch ($arch) {
            "arm64" { return "aarch64-apple-darwin" }
            "x86_64" { return "x86_64-apple-darwin" }
            default { return "x86_64-apple-darwin" }
        }
    }
    else {
        # Linux
        $arch = (uname -m).Trim()
        switch ($arch) {
            "aarch64" { return "aarch64-unknown-linux-gnu" }
            "arm64" { return "aarch64-unknown-linux-gnu" }
            "x86_64" { return "x86_64-unknown-linux-gnu" }
            default { return "x86_64-unknown-linux-gnu" }
        }
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
    }
    catch {
        return $false
    }
}

function Install-ViaGitHubRelease {
    $version = Get-LatestVersion
    if (-not $version) {
        Write-Host "No GitHub release found, skipping." -ForegroundColor $Yellow
        return $false
    }

    $triple = Get-TargetTriple
    $tmpDir = Join-Path $TmpRoot "klein-install-$([System.Guid]::NewGuid().ToString('N').Substring(0,8))"
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

    try {
        if ($OnWindows) {
            $archive = "klein-$version-$triple.zip"
            $url = "https://github.com/$Repo/releases/download/$version/$archive"
            Write-Host "Downloading $archive ($version)…" -ForegroundColor $Yellow
            Write-Host "URL: $url" -ForegroundColor $DarkGray
            Invoke-WebRequest -Uri $url -OutFile "$tmpDir/$archive" -ErrorAction Stop
            Expand-Archive -Path "$tmpDir/$archive" -DestinationPath $tmpDir -Force
            $extracted = Get-ChildItem -Path $tmpDir -Filter "klein.exe" -Recurse | Select-Object -First 1
            if ($extracted) {
                New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
                Copy-Item -Path $extracted.FullName -Destination $BinPath -Force
                Write-Host "✔ Installed $version to $BinPath" -ForegroundColor $Green
                return $true
            }
            # Legacy fallback
            $legacyUrl = "https://github.com/$Repo/releases/download/$version/klein-windows-x86_64.exe"
            Write-Host "Archive binary not found, trying legacy .exe…" -ForegroundColor $Yellow
            Invoke-WebRequest -Uri $legacyUrl -OutFile $BinPath -ErrorAction Stop
            Write-Host "✔ Installed $version (legacy) to $BinPath" -ForegroundColor $Green
            return $true
        }
        else {
            # Linux / macOS — download .tar.gz
            $archive = "klein-$version-$triple.tar.gz"
            $url = "https://github.com/$Repo/releases/download/$version/$archive"
            Write-Host "Downloading $archive ($version)…" -ForegroundColor $Yellow
            Write-Host "URL: $url" -ForegroundColor $DarkGray
            Invoke-WebRequest -Uri $url -OutFile "$tmpDir/$archive" -ErrorAction Stop
            & tar -xzf "$tmpDir/$archive" -C $tmpDir
            $extracted = Get-ChildItem -Path $tmpDir -Filter "klein" | Where-Object { -not $_.PSIsContainer } | Select-Object -First 1
            if ($extracted) {
                New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
                Copy-Item -Path $extracted.FullName -Destination $BinPath -Force
                & chmod +x $BinPath
                Write-Host "✔ Installed $version to $BinPath" -ForegroundColor $Green
                return $true
            }
            # Legacy fallback (plain binary)
            $legacyArch = if ($triple -match "aarch64") { "aarch64" } else { "x86_64" }
            $legacyName = if ($OnMacOS) { "klein-macos-$legacyArch" } else { "klein-linux-$legacyArch" }
            $legacyUrl = "https://github.com/$Repo/releases/download/$version/$legacyName"
            Write-Host "Archive binary not found, trying legacy binary…" -ForegroundColor $Yellow
            Invoke-WebRequest -Uri $legacyUrl -OutFile $BinPath -ErrorAction Stop
            & chmod +x $BinPath
            Write-Host "✔ Installed $version (legacy) to $BinPath" -ForegroundColor $Green
            return $true
        }
    }
    catch {
        Write-Host "Download failed: $_" -ForegroundColor $Yellow
        return $false
    }
    finally {
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
        cargo install --git "https://github.com/$Repo" --root $AppDir

        # Cargo installs binaries into $AppDir/bin.
        # Only copy when source != destination (on Windows $AppDir\bin IS $BinDir,
        # so src and dst are identical — copying to self throws and is unnecessary).
        $built = Join-Path (Join-Path $AppDir "bin") $BinName
        if ((Test-Path $built) -and ($built -ne $BinPath)) {
            New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
            Copy-Item $built $BinPath -Force
        }

        return $true
    }
    catch {
        try {
            cargo install --path . --root $AppDir

            $built = Join-Path (Join-Path $AppDir "bin") $BinName
            if ((Test-Path $built) -and ($built -ne $BinPath)) {
                New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
                Copy-Item $built $BinPath -Force
            }

            return $true
        }
        catch {
            return $false
        }
    }
}
# ── Configuration ─────────────────────────────────────────────────────────────

function Invoke-Configuration {
    if ($OnWindows) {
        New-Item -ItemType Directory -Path $AppDir -Force | Out-Null
    }
    else {
        New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
    }
    Write-Host "`n╭────────────┤ Configuration ├────────────╮" -ForegroundColor $Cyan

    $availableLsps = @("Rust", "Python", "JavaScript", "TypeScript", "C", "Go", "Java", "HTML", "CSS", "JSON", "YAML", "Markdown", "TOML")
    $enabledLsps = @()

    if ($Yes) {
        $workspace = $HomeDir
        $shell = "auto"
        $enabledLsps = @("Rust") # Default enabled in non-interactive
    }
    else {
        if ($OnWindows) {
            $gitBashExists = (Test-Path "C:\Program Files\Git\bin\bash.exe") -or
            (Test-Path "$env:LOCALAPPDATA\Programs\Git\bin\bash.exe")
            if (-not $gitBashExists) {
                Write-Host "Git Bash not found — install from https://gitforwindows.org/ for best experience." -ForegroundColor $Yellow
            }
            $shell = if ($gitBashExists) { "bash" } else { "auto" }
        }
        else {
            $shell = "auto"
        }

        $workspace = Read-Host "Default workspace path (blank = $HomeDir)"
        if ([string]::IsNullOrWhiteSpace($workspace)) { $workspace = $HomeDir }

        if (-not (Test-Path $workspace)) {
            $create = Read-Host "Path '$workspace' does not exist. Create it? (y/N)"
            if ($create -eq 'y') { New-Item -ItemType Directory -Path $workspace | Out-Null }
        }

        Write-Host "`nSelect Language Servers (LSPs) to enable (Add-ons):" -ForegroundColor $Cyan
        for ($i = 0; $i -lt $availableLsps.Count; $i++) {
            Write-Host "  [$($i + 1)] $($availableLsps[$i])" -ForegroundColor $White
        }
        $selection = Read-Host "Enter numbers separated by space (e.g. 1 2 5) [Default: 1]"
        if ([string]::IsNullOrWhiteSpace($selection)) {
            $enabledLsps = @("Rust")
        }
        else {
            $idx = 0
            foreach ($num in $selection.Split(" ", [System.StringSplitOptions]::RemoveEmptyEntries)) {
                if ([int]::TryParse($num, [ref]$idx) -and $idx -ge 1 -and $idx -le $availableLsps.Count) {
                    $enabledLsps += $availableLsps[$idx - 1]
                }
            }
        }
    }

    $lspJson = "[$(($enabledLsps | ForEach-Object { '"{0}"' -f $_.ToLower() }) -join ', ')]"

    @"
# Klein TIDE Configuration
default_workspace = "$workspace"
shell = "$shell"
enabled_lsps = $lspJson
"@ | Out-File -FilePath $ConfigPath -Encoding utf8
    Write-Host "Configuration written to $ConfigPath" -ForegroundColor $Green
}

# ── PATH setup ────────────────────────────────────────────────────────────────

function Add-ToPath {
    if ($OnWindows) {
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($userPath -notmatch [regex]::Escape($BinDir)) {
            [Environment]::SetEnvironmentVariable("Path", "$userPath;$BinDir", "User")
            Write-Host "Added $BinDir to User PATH." -ForegroundColor $Green
            Write-Host "Restart your terminal to use 'klein' globally." -ForegroundColor $Yellow
        }
    }
    else {
        # Determine shell RC file from $SHELL env var (may be empty in containers)
        $shellEnv = $env:SHELL
        $shell = if ($shellEnv) { Split-Path -Leaf $shellEnv } else { "" }
        $rcFile = switch ($shell) {
            "zsh" { "$HomeDir/.zshrc" }
            "bash" { "$HomeDir/.bashrc" }
            "fish" { "$HomeDir/.config/fish/config.fish" }
            default {
                # Prefer .bashrc if bash is present, else .profile
                if (Get-Command bash -ErrorAction SilentlyContinue) { "$HomeDir/.bashrc" }
                else { "$HomeDir/.profile" }
            }
        }
        $exportLine = "export PATH=""$BinDir"":$env:PATH"
        $marker = "# klein PATH"
        # Create RC file if it doesn't exist (containers/CI may have empty $HOME)
        if (-not (Test-Path $rcFile)) {
            New-Item -Path $rcFile -ItemType File -Force | Out-Null
        }
        $alreadySet = Select-String -Path $rcFile -Pattern ([regex]::Escape($BinDir)) -Quiet -ErrorAction SilentlyContinue
        if (-not $alreadySet) {
            Add-Content -Path $rcFile -Value "`n$marker`n$exportLine"
            Write-Host "Added $BinDir to PATH in $rcFile" -ForegroundColor $Green
            Write-Host "Restart your terminal or run: export PATH=""$BinDir"":$env:PATH" -ForegroundColor $Yellow
        }
    }
}

# ── Main ──────────────────────────────────────────────────────────────────────

Write-Host "Starting installation (repo: $Repo)…" -ForegroundColor $Yellow
New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
if ($OnWindows) { New-Item -ItemType Directory -Path $AppDir -Force | Out-Null }

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
