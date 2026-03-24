$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$root = Split-Path -Parent $PSScriptRoot
$driveName = $null
$rootPath = $root

if ($root.StartsWith("\\wsl.localhost\") -or $root.StartsWith("\\wsl$\")) {
    throw @"
Windows-host Inochi2D builds are not supported from a WSL UNC path:
$root

Clone or copy the repository to a native Windows path such as C:\src\waifudex,
run pnpm install on Windows there, and then rerun pnpm inochi2d:build:windows.
"@
}

if ($root.StartsWith("\\")) {
    $driveName = "WDX"
    if (Get-PSDrive -Name $driveName -ErrorAction SilentlyContinue) {
        Remove-PSDrive -Name $driveName -Force
    }
    New-PSDrive -Name $driveName -PSProvider FileSystem -Root $root | Out-Null
    $rootPath = "${driveName}:\"
}

try {
    Set-Location -LiteralPath $rootPath

    $bashCommand = Get-Command bash -ErrorAction SilentlyContinue
    $bash = $null
    if ($bashCommand) {
        $bash = $bashCommand.Source
    }
    if (-not $bash) {
        throw "bash was not found on PATH. Install Git Bash or another bash-compatible environment on Windows to run scripts/build-inochi2d-windows.sh."
    }

    & $bash ./scripts/build-inochi2d-windows.sh
}
finally {
    if ($driveName) {
        Remove-PSDrive -Name $driveName -Force -ErrorAction SilentlyContinue
    }
}
