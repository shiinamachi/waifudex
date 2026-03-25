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

    $signingKeyPath = Join-Path $rootPath "production.key"
    if (-not (Test-Path -LiteralPath $signingKeyPath -PathType Leaf)) {
        throw "Missing Tauri signing key: $signingKeyPath"
    }

    $env:TAURI_SIGNING_PRIVATE_KEY = [System.IO.File]::ReadAllText($signingKeyPath)
    $env:TAURI_SIGNING_PRIVATE_KEY_PATH = $signingKeyPath

    & .\scripts\ensure-windows-host-build-env.ps1
    node .\scripts\sync-app-version.mjs
    node .\scripts\generate-dependency-inventory.mjs
    & .\scripts\build-inochi2d-windows.ps1
    node .\scripts\assert-windows-inochi2d-artifacts.mjs

    $vitePath = node -e "const path=require('node:path'); process.stdout.write(path.join(path.dirname(require.resolve('vite/package.json')), 'bin', 'vite.js'));"
    $tauriPath = node -e "const path=require('node:path'); process.stdout.write(path.join(path.dirname(require.resolve('@tauri-apps/cli/package.json')), 'tauri.js'));"

    node $vitePath build
    $env:XWIN_ARCH = "x86_64"
    node $tauriPath build --config src-tauri/tauri.windows.build.conf.json --runner cargo-xwin -- --target x86_64-pc-windows-msvc
}
finally {
    if ($driveName) {
        Remove-PSDrive -Name $driveName -Force -ErrorAction SilentlyContinue
    }
}
