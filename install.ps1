# minmax-code (Rust) — Install Script for Windows
# Usage: irm https://raw.githubusercontent.com/ezeoli88/minmax-code/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

$REPO = "ezeoli88/minmax-code"
$INSTALL_DIR = "$env:USERPROFILE\.minmax-code\bin"
$ARCHIVE = "minmax-code-windows-x64.zip"
$URL = "https://github.com/$REPO/releases/latest/download/$ARCHIVE"

Write-Host ""
Write-Host "  minmax-code Installer (Rust native binary)"
Write-Host "  ============================================"
Write-Host ""
Write-Host "  Downloading $ARCHIVE..."

# Create install directory
New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null

# Download
$TMPDIR = New-TemporaryFile | ForEach-Object { Remove-Item $_; New-Item -ItemType Directory -Path $_ }
$TMPFILE = Join-Path $TMPDIR $ARCHIVE
Invoke-WebRequest -Uri $URL -OutFile $TMPFILE

# Extract
Expand-Archive -Path $TMPFILE -DestinationPath $INSTALL_DIR -Force
Remove-Item -Recurse -Force $TMPDIR

Write-Host "  Installed to $INSTALL_DIR"

# Add to PATH
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*\.minmax-code\bin*") {
    [Environment]::SetEnvironmentVariable("Path", "$INSTALL_DIR;$userPath", "User")
    Write-Host "  Added to user PATH"
}

Write-Host ""
Write-Host "  Done! Open a new terminal and run 'minmax-code' to start."
Write-Host ""
Write-Host "  Advantages of the Rust binary:"
Write-Host "    - Single binary, no runtime dependencies"
Write-Host "    - ripgrep search engine built-in"
Write-Host "    - ~5-10MB binary size"
Write-Host "    - Faster startup and lower memory usage"
Write-Host ""
