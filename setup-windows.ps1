# CEF Environment Setup Script for Windows
#
# This script downloads and sets up CEF binaries for Windows development

param(
    [string]$InstallPath = "C:\cef",
    [string]$Version = "145.0.27"
)

$ErrorActionPreference = "Stop"

Write-Host "=== CEF Environment Setup for Windows ===" -ForegroundColor Cyan
Write-Host "CEF Version: $Version"
Write-Host "Install Directory: $InstallPath"
Write-Host ""

# Create installation directory
New-Item -ItemType Directory -Force -Path $InstallPath | Out-Null

# Determine download URL
$CefUrl = "https://cef-builds.spotifycdn.com/cef_binary_${Version}%2Bg1a2c619%2Bchromium-145.0.6955.0_windows64.tar.bz2"

Write-Host "Downloading CEF from:" -ForegroundColor Yellow
Write-Host $CefUrl

# Download CEF
$TempFile = "$env:TEMP\cef.tar.bz2"
try {
    Invoke-WebRequest -Uri $CefUrl -OutFile $TempFile -UseBasicParsing
    Write-Host "Download completed" -ForegroundColor Green
} catch {
    Write-Host "Download failed. Please download manually from:" -ForegroundColor Red
    Write-Host "https://cef-builds.spotifycdn.com/index.html#windows64" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "1. Download CEF $Version (Standard Distribution)"
    Write-Host "2. Extract the 'Release' folder contents to: $InstallPath"
    exit 1
}

# Extract (requires 7-Zip or built-in tar on Windows 10+)
Write-Host "Extracting CEF..." -ForegroundColor Yellow

$TempDir = "$env:TEMP\cef_extract"
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

# Try using tar (Windows 10+)
try {
    tar -xjf $TempFile -C $TempDir
    Write-Host "Extraction completed" -ForegroundColor Green
} catch {
    Write-Host "Extraction failed. Please install 7-Zip or extract manually." -ForegroundColor Red
    exit 1
}

# Find extracted folder and copy Release contents
$ExtractedFolder = Get-ChildItem -Path $TempDir -Directory | Select-Object -First 1
$ReleasePath = Join-Path $ExtractedFolder.FullName "Release"

if (Test-Path $ReleasePath) {
    Write-Host "Copying Release files to $InstallPath..." -ForegroundColor Yellow
    Copy-Item -Path "$ReleasePath\*" -Destination $InstallPath -Recurse -Force
    Write-Host "Files copied successfully" -ForegroundColor Green
} else {
    Write-Host "Release folder not found in extracted archive" -ForegroundColor Red
    exit 1
}

# Cleanup
Remove-Item -Path $TempFile -Force -ErrorAction SilentlyContinue
Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "=== Environment Variables ===" -ForegroundColor Cyan
Write-Host "Add the following to your PowerShell profile or set manually:"
Write-Host ""
Write-Host '$env:CEF_PATH = "' + $InstallPath + '"' -ForegroundColor Green
Write-Host '$env:PATH += ";' + $InstallPath + '"' -ForegroundColor Green
Write-Host ""

# Set for current session
$env:CEF_PATH = $InstallPath
$env:PATH += ";$InstallPath"

Write-Host "Environment variables set for current session" -ForegroundColor Green

# Verify installation
if (Test-Path "$InstallPath\libcef.dll") {
    Write-Host ""
    Write-Host "=== Installation Complete ===" -ForegroundColor Green
    Write-Host "CEF installed to: $InstallPath"

    $LibSize = (Get-Item "$InstallPath\libcef.dll").Length / 1MB
    Write-Host "libcef.dll size: $([math]::Round($LibSize, 2)) MB"
} else {
    Write-Host ""
    Write-Host "=== Installation Failed ===" -ForegroundColor Red
    Write-Host "libcef.dll not found in $InstallPath"
    exit 1
}
