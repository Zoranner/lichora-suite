param(
    [switch]$SkipRust
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
        Invoke-QualityStep "cargo test" {
            $PreviousRustFlags = $env:RUSTFLAGS
            try {
                if ($PreviousRustFlags) {
                    $env:RUSTFLAGS = "$PreviousRustFlags -Dwarnings"
                } else {
                    $env:RUSTFLAGS = "-Dwarnings"
                }
                cargo test --no-default-features --all-targets
            }
            finally {
                $env:RUSTFLAGS = $PreviousRustFlags
            }
        }
    }

}
finally {
    Pop-Location
}

Write-Host "Quality checks passed." -ForegroundColor Green
