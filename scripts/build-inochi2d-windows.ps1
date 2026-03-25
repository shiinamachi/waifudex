$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$root = Split-Path -Parent $PSScriptRoot
$sourceDir = Join-Path $root "third_party\inochi2d-c"
$outDir = Join-Path $sourceDir "out"
$targetTriple = "x86_64-pc-windows-msvc"
$homeDir = if ($env:HOME) { $env:HOME } else { $env:USERPROFILE }
$xwinDir = if ($env:XWIN_DIR) {
    $env:XWIN_DIR
} elseif ($env:XWIN_CACHE_DIR) {
    $env:XWIN_CACHE_DIR
} elseif ($env:LOCALAPPDATA) {
    Join-Path $env:LOCALAPPDATA "cargo-xwin\xwin"
} else {
    Join-Path $homeDir ".cache\cargo-xwin\xwin"
}
$waifudexCacheDir = if ($env:XDG_CACHE_HOME) {
    Join-Path $env:XDG_CACHE_HOME "waifudex\inochi2d-windows"
} else {
    Join-Path $homeDir ".cache\waifudex\inochi2d-windows"
}
$artifactCacheDir = Join-Path $waifudexCacheDir "artifacts"
$runtimeBuildDir = Join-Path $waifudexCacheDir "ldc-runtime"
$runtimeLibDir = Join-Path $waifudexCacheDir "runtime-lib"
$workDir = Join-Path $waifudexCacheDir "work"
$wrapperDir = Join-Path $waifudexCacheDir "dub-wrapper"
$buildStampPath = Join-Path $waifudexCacheDir "inochi2d-build.stamp"
$buildStampVersion = "windows-inochi2d-cache-v1"

$ensureScript = Join-Path $PSScriptRoot "ensure-windows-host-build-env.ps1"
if (-not $env:WAIFUDEX_WINDOWS_HOST_BUILD_ENV_READY -and (Test-Path $ensureScript)) {
    & $ensureScript
}

function Require-Command {
    param([string]$Name)

    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if (-not $command) {
        throw "missing required tool: $Name"
    }

    $command.Source
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

function Invoke-NativeCommandQuietly {
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
        if ($LASTEXITCODE -ne 0) {
            throw "$Command failed with exit code $LASTEXITCODE"
        }
    }
    finally {
        $PSNativeCommandUseErrorActionPreference = $previousNativeErrorPreference
        $ErrorActionPreference = $previousErrorAction
    }
}

function Resolve-PythonCommand {
    $candidates = [System.Collections.Generic.List[object]]::new()

    if (Get-Command "mise" -ErrorAction SilentlyContinue) {
        $candidates.Add([pscustomobject]@{
                Command   = "mise"
                Arguments = @("exec", "--", "python")
            })
    }

    if (Get-Command "python" -ErrorAction SilentlyContinue) {
        $candidates.Add([pscustomobject]@{
                Command   = "python"
                Arguments = @()
            })
    }

    if (Get-Command "py" -ErrorAction SilentlyContinue) {
        $candidates.Add([pscustomobject]@{
                Command   = "py"
                Arguments = @("-3")
            })
    }

    if (Get-Command "python3" -ErrorAction SilentlyContinue) {
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
        $pythonCommand = Resolve-PythonCommand
        if (-not $pythonCommand) {
            throw "missing required Python runtime; install the pinned mise python or make python/py/python3 executable in this shell"
        }

        $currentEnvironment = @{}
        foreach ($entry in $Environment.GetEnumerator()) {
            $currentEnvironment[$entry.Key] = [Environment]::GetEnvironmentVariable($entry.Key)
            [Environment]::SetEnvironmentVariable($entry.Key, $entry.Value)
        }

        & $pythonCommand.Command @($pythonCommand.Arguments + @($scriptPath) + $Arguments)
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

function Get-CombinedSha256 {
    param([string[]]$Values)

    $joined = [string]::Join("`n", $Values)
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($joined)
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        return [Convert]::ToHexString($sha256.ComputeHash($bytes))
    }
    finally {
        $sha256.Dispose()
    }
}

function Get-FileSha256 {
    param([Parameter(Mandatory = $true)][string]$LiteralPath)

    $stream = [System.IO.File]::OpenRead($LiteralPath)
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        return [Convert]::ToHexString($sha256.ComputeHash($stream))
    }
    finally {
        $sha256.Dispose()
        $stream.Dispose()
    }
}

