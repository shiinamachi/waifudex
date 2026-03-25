$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$LdcVersion = "1.40.1"
$LdcArchiveName = "ldc2-$LdcVersion-windows-multilib"
$LdcDownloadUrl = "https://github.com/ldc-developers/ldc/releases/download/v$LdcVersion/$LdcArchiveName.zip"
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

function Refresh-PathFromMachine {
    $machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $segments = @()
    if ($machinePath) {
        $segments += $machinePath
    }
    if ($userPath) {
        $segments += $userPath
    }
    if ($env:PATH) {
        $segments += $env:PATH
    }

    $env:PATH = ($segments -join ";")
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

function Test-WingetPackageInstalled {
    param([string]$Id)

    & winget list -e --id $Id --accept-source-agreements | Out-Null
    $LASTEXITCODE -eq 0
}

function Install-WingetPackage {
    param(
        [string]$Id,
        [string[]]$Arguments = @()
    )

    if (-not (Test-CommandAvailable "winget")) {
        throw "winget is required to install Windows build prerequisites automatically."
    }

    $baseCommand = @(
        "install",
        "-e",
        "--id", $Id,
        "--accept-source-agreements",
        "--accept-package-agreements"
    ) + $Arguments

    $command = $baseCommand + @("--scope", "user")
    & winget @command
    if ($LASTEXITCODE -ne 0) {
        if (Test-WingetPackageInstalled $Id) {
            Refresh-PathFromMachine
            return
        }

        Write-Host "User-scope install failed for $Id; retrying without --scope user..."
        & winget @baseCommand
        if ($LASTEXITCODE -ne 0) {
            if (Test-WingetPackageInstalled $Id) {
                Refresh-PathFromMachine
                return
            }

            throw "winget install failed for package $Id with exit code $LASTEXITCODE"
        }
    }

    Refresh-PathFromMachine
}

function Install-LdcDirect {
    if ((Test-CommandAvailable "ldc2") -and (Test-CommandAvailable "dub") -and (Test-CommandAvailable "ldc-build-runtime")) {
        return
    }

    $localLdcBin = Join-Path $LdcInstallRoot "bin"
    if (Test-Path (Join-Path $localLdcBin "ldc2.exe")) {
        Import-LdcBin
        return
    }

    Write-Host "Downloading LDC $LdcVersion from GitHub..."
    $tempZip = Join-Path ([System.IO.Path]::GetTempPath()) "$LdcArchiveName.zip"
    $tempExtract = Join-Path ([System.IO.Path]::GetTempPath()) "ldc-extract-$([System.IO.Path]::GetRandomFileName())"

    try {
        [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072
        Invoke-WebRequest -Uri $LdcDownloadUrl -OutFile $tempZip -UseBasicParsing
        if (-not (Test-Path $tempZip)) {
            throw "Failed to download LDC archive from $LdcDownloadUrl"
        }

        Write-Host "Extracting LDC to $LdcInstallRoot..."
        New-Item -ItemType Directory -Force -Path $tempExtract | Out-Null
        Expand-Archive -LiteralPath $tempZip -DestinationPath $tempExtract -Force

        $extractedDir = Get-ChildItem -LiteralPath $tempExtract -Directory | Select-Object -First 1
        if (-not $extractedDir) {
            throw "LDC archive did not contain a top-level directory"
        }

        if (Test-Path $LdcInstallRoot) {
            Remove-Item $LdcInstallRoot -Recurse -Force
        }
        $parentDir = Split-Path -Parent $LdcInstallRoot
        if (-not (Test-Path $parentDir)) {
            New-Item -ItemType Directory -Force -Path $parentDir | Out-Null
        }
        Move-Item -LiteralPath $extractedDir.FullName -Destination $LdcInstallRoot -Force
    }
    finally {
        Remove-Item $tempZip -Force -ErrorAction SilentlyContinue
        Remove-Item $tempExtract -Recurse -Force -ErrorAction SilentlyContinue
    }

    Import-LdcBin
}

& mise install
if ($LASTEXITCODE -ne 0) {
    throw "mise install failed with exit code $LASTEXITCODE"
}

Refresh-PathFromMachine
Import-LlvmBin
Import-LdcBin

if (-not (Test-Path ".\node_modules")) {
    & pnpm install
    if ($LASTEXITCODE -ne 0) {
        throw "pnpm install failed with exit code $LASTEXITCODE"
    }
}

if (Test-CommandAvailable "rustup") {
    & rustup target add x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) {
        throw "rustup target add x86_64-pc-windows-msvc failed with exit code $LASTEXITCODE"
    }
}

if (-not (Test-CommandAvailable "cmake")) {
    Install-WingetPackage -Id "Kitware.CMake"
}

if (-not (Test-CommandAvailable "ninja")) {
    Install-WingetPackage -Id "Ninja-build.Ninja"
}

if (-not (Test-CommandAvailable "clang") -or -not (Test-CommandAvailable "llvm-lib") -or -not (Test-CommandAvailable "lld-link")) {
    Install-WingetPackage -Id "LLVM.LLVM"
    Refresh-PathFromMachine
    Import-LlvmBin
}

if (-not (Test-CommandAvailable "cargo-xwin")) {
    & cargo install --locked cargo-xwin
    if ($LASTEXITCODE -ne 0) {
        throw "cargo install --locked cargo-xwin failed with exit code $LASTEXITCODE"
    }
}

Install-LdcDirect

if (-not (Test-CommandAvailable "clang") -or -not (Test-CommandAvailable "llvm-lib") -or -not (Test-CommandAvailable "lld-link")) {
    throw @"
LLVM installation did not expose the required tools in the current environment.
Missing one or more of: clang, llvm-lib, lld-link.
Open a new shell if LLVM was just installed, or verify the LLVM winget installation.
"@
}

if (-not (Test-CommandAvailable "ldc2") -or -not (Test-CommandAvailable "ldc-build-runtime")) {
    throw @"
Failed to install LDC automatically.
Install an LDC distribution that provides both ldc2 and ldc-build-runtime, then rerun the command.
"@
}

if (-not (Resolve-PythonCommand)) {
    throw "Failed to provision a usable Python runtime. Run mise install again or verify the Python launcher on this Windows host."
}

& cargo xwin env --target x86_64-pc-windows-msvc | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "cargo xwin env --target x86_64-pc-windows-msvc failed with exit code $LASTEXITCODE"
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
    & cargo xwin cache xwin --cross-compiler clang
    if ($LASTEXITCODE -ne 0) {
        throw "cargo xwin cache xwin --cross-compiler clang failed with exit code $LASTEXITCODE"
    }
}

if (-not (Test-Path (Join-Path $xwinDir "crt\include")) -or -not (Test-Path (Join-Path $xwinDir "sdk\lib\um\x86_64"))) {
    throw "cargo-xwin sysroot is still missing after setup at $xwinDir"
}
