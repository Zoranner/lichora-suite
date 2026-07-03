# CEF Environment Setup Script for Windows
#
# This script downloads and sets up CEF binaries for Windows development.

param(
    [string]$InstallPath = (Join-Path $env:LOCALAPPDATA "Lichora\cef\145.0.27-windows64"),
    [string]$Version = "145.0.27"
)

$ErrorActionPreference = "Stop"

Write-Host "=== CEF Environment Setup for Windows ===" -ForegroundColor Cyan
Write-Host "CEF Version: $Version"
Write-Host "Install Directory: $InstallPath"
Write-Host ""

New-Item -ItemType Directory -Force -Path $InstallPath | Out-Null

$CefBaseUrl = "https://cef-builds.spotifycdn.com"
$IndexUrl = "$CefBaseUrl/index.json"

Write-Host "Resolving CEF archive from:" -ForegroundColor Yellow
Write-Host $IndexUrl

try {
    $Index = Invoke-RestMethod -Uri $IndexUrl -UseBasicParsing
    $VersionEntry = $Index.windows64.versions |
        Where-Object { $_.cef_version -like "$Version+*" } |
        Select-Object -First 1

    if (-not $VersionEntry) {
        throw "CEF version '$Version' was not found in the Windows x64 index."
    }

    $ArchiveEntry = $VersionEntry.files |
        Where-Object { $_.type -eq "minimal" } |
        Select-Object -First 1

    if (-not $ArchiveEntry) {
        throw "CEF version '$($VersionEntry.cef_version)' does not have a minimal archive."
    }
} catch {
    Write-Host "Failed to resolve CEF archive." -ForegroundColor Red
    Write-Host "Reason: $($_.Exception.Message)" -ForegroundColor Yellow
    Write-Host "Manual download page: https://cef-builds.spotifycdn.com/index.html#windows64" -ForegroundColor Cyan
    exit 1
}

$ArchiveName = $ArchiveEntry.name
$EncodedArchiveName = $ArchiveName -replace "\+", "%2B"
$CefUrl = "$CefBaseUrl/$EncodedArchiveName"

Write-Host "Downloading CEF from:" -ForegroundColor Yellow
Write-Host $CefUrl

