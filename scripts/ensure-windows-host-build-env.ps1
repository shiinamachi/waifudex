$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$LdcInstallRoot = Join-Path $env:LOCALAPPDATA "waifudex-tools\ldc2"

function Test-CommandAvailable {
    param([string]$Name)

    [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Test-CommandRuns {
    param(
        [string]$Command,
        [string[]]$Arguments = @()
    )

    $previousNativeErrorPreference = $PSNativeCommandUseErrorActionPreference
    $previousErrorAction = $ErrorActionPreference
    $PSNativeCommandUseErrorActionPreference = $false
    $ErrorActionPreference = "Continue"

    try {
        & $Command @Arguments *> $null
        $LASTEXITCODE -eq 0
    }
    catch {
        $false
    }
    finally {
        $PSNativeCommandUseErrorActionPreference = $previousNativeErrorPreference
        $ErrorActionPreference = $previousErrorAction
    }
}

function Resolve-PythonCommand {
    $candidates = [System.Collections.Generic.List[object]]::new()

    if (Test-CommandAvailable "mise") {
        $candidates.Add([pscustomobject]@{
                Command   = "mise"
                Arguments = @("exec", "--", "python")
            })
    }

    if (Test-CommandAvailable "python") {
        $candidates.Add([pscustomobject]@{
                Command   = "python"
                Arguments = @()
            })
    }

    if (Test-CommandAvailable "py") {
        $candidates.Add([pscustomobject]@{
                Command   = "py"
                Arguments = @("-3")
            })
    }

    if (Test-CommandAvailable "python3") {
        $candidates.Add([pscustomobject]@{
                Command   = "python3"
                Arguments = @()
            })
    }

    foreach ($candidate in $candidates) {
        if (Test-CommandRuns -Command $candidate.Command -Arguments ($candidate.Arguments + @("--version"))) {
            return $candidate
        }
    }

    return $null
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

    $localLdcBin = Join-Path $LdcInstallRoot "bin"
    if (Test-Path $localLdcBin) {
        $candidates += $localLdcBin
    }

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
        "cmake",
        "ninja",
        "clang",
        "clang-cl",
        "lld-link",
        "llvm-ar",
        "llvm-lib",
        "dub",
        "ldc2",
        "ldc-build-runtime"
    )) {
        if (-not (Test-CommandAvailable $command)) {
            $missing.Add($command)
        }
    }

    if (-not (Resolve-PythonCommand)) {
        $missing.Add("python-runtime")
    }

    if (-not (Test-Path ".\node_modules")) {
        $missing.Add("node_modules")
    }

    $xwinDir = if ($env:XWIN_DIR) {
        $env:XWIN_DIR
    } elseif ($env:XWIN_CACHE_DIR) {
        $env:XWIN_CACHE_DIR
    } elseif ($env:LOCALAPPDATA) {
        Join-Path $env:LOCALAPPDATA "cargo-xwin\xwin"
    } else {
        Join-Path $env:USERPROFILE ".cache\cargo-xwin\xwin"
    }

    if (-not (Test-Path (Join-Path $xwinDir "crt\include")) -or -not (Test-Path (Join-Path $xwinDir "sdk\lib\um\x86_64"))) {
        $missing.Add("cargo-xwin-sysroot")
    }

    return $missing
}

Import-LlvmBin
Import-LdcBin

if (-not $env:XWIN_CLANG_CL) {
    $clangCl = Get-Command "clang-cl" -ErrorAction SilentlyContinue
    if ($clangCl) {
        $env:XWIN_CLANG_CL = $clangCl.Source
    }
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

Import-LlvmBin
Import-LdcBin

$missing = Get-MissingWindowsBuildRequirements
if ($missing.Count -gt 0) {
    throw @"
Windows host build prerequisites are still missing after setup:
$($missing -join ", ")
"@
}
