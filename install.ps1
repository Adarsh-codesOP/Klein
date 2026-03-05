param (
    [switch]$Reconfigure
)

$ErrorActionPreference = "Stop"
$AppDir = "$env:LOCALAPPDATA\Klein"
$ConfigPath = "$AppDir\config.toml"

$AsciiArt = @"
  _  __ _      _         _____  ____  ______ 
 | |/ /| |    (_)       |_   _||  _ \|  ____|
 | ' / | |     _  _ __    | |  | |  \| |__   
 |  <  | |    | || '_ \   | |  | |  |  ___|  
 | . \ | |____| || | | | _| |_ | |_/ / |____ 
 |_|\_\|______|_||_| |_||_____||____/|______|
                                             
"@

Write-Host "`n"
Write-Host $AsciiArt -ForegroundColor Cyan
Write-Host " ==========================================" -ForegroundColor DarkGray
Write-Host "     The aesthetic terminal IDE in Rust    " -ForegroundColor White
Write-Host " ==========================================" -ForegroundColor DarkGray
Write-Host "`n"

# 1. Ensure Directory Exists
if (-not (Test-Path -Path $AppDir)) {
    New-Item -ItemType Directory -Path $AppDir | Out-Null
    Write-Host "Created application directory at $AppDir"
}

# 2. Configuration Step
function Prompt-Configuration {
    Write-Host "`n--- Configuration ---" -ForegroundColor Yellow
    
    # Check for Git Bash
    $gitBashExists = Test-Path "C:\Program Files\Git\bin\bash.exe"
    $gitBashLocalExists = Test-Path "$env:LOCALAPPDATA\Programs\Git\bin\bash.exe"
    
    if (-not $gitBashExists -and -not $gitBashLocalExists) {
        Write-Host "WARNING: Git Bash was not found in standard locations." -ForegroundColor Red
        Write-Host "We highly recommend installing Git Bash for the best terminal experience in Klein." -ForegroundColor Yellow
        Write-Host "You can download it from: https://gitforwindows.org/" -ForegroundColor Cyan
        $installGit = Read-Host "Would you like to install Git Bash later? (y/n)"
        if ($installGit -eq 'n') {
            Write-Host "You can continue, but terminal features might be limited to PowerShell or CMD."
        }
    }

    $workspace = Read-Host "Enter your default workspace/projects path (Leave blank for $env:USERPROFILE)"
    if ([string]::IsNullOrWhiteSpace($workspace)) {
        $workspace = $env:USERPROFILE
    }

    # Verify workspace exists or create it
    if (-not (Test-Path -Path $workspace)) {
        $createWs = Read-Host "Path '$workspace' does not exist. Create it? (y/N)"
        if ($createWs -eq 'y') {
            New-Item -ItemType Directory -Path $workspace | Out-Null
        }
        else {
            Write-Host "Warning: Workspace path may be invalid." -ForegroundColor Yellow
        }
    }

    if ($gitBashExists -or $gitBashLocalExists) {
        $shell = "bash"
    }
    else {
        $shell = "auto"
    }

    $configContent = @"
# Klein IDE Configuration
default_workspace = `"$workspace`"
shell = `"$shell`"
"@

    $configContent | Out-File -FilePath $ConfigPath -Encoding utf8
    Write-Host "Configuration saved to $ConfigPath" -ForegroundColor Green
}

if ($Reconfigure) {
    Prompt-Configuration
    Write-Host "`nReconfiguration complete!" -ForegroundColor Green
    exit
}

# 3. Installation Step
Write-Host "`n--- Installation ---" -ForegroundColor Yellow

$exePath = "$AppDir\klein.exe"
try {
    Write-Host "Downloading pre-compiled binary from GitHub Releases..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri "https://github.com/Adarsh-codesOP/Klein/releases/download/stable/klein.exe" -OutFile "$exePath"
    Write-Host "Successfully downloaded to $exePath" -ForegroundColor Green
    
    # Add to User PATH if not present
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notmatch [regex]::Escape($AppDir)) {
        $newPath = $userPath + ";" + $AppDir
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Host "Added $AppDir to your User PATH environment variable." -ForegroundColor Green
        Write-Host "You may need to restart your terminal to use the 'klein' command globally." -ForegroundColor Yellow
    }
}
catch {
    Write-Host "Failed to download the executable. Please install Rust and run 'cargo install --path .' from the source code." -ForegroundColor Red
}

Prompt-Configuration

Write-Host "`nInstallation & Configuration Complete!" -ForegroundColor Green
Write-Host "You can run this script later with '-Reconfigure' to update your settings." -ForegroundColor Cyan