$TempRoot = Join-Path $env:TEMP "lichora_cef_setup"
$TempFile = Join-Path $TempRoot $ArchiveName
$TempDir = Join-Path $TempRoot "extract"

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
if (Test-Path $TempDir) {
    Remove-Item -Path $TempDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

$DownloadCompleted = $false
if ((Test-Path $TempFile) -and $ArchiveEntry.sha1) {
    $ExistingSha1 = (Get-FileHash -Path $TempFile -Algorithm SHA1).Hash.ToLowerInvariant()
    if ($ExistingSha1 -eq $ArchiveEntry.sha1) {
        Write-Host "Using verified cached archive: $TempFile" -ForegroundColor Green
        $DownloadCompleted = $true
    }
}

if (-not $DownloadCompleted) {
    $CurlCommand = Get-Command curl.exe -ErrorAction SilentlyContinue
    if ($CurlCommand) {
        Write-Host "Downloading with curl.exe..." -ForegroundColor Yellow
        & curl.exe --location --fail --continue-at - --retry 10 --retry-all-errors --retry-delay 5 --output $TempFile $CefUrl
        if ($LASTEXITCODE -eq 0) {
            Write-Host "Download completed" -ForegroundColor Green
            $DownloadCompleted = $true
        }
    }
}

if (-not $DownloadCompleted) {
    Remove-Item -Path $TempFile -Force -ErrorAction SilentlyContinue

    try {
        Write-Host "Downloading with Invoke-WebRequest..." -ForegroundColor Yellow
        Invoke-WebRequest -Uri $CefUrl -OutFile $TempFile -UseBasicParsing
        Write-Host "Download completed" -ForegroundColor Green
        $DownloadCompleted = $true
    } catch {
        Write-Host "Invoke-WebRequest download failed." -ForegroundColor Yellow
        Write-Host "Reason: $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

if (-not $DownloadCompleted) {
    Write-Host "Download failed. Please download manually from:" -ForegroundColor Red
    Write-Host "https://cef-builds.spotifycdn.com/index.html#windows64" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "1. Download: $ArchiveName"
    Write-Host "2. Extract it and use this script layout as reference:"
    Write-Host "   - Copy Release/* and Resources/* into: $InstallPath"
    Write-Host "   - Copy CMakeLists.txt, cmake/, include/, and libcef_dll/ into: $InstallPath"
    Write-Host "   - Create archive.json with the archive name and SHA1"
    exit 1
}

if ($ArchiveEntry.sha1) {
    Write-Host "Verifying SHA1..." -ForegroundColor Yellow
    $ActualSha1 = (Get-FileHash -Path $TempFile -Algorithm SHA1).Hash.ToLowerInvariant()
    if ($ActualSha1 -ne $ArchiveEntry.sha1) {
        Write-Host "Downloaded archive SHA1 mismatch." -ForegroundColor Red
        Write-Host "Expected: $($ArchiveEntry.sha1)" -ForegroundColor Yellow
        Write-Host "Actual:   $ActualSha1" -ForegroundColor Yellow
        exit 1
    }
}

Write-Host "Extracting CEF..." -ForegroundColor Yellow

$Extracted = $false

$TarCommand = Get-Command tar.exe -ErrorAction SilentlyContinue
if ($TarCommand) {
    & tar.exe -xjf $TempFile -C $TempDir 2>$null
    if ($LASTEXITCODE -eq 0) {
        $Extracted = $true
    } else {
        Write-Host "tar.exe extraction failed with exit code $LASTEXITCODE." -ForegroundColor Yellow
    }
}

if (-not $Extracted) {
    $SevenZipCommand = Get-Command 7z.exe, 7zz.exe, 7za.exe -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($SevenZipCommand) {
        $TarFile = Join-Path $TempRoot ($ArchiveName -replace "\.bz2$", "")
        Remove-Item -Path $TarFile -Force -ErrorAction SilentlyContinue
        & $SevenZipCommand.Source x $TempFile "-o$TempRoot" -y
        if ($LASTEXITCODE -eq 0 -and (Test-Path $TarFile)) {
            & $SevenZipCommand.Source x $TarFile "-o$TempDir" -y
            if ($LASTEXITCODE -eq 0) {
                $Extracted = $true
            }
        }
    }
}

if (-not $Extracted) {
    $PythonCommand = Get-Command python.exe, python -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($PythonCommand) {
        & $PythonCommand.Source -W ignore::DeprecationWarning -m tarfile -e $TempFile $TempDir
        if ($LASTEXITCODE -eq 0) {
            $Extracted = $true
        }
    }
}

if ($Extracted) {
    Write-Host "Extraction completed" -ForegroundColor Green
} else {
    Write-Host "Extraction failed. Please install 7-Zip or Python, or extract manually." -ForegroundColor Red
    exit 1
}

$ExtractedFolder = Get-ChildItem -Path $TempDir -Directory | Select-Object -First 1
if (-not $ExtractedFolder) {
    Write-Host "Extracted CEF folder was not found." -ForegroundColor Red
    exit 1
}

$ReleasePath = Join-Path $ExtractedFolder.FullName "Release"
$ResourcesPath = Join-Path $ExtractedFolder.FullName "Resources"
$RequiredPaths = @(
    $ReleasePath,
    $ResourcesPath,
    (Join-Path $ExtractedFolder.FullName "CMakeLists.txt"),
    (Join-Path $ExtractedFolder.FullName "cmake"),
    (Join-Path $ExtractedFolder.FullName "include"),
    (Join-Path $ExtractedFolder.FullName "libcef_dll")
)

foreach ($RequiredPath in $RequiredPaths) {
    if (-not (Test-Path $RequiredPath)) {
        Write-Host "Required CEF path was not found: $RequiredPath" -ForegroundColor Red
        exit 1
    }
}

Write-Host "Installing CEF files to $InstallPath..." -ForegroundColor Yellow
Copy-Item -Path (Join-Path $ReleasePath "*") -Destination $InstallPath -Recurse -Force
Copy-Item -Path (Join-Path $ResourcesPath "*") -Destination $InstallPath -Recurse -Force
Copy-Item -Path (Join-Path $ExtractedFolder.FullName "CMakeLists.txt") -Destination $InstallPath -Force

foreach ($DirectoryName in @("cmake", "include", "libcef_dll")) {
    $Source = Join-Path $ExtractedFolder.FullName $DirectoryName
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
$ArchiveJsonPath = Join-Path $InstallPath "archive.json"
$Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($ArchiveJsonPath, $ArchiveJson, $Utf8NoBom)

Write-Host "Files installed successfully" -ForegroundColor Green

Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "=== Environment Variables ===" -ForegroundColor Cyan
Write-Host "Add the following to your PowerShell profile or set manually:"
Write-Host ""
Write-Host "`$env:CEF_PATH = `"$InstallPath`"" -ForegroundColor Green
Write-Host "`$env:PATH += `";$InstallPath`"" -ForegroundColor Green
Write-Host ""

# Set for current session
$env:CEF_PATH = $InstallPath
$env:PATH += ";$InstallPath"

Write-Host "Environment variables set for current session" -ForegroundColor Green

$ExpectedFiles = @(
    "libcef.dll",
    "resources.pak",
    "locales",
    "CMakeLists.txt",
    "cmake",
    "include",
    "libcef_dll",
    "archive.json"
)
$MissingFiles = $ExpectedFiles | Where-Object { -not (Test-Path (Join-Path $InstallPath $_)) }

if ($MissingFiles.Count -eq 0) {
    Write-Host ""
    Write-Host "=== Installation Complete ===" -ForegroundColor Green
    Write-Host "CEF installed to: $InstallPath"

    $LibSize = (Get-Item "$InstallPath\libcef.dll").Length / 1MB
    Write-Host "libcef.dll size: $([math]::Round($LibSize, 2)) MB"
} else {
    Write-Host ""
    Write-Host "=== Installation Failed ===" -ForegroundColor Red
    Write-Host "Missing required paths:"
    foreach ($MissingFile in $MissingFiles) {
        Write-Host "  - $MissingFile" -ForegroundColor Yellow
    }
    exit 1
}
