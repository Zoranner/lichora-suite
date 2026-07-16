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
    Write-Host "  -Release        Build in release mode (optimized)"
    Write-Host "  -NoCEF          Build without CEF dependency (stub mode)"
    Write-Host "  -Help           Show this help message"
    Write-Host ""
    Write-Host "Examples:"
    Write-Host "  .\build.ps1              # Debug build with CEF"
    Write-Host "  .\build.ps1 -Release     # Release build with CEF"
    Write-Host "  .\build.ps1 -NoCEF       # Debug build without CEF"
    exit 0
}

$DistDir = Join-Path $PSScriptRoot "dist\win-x64"
$CefVersion = "145.0.27"
$DefaultCefPath = Join-Path $env:LOCALAPPDATA "Lichora\cef\145.0.27-windows64"
$CefBaseUrl = "https://cef-builds.spotifycdn.com"
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

function Get-CefArchiveEntry {
    Write-Host "Resolving CEF $CefVersion Windows x64 archive..." -ForegroundColor Yellow
    $Index = Invoke-RestMethod -Uri "$CefBaseUrl/index.json" -UseBasicParsing
    $VersionEntry = $Index.windows64.versions |
        Where-Object { $_.cef_version -like "$CefVersion+*" } |
        Select-Object -First 1

    if (-not $VersionEntry) {
        throw "CEF version '$CefVersion' was not found in the Windows x64 index."
    }

    $ArchiveEntry = $VersionEntry.files |
        Where-Object { $_.type -eq "minimal" } |
        Select-Object -First 1

    if (-not $ArchiveEntry) {
        throw "CEF version '$($VersionEntry.cef_version)' does not have a minimal archive."
    }

    return $ArchiveEntry
}

function Save-CefArchive {
    param([object]$ArchiveEntry)

    $ArchiveName = $ArchiveEntry.name
    $EncodedArchiveName = $ArchiveName -replace "\+", "%2B"
    $CefUrl = "$CefBaseUrl/$EncodedArchiveName"
    $TempRoot = Join-Path $env:TEMP "lichora_cef_setup"
    $TempFile = Join-Path $TempRoot $ArchiveName

    New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

    if ((Test-Path $TempFile) -and $ArchiveEntry.sha1) {
        $ExistingSha1 = (Get-FileHash -Path $TempFile -Algorithm SHA1).Hash.ToLowerInvariant()
        if ($ExistingSha1 -eq $ArchiveEntry.sha1) {
            Write-Host "Using verified cached CEF archive: $TempFile" -ForegroundColor Green
            return $TempFile
        }
    }

    Write-Host "Downloading CEF from: $CefUrl" -ForegroundColor Yellow
    $DownloadCompleted = $false
    $CurlCommand = Get-Command curl.exe -ErrorAction SilentlyContinue
    if ($CurlCommand) {
        & curl.exe --location --fail --continue-at - --retry 10 --retry-all-errors --retry-delay 5 --output $TempFile $CefUrl
        $DownloadCompleted = $LASTEXITCODE -eq 0
    }

    if (-not $DownloadCompleted) {
        Remove-Item -Path $TempFile -Force -ErrorAction SilentlyContinue
        Invoke-WebRequest -Uri $CefUrl -OutFile $TempFile -UseBasicParsing
    }

    if ($ArchiveEntry.sha1) {
        $ActualSha1 = (Get-FileHash -Path $TempFile -Algorithm SHA1).Hash.ToLowerInvariant()
        if ($ActualSha1 -ne $ArchiveEntry.sha1) {
            throw "Downloaded CEF archive SHA1 mismatch. Expected $($ArchiveEntry.sha1), got $ActualSha1."
        }
    }

    return $TempFile
}

