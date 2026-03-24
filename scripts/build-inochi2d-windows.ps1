$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

. (Join-Path $PSScriptRoot "import-msvc-dev-shell.ps1")

$root = Split-Path -Parent $PSScriptRoot
$sourceDir = Join-Path $root "third_party\inochi2d-c"
$outDir = Join-Path $sourceDir "out"
$targetTriple = "x86_64-pc-windows-msvc"
$homeDir = if ($env:HOME) { $env:HOME } else { $env:USERPROFILE }
$xwinDir = if ($env:XWIN_DIR) { $env:XWIN_DIR } else { Join-Path $homeDir ".cache\cargo-xwin\xwin" }
$waifudexCacheDir = if ($env:XDG_CACHE_HOME) {
    Join-Path $env:XDG_CACHE_HOME "waifudex\inochi2d-windows"
} else {
    Join-Path $homeDir ".cache\waifudex\inochi2d-windows"
}
$runtimeBuildDir = Join-Path $waifudexCacheDir "ldc-runtime"
$runtimeLibDir = Join-Path $waifudexCacheDir "runtime-lib"
$workDir = Join-Path $waifudexCacheDir "work"
$wrapperDir = Join-Path $waifudexCacheDir "dub-wrapper"

$ensureScript = Join-Path $PSScriptRoot "ensure-windows-host-build-env.ps1"
if (Test-Path $ensureScript) {
    & $ensureScript
}

Import-MsvcDevShell | Out-Null

function Require-Command {
    param([string]$Name)

    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $command) {
        throw "missing required tool: $Name"
    }

    $command.Source
}

function Invoke-PythonScript {
    param(
        [string]$Script,
        [string[]]$Arguments,
        [hashtable]$Environment = @{}
    )

    $tempFile = [System.IO.Path]::GetTempFileName()
    $scriptPath = [System.IO.Path]::ChangeExtension($tempFile, ".py")
    Remove-Item $tempFile -Force
    Set-Content -LiteralPath $scriptPath -Value $Script -Encoding utf8

    try {
        $currentEnvironment = @{}
        foreach ($entry in $Environment.GetEnumerator()) {
            $currentEnvironment[$entry.Key] = [Environment]::GetEnvironmentVariable($entry.Key)
            [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value)
        }

        & python $scriptPath @Arguments
        if ($LASTEXITCODE -ne 0) {
            throw "python script failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        foreach ($entry in $Environment.GetEnumerator()) {
            [Environment]::SetEnvironmentVariable($entry.Key, $currentEnvironment[$entry.Key])
        }
        Remove-Item $scriptPath -Force -ErrorAction SilentlyContinue
    }
}

function Ensure-HostGitver {
    param(
        [string]$Dub,
        [string]$Ldc2
    )

    $hostGitverDir = Join-Path $homeDir ".dub\packages\gitver\1.7.2\gitver"
    $hostGitverBin = Join-Path $hostGitverDir "out\gitver"

    New-Item -ItemType Directory -Force -Path $wrapperDir | Out-Null
    Push-Location $hostGitverDir
    try {
        & $Dub build "--compiler=$Ldc2" --force | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "failed to build host gitver"
        }
    }
    finally {
        Pop-Location
    }

    $wrapperPath = Join-Path $wrapperDir "dub.cmd"
    @"
@echo off
setlocal
set REAL_DUB=$Dub
set HOST_GITVER_BIN=$hostGitverBin
if "%1"=="run" if "%2"=="gitver" (
  shift
  shift
  if "%1"=="--" shift
  "%HOST_GITVER_BIN%" %*
  exit /b %errorlevel%
)
"%REAL_DUB%" %*
exit /b %errorlevel%
"@ | Set-Content -LiteralPath $wrapperPath -Encoding ascii
}

