$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

$root = Split-Path -Parent $PSScriptRoot
$driveName = $null
$rootPath = $root

if ($root.StartsWith("\\wsl.localhost\") -or $root.StartsWith("\\wsl$\")) {
    throw @"
Windows-host Tauri release builds are not supported from a WSL UNC path:
$root

Clone or copy the repository to a native Windows path such as C:\src\waifudex,
run pnpm install on Windows there, and then rerun pnpm tauri:release:windows.
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

    node .\scripts\sync-app-version.mjs
    node .\scripts\generate-dependency-inventory.mjs
    & .\scripts\build-inochi2d-windows.ps1
    node .\scripts\assert-windows-inochi2d-artifacts.mjs

    $vitePath = node -e "const path=require('node:path'); process.stdout.write(path.join(path.dirname(require.resolve('vite/package.json')), 'bin', 'vite.js'));"
    $tauriPath = node -e "const path=require('node:path'); process.stdout.write(path.join(path.dirname(require.resolve('@tauri-apps/cli/package.json')), 'tauri.js'));"

    node $vitePath build
    node $tauriPath build --config src-tauri/tauri.windows.updater.conf.json
}
finally {
    if ($driveName) {
        Remove-PSDrive -Name $driveName -Force -ErrorAction SilentlyContinue
    }
}
