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
$DefaultCefPath = Join-Path $env:LOCALAPPDATA "Lichora\cef\145.0.27-windows64"
$RequiredCefLayoutPaths = @(
    "libcef.dll",
    "resources.pak",
    "locales",
    "CMakeLists.txt",
    "cmake",
    "include",
    "libcef_dll",
    "archive.json"
)
$CefRuntimeFiles = @(
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

function Test-CefLayout {
    param([string]$Path)

    if (-not $Path -or -not (Test-Path -LiteralPath $Path)) {
        return $false
    }

    foreach ($RequiredPath in $RequiredCefLayoutPaths) {
        if (-not (Test-Path -LiteralPath (Join-Path $Path $RequiredPath))) {
            return $false
        }
    }

    return $true
}

function Initialize-CefEnvironment {
    if ($env:CEF_PATH) {
        if (-not (Test-CefLayout $env:CEF_PATH)) {
            throw "CEF_PATH does not contain the full CEF build layout. Run .\setup-windows.ps1 -InstallPath `"$env:CEF_PATH`" or fix CEF_PATH."
        }

        Write-Host "CEF_PATH: $env:CEF_PATH" -ForegroundColor Green
        return
    }

    if (Test-CefLayout $DefaultCefPath) {
        $env:CEF_PATH = $DefaultCefPath
        $env:PATH += ";$DefaultCefPath"
        Write-Host "CEF_PATH: $env:CEF_PATH" -ForegroundColor Green
        return
    }

    Write-Host "CEF was not found at the default path. Running setup-windows.ps1..." -ForegroundColor Yellow
    & (Join-Path $PSScriptRoot "setup-windows.ps1") -InstallPath $DefaultCefPath

    if (-not (Test-CefLayout $DefaultCefPath)) {
        throw "CEF setup finished but the expected CEF layout is still missing: $DefaultCefPath"
    }

    $env:CEF_PATH = $DefaultCefPath
    $env:PATH += ";$DefaultCefPath"
    Write-Host "CEF_PATH: $env:CEF_PATH" -ForegroundColor Green
}

Write-Host "=== Building Lichora ===" -ForegroundColor Cyan

if (-not $NoCEF) {
    Initialize-CefEnvironment
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
        Write-Host "Copying CEF runtime to dist\win-x64..." -ForegroundColor Yellow
        $MissingRuntimeFiles = @()
        foreach ($File in $CefRuntimeFiles) {
            $Source = Join-Path $env:CEF_PATH $File
            if (Test-Path -LiteralPath $Source) {
                Copy-Item -LiteralPath $Source -Destination (Join-Path $DistDir $File) -Force
            } else {
                $MissingRuntimeFiles += $File
            }
        }

        $Locales = Join-Path $env:CEF_PATH "locales"
        if (Test-Path -LiteralPath $Locales) {
            Copy-Item -LiteralPath $Locales -Destination $DistDir -Recurse -Force
        } else {
            $MissingRuntimeFiles += "locales"
        }

        if ($MissingRuntimeFiles.Count -gt 0) {
            throw "CEF runtime is incomplete. Missing: $($MissingRuntimeFiles -join ', ')"
        }

        Write-Host "CEF runtime copied to: $DistDir" -ForegroundColor Green
    }
} else {
    Write-Host ""
    Write-Host "=== Build Failed ===" -ForegroundColor Red
    Write-Host "Check the error messages above."
}

Write-Host ""
