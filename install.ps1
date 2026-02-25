#!/usr/bin/env pwsh
# minmax-code — Install Script (Windows)
# Usage: irm https://raw.githubusercontent.com/ezequiel/minmax-code/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "ezeoli88/minmax-code"
$InstallDir = "$env:USERPROFILE\.minmax-code\bin"

Write-Host ""
Write-Host "  minmax-code Installer" -ForegroundColor Cyan
Write-Host "  =====================" -ForegroundColor DarkGray
Write-Host ""

# Detect architecture
$Arch = "x64"
if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") {
    Write-Host "  ARM64 detected, but only x64 binaries are available." -ForegroundColor Yellow
    Write-Host "  Attempting x64 (emulation)..." -ForegroundColor Yellow
}

$Archive = "minmax-code-windows-${Arch}.zip"
$Url = "https://github.com/${Repo}/releases/latest/download/${Archive}"

Write-Host "  Platform: windows-${Arch}" -ForegroundColor White
Write-Host "  Downloading ${Archive}..." -ForegroundColor Yellow

# Download
$TempFile = Join-Path $env:TEMP $Archive
Invoke-WebRequest -Uri $Url -OutFile $TempFile -UseBasicParsing

# Extract
if (!(Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}
Expand-Archive -Path $TempFile -DestinationPath $InstallDir -Force
Remove-Item $TempFile -Force

Write-Host "  Installed to ${InstallDir}" -ForegroundColor Green

# Add to PATH
$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*\.minmax-code\bin*") {
    [Environment]::SetEnvironmentVariable("PATH", "${InstallDir};${UserPath}", "User")
    $env:PATH = "${InstallDir};${env:PATH}"
    Write-Host "  Added to user PATH" -ForegroundColor Green
}

Write-Host ""
Write-Host "  Done! Run 'minmax-code' to start." -ForegroundColor Green
Write-Host "  (You may need to restart your terminal for PATH changes to take effect)" -ForegroundColor DarkGray
Write-Host ""
