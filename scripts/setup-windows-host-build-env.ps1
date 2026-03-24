$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

. (Join-Path $PSScriptRoot "import-msvc-dev-shell.ps1")

function Test-CommandAvailable {
    param([string]$Name)

    [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Install-WingetPackage {
    param(
        [string]$Id,
        [string[]]$Arguments = @()
    )

    if (-not (Test-CommandAvailable "winget")) {
        throw "winget is required to install Windows build prerequisites automatically."
    }

    $command = @(
        "install",
        "-e",
        "--id", $Id,
        "--accept-source-agreements",
        "--accept-package-agreements"
    ) + $Arguments

    & winget @command
    if ($LASTEXITCODE -ne 0) {
        throw "winget install failed for package $Id with exit code $LASTEXITCODE"
    }
}

& mise install
if ($LASTEXITCODE -ne 0) {
    throw "mise install failed with exit code $LASTEXITCODE"
}

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

if (-not (Test-CommandAvailable "cargo-xwin")) {
    & cargo install --locked cargo-xwin
    if ($LASTEXITCODE -ne 0) {
        throw "cargo install --locked cargo-xwin failed with exit code $LASTEXITCODE"
    }
}

if (-not (Test-CommandAvailable "python")) {
    Install-WingetPackage -Id "Python.Python.3.12"
}

if (-not (Test-CommandAvailable "cmake")) {
    Install-WingetPackage -Id "Kitware.CMake"
}

if (-not (Test-CommandAvailable "ninja")) {
    Install-WingetPackage -Id "Ninja-build.Ninja"
}

if (-not (Test-CommandAvailable "clang") -or -not (Test-CommandAvailable "llvm-lib") -or -not (Test-CommandAvailable "lld-link")) {
    Install-WingetPackage -Id "LLVM.LLVM"
}

if (-not (Test-CommandAvailable "link.exe")) {
    Install-WingetPackage -Id "Microsoft.VisualStudio.2022.BuildTools" -Arguments @(
        "--override",
        "--wait --quiet --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    )
}

$importedMsvc = Import-MsvcDevShell
if (-not $importedMsvc) {
    throw "Visual Studio Build Tools may not be installed or discoverable. Could not import the MSVC developer shell."
}

if (-not (Test-CommandAvailable "dub")) {
    Install-WingetPackage -Id "Dlang.DMD"
}

if (-not (Test-CommandAvailable "ldc2") -or -not (Test-CommandAvailable "ldc-build-runtime")) {
    try {
        Install-WingetPackage -Id "LLVM.LDC"
    }
    catch {
        throw @"
Failed to install LDC automatically.
Install an LDC distribution that provides both ldc2 and ldc-build-runtime, then rerun the command.
"@
    }
}

if (-not (Test-CommandAvailable "link.exe")) {
    throw "Visual Studio Build Tools were installed, but link.exe is still not available in the current environment. Open a new shell or verify the VC++ workload installation."
}

& cargo xwin env --target x86_64-pc-windows-msvc | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "cargo xwin env --target x86_64-pc-windows-msvc failed with exit code $LASTEXITCODE"
}