function Get-Inochi2dBuildStamp {
    $entries = [System.Collections.Generic.List[string]]::new()
    $entries.Add("stamp-version=$buildStampVersion")
    $entries.Add("target=$targetTriple")
    $entries.Add("script=$(Get-FileSha256 -LiteralPath $PSCommandPath)")

    Get-ChildItem -LiteralPath $sourceDir -File -Recurse |
        Where-Object { $_.FullName -notlike "$outDir*" } |
        Where-Object { $_.FullName -notmatch '[\\/]\.git([\\/]|$)' } |
        Sort-Object FullName |
        ForEach-Object {
            $relativePath = [System.IO.Path]::GetRelativePath($sourceDir, $_.FullName)
            $entries.Add("$relativePath=$(Get-FileSha256 -LiteralPath $_.FullName)")
        }

    Get-CombinedSha256 -Values $entries
}

function Restore-CachedInochi2dArtifacts {
    param([string]$ExpectedStamp)

    $cachedStamp = if (Test-Path $buildStampPath) {
        (Get-Content -LiteralPath $buildStampPath -Raw).Trim()
    } else {
        ""
    }

    if ([string]::IsNullOrWhiteSpace($cachedStamp) -or $cachedStamp -ne $ExpectedStamp) {
        return $false
    }

    foreach ($file in @(
        (Join-Path $artifactCacheDir "inochi2d-c.dll"),
        (Join-Path $artifactCacheDir "inochi2d-c.lib"),
        (Join-Path $runtimeLibDir "druntime-ldc.lib"),
        (Join-Path $runtimeLibDir "phobos2-ldc.lib")
    )) {
        if (-not (Test-Path $file)) {
            return $false
        }
    }

    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
    Copy-Item -LiteralPath (Join-Path $artifactCacheDir "inochi2d-c.dll") -Destination (Join-Path $outDir "inochi2d-c.dll") -Force
    Copy-Item -LiteralPath (Join-Path $artifactCacheDir "inochi2d-c.lib") -Destination (Join-Path $outDir "inochi2d-c.lib") -Force
    return $true
}

function Save-CachedInochi2dArtifacts {
    param([string]$ExpectedStamp)

    New-Item -ItemType Directory -Force -Path $artifactCacheDir | Out-Null
    Copy-Item -LiteralPath (Join-Path $outDir "inochi2d-c.dll") -Destination (Join-Path $artifactCacheDir "inochi2d-c.dll") -Force
    Copy-Item -LiteralPath (Join-Path $outDir "inochi2d-c.lib") -Destination (Join-Path $artifactCacheDir "inochi2d-c.lib") -Force
    Set-Content -LiteralPath $buildStampPath -Value "$ExpectedStamp`n" -Encoding ascii
}