function Build-RuntimeLibs {
    $runtimeDFlags = "--mtriple=$targetTriple;--linker=lld-link;--mscrtlib=msvcrt;-link-defaultlib-shared=false"
    $runtimeCFlags = "--target=$targetTriple;-Wno-unused-command-line-argument;-fuse-ld=lld-link;-isystem;$xwinDir/crt/include;-isystem;$xwinDir/sdk/include/ucrt;-isystem;$xwinDir/sdk/include/um;-isystem;$xwinDir/sdk/include/shared;-isystem;$xwinDir/sdk/include/winrt"
    $runtimeLinkerFlags = "-fuse-ld=lld-link;/LIBPATH:$xwinDir/crt/lib/x86_64;/LIBPATH:$xwinDir/sdk/lib/um/x86_64;/LIBPATH:$xwinDir/sdk/lib/ucrt/x86_64"

    Remove-Item $runtimeBuildDir, $runtimeLibDir -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $runtimeBuildDir, $runtimeLibDir | Out-Null

    $previousCc = $env:CC
    $previousCxx = $env:CXX
    $env:CC = "clang"
    $env:CXX = "clang++"

    try {
        & ldc-build-runtime --ninja --buildDir $runtimeBuildDir `
            CMAKE_POLICY_VERSION_MINIMUM=3.5 `
            BUILD_SHARED_LIBS=OFF `
            HAVE_UNISTD_H=0 `
            --targetSystem "Windows;MSVC" `
            "--dFlags=$runtimeDFlags" `
            "--cFlags=$runtimeCFlags" `
            "--linkerFlags=$runtimeLinkerFlags" `
            -j2 | Out-Null

        Push-Location $runtimeBuildDir
        try {
            & ninja -k 0 "lib/libdruntime-ldc.a" "lib/libphobos2-ldc.a" | Out-Null
            if ($LASTEXITCODE -ne 0) {
                throw "failed to build ldc runtime libraries"
            }
        }
        finally {
            Pop-Location
        }
    }
    finally {
        $env:CC = $previousCc
        $env:CXX = $previousCxx
    }

    Invoke-PythonScript -Arguments @($runtimeBuildDir, $runtimeLibDir) -Script @'
import os
import shlex
import subprocess
import sys

build_dir = sys.argv[1]
out_dir = sys.argv[2]
targets = [
    ("lib/libdruntime-ldc.a", "druntime-ldc.lib"),
    ("lib/libphobos2-ldc.a", "phobos2-ldc.lib"),
]

for ninja_target, out_name in targets:
    output = subprocess.check_output(
        ["ninja", "-t", "commands", ninja_target], cwd=build_dir, text=True
    )
    link_line = [line for line in output.splitlines() if " -lib -of=" in line][-1]
    args = shlex.split(link_line)
    objects = []
    capture = False
    for arg in args:
        if arg.startswith("-of=lib/lib"):
            capture = True
            continue
        if not capture:
            continue
        if arg.startswith("-") or arg in {"&&", ":"}:
            break
        if arg.endswith(".o") or arg.endswith(".obj"):
            direct = os.path.join(build_dir, arg)
            alt = (
                os.path.join(build_dir, arg[:-2] + ".obj")
                if arg.endswith(".o")
                else direct
            )
            if os.path.exists(direct):
                objects.append(direct)
            elif os.path.exists(alt):
                objects.append(alt)
            else:
                raise SystemExit(f"missing runtime object: {direct} or {alt}")
    subprocess.run(
        ["llvm-lib", f"/OUT:{os.path.join(out_dir, out_name)}", *objects],
        check=True,
    )
'@

    foreach ($file in @("druntime-ldc.lib", "phobos2-ldc.lib")) {
        if (-not (Test-Path (Join-Path $runtimeLibDir $file))) {
            throw "missing runtime library: $file"
        }
    }
}

