param(
    [switch]$SkipRust,
    [switch]$SkipOverlay,
    [switch]$SkipUnity,
    [switch]$SkipPackaging
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

function Invoke-QualityStep {
    param(
        [string]$Name,
        [scriptblock]$Action
    )

    Write-Host "== $Name ==" -ForegroundColor Cyan
    & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "Quality check failed: $Name"
    }
}

Push-Location $repoRoot
try {
    if (-not $SkipRust) {
        Invoke-QualityStep "cargo fmt" { cargo fmt --all -- --check }
        Invoke-QualityStep "cargo clippy" { cargo clippy --all-targets --all-features -- -D warnings }
        Invoke-QualityStep "cargo test" { cargo test --no-default-features --all-targets }
    }

    if (-not $SkipOverlay) {
        Push-Location (Join-Path $repoRoot "packages\overlay")
        try {
            Invoke-QualityStep "overlay install" { bun install --frozen-lockfile }
            Invoke-QualityStep "overlay check" { bun run check }
        }
        finally {
            Pop-Location
        }
    }

    if (-not $SkipUnity) {
        $unityFiles = @(
            Get-ChildItem -LiteralPath (Join-Path $repoRoot "hosts\unity-host") -Recurse -Filter *.cs -File |
                ForEach-Object { $_.FullName }
        )
        if ($unityFiles.Count -eq 0) {
            throw "No Unity C# source files found."
        }
        Invoke-QualityStep "Unity CSharpier" { csharpier check $unityFiles }
    }

    if (-not $SkipPackaging) {
        Invoke-QualityStep "Unity plugin contract" { & (Join-Path $repoRoot "scripts\check-plugin-contract.ps1") }
    }
}
finally {
    Pop-Location
}

Write-Host "Quality checks passed." -ForegroundColor Green