function Ensure-HostGitver {
    param(
        [string]$Dub,
        [string]$Ldc2
    )

    $dubPackagesDir = if ($env:DUB_HOME) {
        Join-Path $env:DUB_HOME "packages"
    } elseif ($env:DPATH) {
        Join-Path $env:DPATH "dub\packages"
    } elseif ($env:LOCALAPPDATA) {
        Join-Path $env:LOCALAPPDATA "dub\packages"
    } else {
        Join-Path $homeDir ".dub\packages"
    }
    $gitverPackage = "gitver@1.7.2"
    if (-not (Test-Path $dubPackagesDir)) {
        New-Item -ItemType Directory -Force -Path $dubPackagesDir | Out-Null
    }

    function Find-HostGitverDir {
        $candidates = @()
        $candidates += Join-Path $dubPackagesDir "gitver\1.7.2\gitver"
        $candidates += Join-Path $dubPackagesDir "local\gitver\1.7.2\gitver"

        if (Test-Path $dubPackagesDir) {
            $candidates += Get-ChildItem -Path $dubPackagesDir -File -Recurse -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -in @("dub.sdl", "dub.json", "package.json") } |
                Where-Object { $_.Directory.Name -eq "gitver" } |
                Select-Object -ExpandProperty DirectoryName
        }

        if (Test-Path $homeDir) {
            $candidates += Get-ChildItem -Path $homeDir -File -Recurse -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -in @("dub.sdl", "dub.json", "package.json") } |
                Where-Object { $_.Directory.Name -eq "gitver" } |
                Select-Object -ExpandProperty DirectoryName
        }

        return $candidates |
            Where-Object { $_ -and (Test-Path $_) } |
            Select-Object -Unique |
            Select-Object -First 1
    }

    function Find-HostGitverBin {
        param(
            [string]$SearchRoot = $null
        )

        $candidates = @()

        if ($SearchRoot -and (Test-Path $SearchRoot)) {
            $candidates += @(
                (Join-Path $SearchRoot "out\gitver.exe"),
                (Join-Path $SearchRoot "out\gitver"),
                (Join-Path $SearchRoot "gitver.exe"),
                (Join-Path $SearchRoot "gitver")
            )

            $candidates += Get-ChildItem -Path $SearchRoot -File -Recurse -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -in @("gitver.exe", "gitver") } |
                Select-Object -ExpandProperty FullName
        }

        if (Test-Path $dubPackagesDir) {
            $candidates += Get-ChildItem -Path $dubPackagesDir -File -Recurse -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -in @("gitver.exe", "gitver") } |
                Select-Object -ExpandProperty FullName
        }

        if (Test-Path $homeDir) {
            $candidates += Get-ChildItem -Path $homeDir -File -Recurse -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -in @("gitver.exe", "gitver") } |
                Select-Object -ExpandProperty FullName
        }

        return $candidates |
            Where-Object { $_ -and (Test-Path $_) } |
            Select-Object -Unique |
            Select-Object -First 1
    }

    $hostGitverDir = Find-HostGitverDir
    $hostGitverBin = Find-HostGitverBin -SearchRoot $hostGitverDir

    if (-not $hostGitverDir -and -not $hostGitverBin) {
        Invoke-NativeCommandQuietly -Command $Dub -Arguments @("fetch", $gitverPackage)

        $hostGitverDir = Find-HostGitverDir
        $hostGitverBin = Find-HostGitverBin -SearchRoot $hostGitverDir
    }

    if (-not $hostGitverDir -and $hostGitverBin) {
        $hostGitverDir = Split-Path -Parent (Split-Path -Parent $hostGitverBin)
    }

    New-Item -ItemType Directory -Force -Path $wrapperDir | Out-Null
    if ($hostGitverDir) {
        Push-Location $hostGitverDir
        try {
            Invoke-NativeCommandQuietly -Command $Dub -Arguments @("build", "--compiler=$Ldc2", "--force")
        }
        finally {
            Pop-Location
        }

        $hostGitverBin = Find-HostGitverBin -SearchRoot $hostGitverDir
    }

    if (-not $hostGitverBin) {
        $probePath = Join-Path $wrapperDir "gitver-probe.cmd"
        @"
@echo off
setlocal
"%Dub%" run $gitverPackage -- --help >nul 2>&1
exit /b %errorlevel%
"@ | Set-Content -LiteralPath $probePath -Encoding ascii

        & $probePath
        if ($LASTEXITCODE -eq 0) {
            $wrapperPath = Join-Path $wrapperDir "dub.cmd"
            @"
@echo off
setlocal
set REAL_DUB=$Dub
if "%1"=="run" if "%2"=="gitver" (
  shift
  shift
  if "%1"=="--" shift
  "%REAL_DUB%" run $gitverPackage -- %*
  exit /b %errorlevel%
)
"%REAL_DUB%" %*
exit /b %errorlevel%
"@ | Set-Content -LiteralPath $wrapperPath -Encoding ascii
            return
        }

        throw "failed to locate or build gitver under $dubPackagesDir"
    }

    if (-not (Test-Path $hostGitverBin)) {
        throw "failed to build host gitver executable at $hostGitverBin"
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
    $xwinDirForFlags = $xwinDir -replace "\\", "/"
    $runtimeDFlags = "--mtriple=$targetTriple;--linker=lld-link;--mscrtlib=msvcrt;-link-defaultlib-shared=false"
    $runtimeCFlags = "--target=$targetTriple;-Wno-unused-command-line-argument;-fuse-ld=lld-link;-imsvc;$xwinDirForFlags/crt/include;-imsvc;$xwinDirForFlags/sdk/include/ucrt;-imsvc;$xwinDirForFlags/sdk/include/um;-imsvc;$xwinDirForFlags/sdk/include/shared;-imsvc;$xwinDirForFlags/sdk/include/winrt"
    $runtimeLinkerFlags = "-fuse-ld=lld-link;/LIBPATH:$xwinDirForFlags/crt/lib/x86_64;/LIBPATH:$xwinDirForFlags/sdk/lib/um/x86_64;/LIBPATH:$xwinDirForFlags/sdk/lib/ucrt/x86_64"
    $runtimeConfigureLog = Join-Path $waifudexCacheDir "ldc-runtime-configure.log"
    $runtimeNinjaLog = Join-Path $waifudexCacheDir "ldc-runtime-ninja.log"

    Remove-Item $runtimeBuildDir, $runtimeLibDir -Recurse -Force -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $runtimeBuildDir, $runtimeLibDir | Out-Null

    $previousCc = $env:CC
    $previousCxx = $env:CXX
    $previousNativeErrorPreference = $PSNativeCommandUseErrorActionPreference
    $previousErrorAction = $ErrorActionPreference
    $env:CC = "clang-cl"
    $env:CXX = "clang-cl"
    $PSNativeCommandUseErrorActionPreference = $false
    # Stderr from ldc-build-runtime/ninja (e.g. CMake warnings) creates
    # ErrorRecords that become terminating errors under "Stop" preference
    # even when *> redirects all streams. Use "Continue" so stderr flows
    # into the log file and exit codes are checked explicitly.
    $ErrorActionPreference = "Continue"

    try {
        & ldc-build-runtime --ninja --buildDir $runtimeBuildDir `
            CMAKE_POLICY_VERSION_MINIMUM=3.5 `
            BUILD_SHARED_LIBS=OFF `
            HAVE_UNISTD_H=0 `
            CMAKE_C_COMPILER=clang-cl `
            CMAKE_CXX_COMPILER=clang-cl `
            --targetSystem "Windows;MSVC" `
            "--dFlags=$runtimeDFlags" `
            "--cFlags=$runtimeCFlags" `
            "--linkerFlags=$runtimeLinkerFlags" `
            -j2 *> $runtimeConfigureLog
        if ($LASTEXITCODE -ne 0) {
            Write-Host "ldc-build-runtime configure failed; showing tail of $runtimeConfigureLog"
            Get-Content -LiteralPath $runtimeConfigureLog -Tail 200
            throw "failed to configure ldc runtime libraries"
        }

        Push-Location $runtimeBuildDir
        try {
            & ninja -k 0 -v *> $runtimeNinjaLog
            if ($LASTEXITCODE -ne 0) {
                Write-Host "ninja failed; showing tail of $runtimeNinjaLog"
                Get-Content -LiteralPath $runtimeNinjaLog -Tail 200
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
        $PSNativeCommandUseErrorActionPreference = $previousNativeErrorPreference
        $ErrorActionPreference = $previousErrorAction
    }

Invoke-PythonScript -Arguments @($runtimeBuildDir, $runtimeLibDir) -Script @'
import os
import shutil
import subprocess
import sys
import tempfile

build_dir = sys.argv[1]
out_dir = sys.argv[2]
targets = [
    ("druntime-ldc", "druntime-ldc.lib"),
    ("phobos2-ldc", "phobos2-ldc.lib"),
]

def find_runtime_library(stem: str) -> str:
    preferred = [
        f"{stem}.lib",
        f"lib{stem}.a",
        f"{stem}.a",
    ]
    fallback = []

    for root, _, files in os.walk(build_dir):
        for name in files:
            lower_name = name.lower()
            full_path = os.path.join(root, name)
            if lower_name in preferred:
                return full_path
            if lower_name.startswith(stem) and os.path.splitext(lower_name)[1] in {".lib", ".a"}:
                fallback.append(full_path)

    if fallback:
        fallback.sort(key=lambda path: (0 if path.lower().endswith(".lib") else 1, len(path)))
        return fallback[0]

    raise SystemExit(f"missing runtime library matching {stem!r} under {build_dir}")


for stem, out_name in targets:
    source = find_runtime_library(stem)
    destination = os.path.join(out_dir, out_name)

    if source.lower().endswith(".lib"):
        shutil.copyfile(source, destination)
        continue

    with tempfile.TemporaryDirectory(dir=build_dir) as tmp:
        subprocess.run(["llvm-ar", "x", source], cwd=tmp, check=True)
        members = []
        for root, _, files in os.walk(tmp):
            for name in files:
                if name.endswith(".o") or name.endswith(".obj"):
                    members.append(os.path.join(root, name))
        subprocess.run(["llvm-lib", f"/OUT:{destination}", *sorted(members)], check=True)
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

    $xwinDirForFlags = $xwinDir -replace "\\", "/"
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
        "-L/LIBPATH:$xwinDirForFlags/crt/lib/x86_64",
        "-L/LIBPATH:$xwinDirForFlags/sdk/lib/um/x86_64",
        "-L/LIBPATH:$xwinDirForFlags/sdk/lib/ucrt/x86_64"
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

$expectedBuildStamp = Get-Inochi2dBuildStamp
if (Restore-CachedInochi2dArtifacts -ExpectedStamp $expectedBuildStamp) {
    Write-Host "Reusing cached Windows inochi2d artifacts for stamp $expectedBuildStamp"
    Get-Item (Join-Path $outDir "inochi2d-c.dll"), (Join-Path $outDir "inochi2d-c.lib") | ForEach-Object {
        Write-Host $_.FullName
    }
    return
}

$cargo = Require-Command "cargo"
Require-Command "cargo-xwin" | Out-Null
$dub = Require-Command "dub"
$ldc2 = Require-Command "ldc2"
Require-Command "ldc-build-runtime" | Out-Null
Require-Command "llvm-ar" | Out-Null
Require-Command "llvm-lib" | Out-Null
Require-Command "ninja" | Out-Null
Require-Command "cmake" | Out-Null
Require-Command "clang" | Out-Null
Require-Command "clang-cl" | Out-Null
Require-Command "lld-link" | Out-Null

if (-not (Resolve-PythonCommand)) {
    throw "missing required Python runtime; run mise install or make python/py/python3 executable in this shell"
}

if (-not (Test-Path (Join-Path $xwinDir "crt\include")) -or -not (Test-Path (Join-Path $xwinDir "sdk\lib\um\x86_64"))) {
    & $cargo xwin cache xwin --cross-compiler clang
    if ($LASTEXITCODE -ne 0) {
        throw "cargo xwin cache xwin --cross-compiler clang failed with exit code $LASTEXITCODE"
    }
}

if (-not (Test-Path (Join-Path $xwinDir "crt\include")) -or -not (Test-Path (Join-Path $xwinDir "sdk\lib\um\x86_64"))) {
    throw "missing cargo-xwin sysroot at $xwinDir"
}

Ensure-HostGitver -Dub $dub -Ldc2 $ldc2
Build-RuntimeLibs

$previousPath = $env:PATH
$previousDFlags = $env:DFLAGS
$dubWarmupLog = Join-Path $waifudexCacheDir "inochi2d-dub-build.log"
$env:PATH = "$wrapperDir;$homeDir\.cache\cargo-xwin;$previousPath"
$env:DFLAGS = "--mtriple=$targetTriple --linker=lld-link --mscrtlib=msvcrt -link-defaultlib-shared=false"

Push-Location $sourceDir
$previousCrossCompileErrorAction = $ErrorActionPreference
# Native cross-compilation tools (dub, ldc2) write warnings/progress to
# stderr which PowerShell wraps as ErrorRecords. Under "Stop" preference
# these become terminating errors. Use "Continue" and check exit codes.
$ErrorActionPreference = "Continue"
try {
    & (Join-Path $wrapperDir "dub.cmd") build "--compiler=$ldc2" --config=yesgl --arch=x86_64 --force *> $dubWarmupLog
    if ($LASTEXITCODE -ne 0) {
        Write-Warning "dub build warm-up failed with exit code $LASTEXITCODE; continuing with manual Windows link step. Log: $dubWarmupLog"
        if (Test-Path $dubWarmupLog) {
            Get-Content -LiteralPath $dubWarmupLog -Tail 200
        }
    }
    Build-WindowsInochi2d -Ldc2 $ldc2 -Dub (Join-Path $wrapperDir "dub.cmd")
}
finally {
    Pop-Location
    $env:PATH = $previousPath
    $env:DFLAGS = $previousDFlags
    $ErrorActionPreference = $previousCrossCompileErrorAction
}

Save-CachedInochi2dArtifacts -ExpectedStamp $expectedBuildStamp

Get-Item (Join-Path $outDir "inochi2d-c.dll"), (Join-Path $outDir "inochi2d-c.lib") | ForEach-Object {
    Write-Host $_.FullName
}
