$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

function Test-CommandAvailable {
    param([string]$Name)

    [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Get-MissingWindowsBuildRequirements {
    $missing = [System.Collections.Generic.List[string]]::new()

    foreach ($command in @(
        "node",
        "pnpm",
        "cargo",
        "rustup",
        "cargo-xwin",
        "python",
        "cmake",
        "ninja",
        "clang",
        "lld-link",
        "llvm-ar",
        "llvm-lib",
        "dub",
        "ldc2",
        "ldc-build-runtime",
        "link"
    )) {
        if (-not (Test-CommandAvailable $command)) {
            $missing.Add($command)
        }
    }

    if (-not (Test-Path ".\node_modules")) {
        $missing.Add("node_modules")
    }

    return $missing
}

$missing = Get-MissingWindowsBuildRequirements
if ($missing.Count -eq 0) {
    return
}

if (-not (Test-CommandAvailable "mise")) {
    throw @"
Windows host build prerequisites are missing: $($missing -join ", ")

mise is also not installed, so automatic setup cannot continue.
Install mise first, then rerun this command.
"@
}

Write-Host "Missing Windows host build prerequisites detected: $($missing -join ', ')"
Write-Host "Running: mise run setup:windows-host-build"
& mise run setup:windows-host-build
if ($LASTEXITCODE -ne 0) {
    throw "mise run setup:windows-host-build failed with exit code $LASTEXITCODE"
}

$missing = Get-MissingWindowsBuildRequirements
if ($missing.Count -gt 0) {
    throw @"
Windows host build prerequisites are still missing after setup:
$($missing -join ", ")
"@
}
