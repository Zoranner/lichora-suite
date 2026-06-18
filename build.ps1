# Build Script for Windows
#
# Builds the headless browser for Windows and assembles dist/win-x64.

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

$DistDir = Join-Path $PSScriptRoot "dist\win-x64"

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
        $BinaryPath = Join-Path $PSScriptRoot "target\release\headless_browser.exe"
        $PdbPath = Join-Path $PSScriptRoot "target\release\headless_browser.pdb"
    } else {
        $BinaryPath = Join-Path $PSScriptRoot "target\debug\headless_browser.exe"
        $PdbPath = Join-Path $PSScriptRoot "target\debug\headless_browser.pdb"
    }

    if (Test-Path $BinaryPath) {
        Write-Host "Executable: $BinaryPath" -ForegroundColor Green
        New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
        Copy-Item -Path $BinaryPath -Destination (Join-Path $DistDir "headless_browser.exe") -Force
        Write-Host "Copied executable to: $DistDir" -ForegroundColor Green
    }
    if (Test-Path $PdbPath) {
        Copy-Item -Path $PdbPath -Destination (Join-Path $DistDir "headless_browser.pdb") -Force
    }

    Write-Host ""
    Write-Host "Target platforms: Windows and Linux." -ForegroundColor Cyan

    if (-not $NoCEF) {
        Write-Host ""
        if ($env:CEF_PATH -and (Test-Path $env:CEF_PATH)) {
            Write-Host "Copying CEF runtime to dist\win-x64..." -ForegroundColor Yellow
            $RuntimeFiles = @(
                "libcef.dll",
                "chrome_elf.dll",
                "d3dcompiler_47.dll",
                "dxcompiler.dll",
                "dxil.dll",
                "libEGL.dll",
                "libGLESv2.dll",
                "vulkan-1.dll",
                "vk_swiftshader.dll",
                "vk_swiftshader_icd.json",
                "icudtl.dat",
                "resources.pak",
                "chrome_100_percent.pak",
                "chrome_200_percent.pak",
                "v8_context_snapshot.bin"
            )
            foreach ($File in $RuntimeFiles) {
                $Source = Join-Path $env:CEF_PATH $File
                if (Test-Path $Source) {
                    Copy-Item -Path $Source -Destination (Join-Path $DistDir $File) -Force
                }
            }
            $Locales = Join-Path $env:CEF_PATH "locales"
            if (Test-Path $Locales) {
                Copy-Item -Path $Locales -Destination $DistDir -Recurse -Force
            }
            Write-Host "CEF runtime copied to: $DistDir" -ForegroundColor Green
        } else {
            Write-Host "CEF runtime was not copied because CEF_PATH is not set or does not exist." -ForegroundColor Yellow
        }
    }
} else {
    Write-Host ""
    Write-Host "=== Build Failed ===" -ForegroundColor Red
    Write-Host "Check the error messages above."
}

Write-Host ""