function Build-WindowsInochi2d {
    param(
        [string]$Ldc2,
        [string]$Dub
    )

    $importPaths = (& $Dub describe "--compiler=$Ldc2" --config=yesgl --data=import-paths --data-list)
    $stringImportPaths = (& $Dub describe "--compiler=$Ldc2" --config=yesgl --data=string-import-paths --data-list)
    $versions = (& $Dub describe "--compiler=$Ldc2" --config=yesgl --data=versions --data-list)
    $sourceFiles = (& $Dub describe "--compiler=$Ldc2" --config=yesgl --data=source-files --data-list)
    $linkerFiles = (& $Dub describe "--compiler=$Ldc2" --config=yesgl --data=linker-files --data-list)

    Remove-Item $workDir -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $workDir, $outDir | Out-Null

    $topObj = Join-Path $workDir "inochi2d-c.obj"
    $compileArgs = @(
        "-c",
        "-fvisibility=hidden",
        "-link-defaultlib-shared=false",
        "--mtriple=$targetTriple",
        "--linker=lld-link",
        "--mscrtlib=msvcrt",
        "-of=$topObj"
    )

    foreach ($line in $importPaths) {
        if ($line) { $compileArgs += "-I$line" }
    }
    foreach ($line in $stringImportPaths) {
        if ($line) { $compileArgs += "-J$line" }
    }
    foreach ($line in $versions) {
        if ($line) { $compileArgs += "-d-version=$line" }
    }
    foreach ($line in $sourceFiles) {
        if ($line) { $compileArgs += $line }
    }

    & $Ldc2 @compileArgs
    if ($LASTEXITCODE -ne 0) {
        throw "failed to compile inochi2d-c object"
    }

    Invoke-PythonScript -Arguments @($workDir) -Environment @{ LINKER_FILES = ($linkerFiles -join [Environment]::NewLine) } -Script @'
import os
import subprocess
import sys
import tempfile

work_dir = sys.argv[1]
for path in filter(None, os.environ["LINKER_FILES"].splitlines()):
    base = os.path.basename(path)
    stem = base[:-2] if base.endswith(".a") else os.path.splitext(base)[0]
    out = os.path.join(work_dir, stem + ".lib")
    with tempfile.TemporaryDirectory(dir=work_dir) as tmp:
        subprocess.run(["llvm-ar", "x", path], cwd=tmp, check=True)
        members = []
        for root, _, files in os.walk(tmp):
            for name in files:
                if name.endswith(".o") or name.endswith(".obj"):
                    members.append(os.path.join(root, name))
        subprocess.run(["llvm-lib", f"/OUT:{out}", *sorted(members)], check=True)
'@

    $dllPath = Join-Path $outDir "inochi2d-c.dll"
    $libPath = Join-Path $outDir "inochi2d-c.lib"
    Remove-Item $dllPath, $libPath -Force -ErrorAction SilentlyContinue

    $linkArgs = @(
        "--shared",
        "-link-defaultlib-shared=false",
        "--mtriple=$targetTriple",
        "--linker=lld-link",
        "--mscrtlib=msvcrt",
        "-of=$dllPath",
        $topObj,
        "-L/IMPLIB:$libPath",
        "-L/LIBPATH:$runtimeLibDir",
        "-L/LIBPATH:$xwinDir/crt/lib/x86_64",
        "-L/LIBPATH:$xwinDir/sdk/lib/um/x86_64",
        "-L/LIBPATH:$xwinDir/sdk/lib/ucrt/x86_64"
    )

    Get-ChildItem -LiteralPath $workDir -Filter *.lib | ForEach-Object {
        $linkArgs += $_.FullName
    }

    & $Ldc2 @linkArgs
    if ($LASTEXITCODE -ne 0) {
        throw "failed to link inochi2d-c windows artifacts"
    }

    foreach ($file in @($dllPath, $libPath)) {
        if (-not (Test-Path $file)) {
            throw "missing build output: $file"
        }
    }
}

if (-not (Test-Path $sourceDir)) {
    throw "missing submodule: $sourceDir`nrun: git submodule update --init --recursive"
}

$cargo = Require-Command "cargo"
Require-Command "cargo-xwin" | Out-Null
$dub = Require-Command "dub"
$ldc2 = Require-Command "ldc2"
Require-Command "ldc-build-runtime" | Out-Null
Require-Command "llvm-ar" | Out-Null
Require-Command "llvm-lib" | Out-Null
Require-Command "ninja" | Out-Null
Require-Command "python" | Out-Null
Require-Command "cmake" | Out-Null
Require-Command "clang" | Out-Null
Require-Command "lld-link" | Out-Null

& $cargo xwin env --target $targetTriple | Out-Null

if (-not (Test-Path (Join-Path $xwinDir "crt\include")) -or -not (Test-Path (Join-Path $xwinDir "sdk\lib\um\x86_64"))) {
    throw "missing cargo-xwin sysroot at $xwinDir; run cargo xwin env --target $targetTriple once first"
}

Ensure-HostGitver -Dub $dub -Ldc2 $ldc2
Build-RuntimeLibs

$previousPath = $env:PATH
$previousDFlags = $env:DFLAGS
$env:PATH = "$wrapperDir;$homeDir\.cache\cargo-xwin;$previousPath"
$env:DFLAGS = "--mtriple=$targetTriple --linker=lld-link --mscrtlib=msvcrt -link-defaultlib-shared=false"

Push-Location $sourceDir
try {
    & (Join-Path $wrapperDir "dub.cmd") build "--compiler=$ldc2" --config=yesgl --arch=x86_64 --force | Out-Null
    Build-WindowsInochi2d -Ldc2 $ldc2 -Dub (Join-Path $wrapperDir "dub.cmd")
}
finally {
    Pop-Location
    $env:PATH = $previousPath
    $env:DFLAGS = $previousDFlags
}

Get-Item (Join-Path $outDir "inochi2d-c.dll"), (Join-Path $outDir "inochi2d-c.lib") | ForEach-Object {
    Write-Host $_.FullName
}
