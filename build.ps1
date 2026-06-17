# Build Script for Windows
#
# Builds the headless browser for Windows (for testing only)

param(
    [switch]$Release,
    [switch]$NoCEF,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Headless Browser Build Script for Windows" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Usage: .\build.ps1 [OPTIONS]"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Release    Build in release mode (optimized)"
    Write-Host "  -NoCEF      Build without CEF dependency (stub mode)"
    Write-Host "  -Help       Show this help message"
    Write-Host ""
    Write-Host "Examples:"
    Write-Host "  .\build.ps1              # Debug build with CEF"
    Write-Host "  .\build.ps1 -Release     # Release build with CEF"
    Write-Host "  .\build.ps1 -NoCEF       # Debug build without CEF"
    exit 0
}

Write-Host "=== Building Headless Browser ===" -ForegroundColor Cyan

# Check CEF environment
if (-not $NoCEF) {
    if (-not $env:CEF_PATH) {
        Write-Host "Warning: CEF_PATH environment variable not set" -ForegroundColor Yellow
        Write-Host "Run .\setup-windows.ps1 first or set CEF_PATH manually" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "To build without CEF, use: .\build.ps1 -NoCEF" -ForegroundColor Cyan
        Write-Host ""
    } else {
        Write-Host "CEF_PATH: $env:CEF_PATH" -ForegroundColor Green

        if (-not (Test-Path "$env:CEF_PATH\libcef.dll")) {
            Write-Host "Warning: libcef.dll not found in CEF_PATH" -ForegroundColor Yellow
        }
    }
}

# Build command
$BuildArgs = @("build")

if ($Release) {
    $BuildArgs += "--release"
    $Mode = "Release"
} else {
    $Mode = "Debug"
}

if ($NoCEF) {
    $BuildArgs += "--no-default-features"
    $Features = "No CEF (stub mode)"
} else {
    $Features = "CEF enabled"
}

Write-Host "Build Mode: $Mode" -ForegroundColor Yellow
Write-Host "Features: $Features" -ForegroundColor Yellow
Write-Host ""

# Run build
Write-Host "Running: cargo $BuildArgs" -ForegroundColor Yellow
& cargo $BuildArgs

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "=== Build Complete ===" -ForegroundColor Green

    if ($Release) {
        $BinaryPath = ".\target\release\headless_browser.exe"
        $DllPath = ".\target\release\headless_browser_rust.dll"
    } else {
        $BinaryPath = ".\target\debug\headless_browser.exe"
        $DllPath = ".\target\debug\headless_browser_rust.dll"
    }

    if (Test-Path $BinaryPath) {
        Write-Host "Executable: $BinaryPath" -ForegroundColor Green
    }
    if (Test-Path $DllPath) {
        Write-Host "Library: $DllPath" -ForegroundColor Green
    }

    Write-Host ""
    Write-Host "Target platforms: Windows and Linux." -ForegroundColor Cyan

    if (-not $NoCEF) {
        Write-Host ""
        Write-Host "To run, ensure CEF DLLs are in PATH or copy them to the output directory." -ForegroundColor Yellow
    }
} else {
    Write-Host ""
    Write-Host "=== Build Failed ===" -ForegroundColor Red
    Write-Host "Check the error messages above."
}

Write-Host ""
