$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

. (Join-Path $PSScriptRoot "import-msvc-dev-shell.ps1")

function Test-CommandAvailable {
    param([string]$Name)

    [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Import-LlvmBin {
    $candidates = @(
        (Join-Path ${env:ProgramFiles} "LLVM\bin"),
        (Join-Path ${env:ProgramFiles(x86)} "LLVM\bin"),
        (Join-Path $env:LOCALAPPDATA "Programs\LLVM\bin")
    ) | Where-Object { $_ -and (Test-Path $_) }

    foreach ($candidate in $candidates) {
        if (-not ($env:PATH -split ";" | Where-Object { $_ -eq $candidate })) {
            $env:PATH = "$candidate;$env:PATH"
        }
    }
}

function Import-LdcBin {
    $candidates = @()

    $toolsRoot = "C:\tools"
    if (Test-Path $toolsRoot) {
        $candidates += Get-ChildItem -LiteralPath $toolsRoot -Directory -Filter "ldc2-*-windows-multilib" -ErrorAction SilentlyContinue |
            Sort-Object Name -Descending |
            ForEach-Object { Join-Path $_.FullName "bin" }
    }

    $chocoLibRoot = Join-Path $env:ProgramData "chocolatey\lib"
    if (Test-Path $chocoLibRoot) {
        $candidates += Get-ChildItem -LiteralPath $chocoLibRoot -Directory -Filter "ldc*" -ErrorAction SilentlyContinue |
            ForEach-Object {
                @(
                    (Join-Path $_.FullName "tools\bin"),
                    (Join-Path $_.FullName "bin")
                )
            }
    }

    $candidates = $candidates |
        Where-Object { $_ -and (Test-Path $_) } |
        Select-Object -Unique

    foreach ($candidate in $candidates) {
        if (-not ($env:PATH -split ";" | Where-Object { $_ -eq $candidate })) {
            $env:PATH = "$candidate;$env:PATH"
        }
    }
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
        "link.exe"
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

Import-MsvcDevShell | Out-Null
Import-LlvmBin
Import-LdcBin

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

Import-MsvcDevShell | Out-Null
Import-LlvmBin
Import-LdcBin

$missing = Get-MissingWindowsBuildRequirements
if ($missing.Count -gt 0) {
    throw @"
Windows host build prerequisites are still missing after setup:
$($missing -join ", ")
"@
}
