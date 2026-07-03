# Build Script for Windows
#
# Builds Lichora for Windows and assembles dist/win-x64.

param(
    [switch]$Release,
    [switch]$NoCEF,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

if ($Help) {
    Write-Host "Lichora Build Script for Windows" -ForegroundColor Cyan
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

Write-Host "=== Building Lichora ===" -ForegroundColor Cyan

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
$BuildArgs = @("build", "-p", "lichora", "-p", "lichora-ipc-native", "-p", "process-host")

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
        $TargetDir = Join-Path $PSScriptRoot "target\release"
    } else {
        $TargetDir = Join-Path $PSScriptRoot "target\debug"
    }

    $BinaryPath = Join-Path $TargetDir "lichora.exe"
    $PdbPaths = @(
        (Join-Path $TargetDir "lichora.pdb"),
        (Join-Path $TargetDir "lichora_core.pdb"),
        (Join-Path $TargetDir "lichora_ipc_native.pdb"),
        (Join-Path $TargetDir "process_host.pdb")
    )
    $IpcNativePath = Join-Path $TargetDir "lichora_ipc_native.dll"
    $IpcNativeOutputName = "lichora_ipc_native.dll"
    $ProcessHostPath = Join-Path $TargetDir "process_host.dll"
    $UnityPluginDir = Join-Path $PSScriptRoot "hosts\unity-host\Plugins\Windows"

    if (Test-Path $BinaryPath) {
        Write-Host "Executable: $BinaryPath" -ForegroundColor Green
        New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
        Copy-Item -Path $BinaryPath -Destination (Join-Path $DistDir "lichora.exe") -Force
        Write-Host "Copied executable to: $DistDir" -ForegroundColor Green
    }
    foreach ($PdbPath in $PdbPaths) {
        if (Test-Path $PdbPath) {
            Copy-Item -Path $PdbPath -Destination (Join-Path $DistDir (Split-Path $PdbPath -Leaf)) -Force
        }
    }
    if (Test-Path $IpcNativePath) {
        New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
        Copy-Item -Path $IpcNativePath -Destination (Join-Path $DistDir $IpcNativeOutputName) -Force
        Write-Host "Copied IPC native DLL to: $DistDir\$IpcNativeOutputName" -ForegroundColor Green

        if (Test-Path $UnityPluginDir) {
            $UnityPluginPath = Join-Path $UnityPluginDir $IpcNativeOutputName
            try {
                Copy-Item -Path $IpcNativePath -Destination $UnityPluginPath -Force
                Write-Host "Copied IPC native DLL to Unity plugin dir: $UnityPluginPath" -ForegroundColor Green
            } catch {
                Write-Host "Warning: IPC native DLL was built and copied to dist, but Unity plugin DLL could not be updated." -ForegroundColor Yellow
                Write-Host "Path: $UnityPluginPath" -ForegroundColor Yellow
                Write-Host "Reason: $($_.Exception.Message)" -ForegroundColor Yellow
            }
        } else {
            Write-Host "Unity plugin dir not found; IPC native DLL was not copied to hosts/unity-host." -ForegroundColor Yellow
        }
    } else {
        Write-Host "Warning: IPC native DLL not found at $IpcNativePath" -ForegroundColor Yellow
    }

    if (Test-Path $ProcessHostPath) {
        New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
        Copy-Item -Path $ProcessHostPath -Destination (Join-Path $DistDir "process_host.dll") -Force
        Write-Host "Copied process host DLL to: $DistDir\process_host.dll" -ForegroundColor Green

        if (Test-Path $UnityPluginDir) {
            $UnityProcessHostPath = Join-Path $UnityPluginDir "process_host.dll"
            try {
                Copy-Item -Path $ProcessHostPath -Destination $UnityProcessHostPath -Force
                Write-Host "Copied process host DLL to Unity plugin dir: $UnityProcessHostPath" -ForegroundColor Green
            } catch {
                Write-Host "Warning: process host DLL was built and copied to dist, but Unity plugin DLL could not be updated." -ForegroundColor Yellow
                Write-Host "Path: $UnityProcessHostPath" -ForegroundColor Yellow
                Write-Host "Reason: $($_.Exception.Message)" -ForegroundColor Yellow
            }
        }
    } else {
        Write-Host "Warning: process host DLL not found at $ProcessHostPath" -ForegroundColor Yellow
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