function Expand-CefArchive {
    param([string]$ArchivePath)

    $TempRoot = Split-Path -Parent $ArchivePath
    $TempDir = Join-Path $TempRoot "extract"
    if (Test-Path $TempDir) {
        Remove-Item -Path $TempDir -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

    $Extracted = $false
    $TarCommand = Get-Command tar.exe -ErrorAction SilentlyContinue
    if ($TarCommand) {
        & tar.exe -xjf $ArchivePath -C $TempDir 2>$null
        $Extracted = $LASTEXITCODE -eq 0
    }

    if (-not $Extracted) {
        $SevenZipCommand = Get-Command 7z.exe, 7zz.exe, 7za.exe -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($SevenZipCommand) {
            $TarFile = Join-Path $TempRoot ((Split-Path -Leaf $ArchivePath) -replace "\.bz2$", "")
            Remove-Item -Path $TarFile -Force -ErrorAction SilentlyContinue
            & $SevenZipCommand.Source x $ArchivePath "-o$TempRoot" -y
            if ($LASTEXITCODE -eq 0 -and (Test-Path $TarFile)) {
                & $SevenZipCommand.Source x $TarFile "-o$TempDir" -y
                $Extracted = $LASTEXITCODE -eq 0
            }
        }
    }

    if (-not $Extracted) {
        $PythonCommand = Get-Command python.exe, python -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($PythonCommand) {
            & $PythonCommand.Source -W ignore::DeprecationWarning -m tarfile -e $ArchivePath $TempDir
            $Extracted = $LASTEXITCODE -eq 0
        }
    }

    if (-not $Extracted) {
        throw "CEF extraction failed. Install 7-Zip or Python, or provide a valid CEF_PATH."
    }

    $ExtractedFolder = Get-ChildItem -Path $TempDir -Directory | Select-Object -First 1
    if (-not $ExtractedFolder) {
        throw "Extracted CEF folder was not found."
    }

    return $ExtractedFolder.FullName
}

function Install-CefLayout {
    param(
        [object]$ArchiveEntry,
        [string]$ExtractedPath,
        [string]$InstallPath
    )

    $ReleasePath = Join-Path $ExtractedPath "Release"
    $ResourcesPath = Join-Path $ExtractedPath "Resources"
    foreach ($RequiredPath in @(
        $ReleasePath,
        $ResourcesPath,
        (Join-Path $ExtractedPath "CMakeLists.txt"),
        (Join-Path $ExtractedPath "cmake"),
        (Join-Path $ExtractedPath "include"),
        (Join-Path $ExtractedPath "libcef_dll")
    )) {
        if (-not (Test-Path $RequiredPath)) {
            throw "Required CEF path was not found: $RequiredPath"
        }
    }

    Write-Host "Installing CEF files to $InstallPath..." -ForegroundColor Yellow
    New-Item -ItemType Directory -Force -Path $InstallPath | Out-Null
    Copy-Item -Path (Join-Path $ReleasePath "*") -Destination $InstallPath -Recurse -Force
    Copy-Item -Path (Join-Path $ResourcesPath "*") -Destination $InstallPath -Recurse -Force
    Copy-Item -Path (Join-Path $ExtractedPath "CMakeLists.txt") -Destination $InstallPath -Force

    foreach ($DirectoryName in @("cmake", "include", "libcef_dll")) {
        $Source = Join-Path $ExtractedPath $DirectoryName
        $Destination = Join-Path $InstallPath $DirectoryName
        if (Test-Path $Destination) {
            Remove-Item -Path $Destination -Recurse -Force
        }
        Copy-Item -Path $Source -Destination $Destination -Recurse -Force
    }

    $ArchiveJson = [ordered]@{
        type = $ArchiveEntry.type
        name = $ArchiveEntry.name
        sha1 = $ArchiveEntry.sha1
    } | ConvertTo-Json
    $Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText((Join-Path $InstallPath "archive.json"), $ArchiveJson, $Utf8NoBom)
}

function Install-DefaultCef {
    $ArchiveEntry = Get-CefArchiveEntry
    $ArchivePath = Save-CefArchive $ArchiveEntry
    $ExtractedPath = Expand-CefArchive $ArchivePath
    Install-CefLayout -ArchiveEntry $ArchiveEntry -ExtractedPath $ExtractedPath -InstallPath $DefaultCefPath
    Remove-Item -Path (Join-Path (Split-Path -Parent $ArchivePath) "extract") -Recurse -Force -ErrorAction SilentlyContinue
}

function Initialize-CefEnvironment {
    if ($env:CEF_PATH) {
        if (-not (Test-CefLayout $env:CEF_PATH)) {
            throw "CEF_PATH does not contain the full CEF build layout. Fix CEF_PATH or unset it so build.ps1 can install the default CEF package."
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

    Write-Host "CEF was not found at the default path. Installing CEF..." -ForegroundColor Yellow
    Install-DefaultCef

    if (-not (Test-CefLayout $DefaultCefPath)) {
        throw "CEF install finished but the expected CEF layout is still missing: $DefaultCefPath"
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
    } else {
        Write-Host "Warning: IPC native DLL not found at $IpcNativePath" -ForegroundColor Yellow
    }

    if (Test-Path $ProcessHostPath) {
        New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
        Copy-Item -Path $ProcessHostPath -Destination (Join-Path $DistDir "process_host.dll") -Force
        Write-Host "Copied process host DLL to: $DistDir\process_host.dll" -ForegroundColor Green
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
