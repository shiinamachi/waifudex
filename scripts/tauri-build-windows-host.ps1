$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

. (Join-Path $PSScriptRoot "cargo-xwin-toolchain-env.ps1")

$root = Split-Path -Parent $PSScriptRoot
$driveName = $null
$rootPath = $root

if ($root.StartsWith("\\wsl.localhost\") -or $root.StartsWith("\\wsl$\")) {
    throw @"
Windows-host Tauri builds are not supported from a WSL UNC path:
$root

Clone or copy the repository to a native Windows path such as C:\src\waifudex,
run pnpm install on Windows there, and then rerun pnpm tauri:build:windows.
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
    $generatedTauriConfigPath = Join-Path $rootPath "src-tauri\tauri.windows.build.generated.json"

    & .\scripts\ensure-windows-host-build-env.ps1
    node .\scripts\sync-app-version.mjs
    node .\scripts\generate-dependency-inventory.mjs
    & .\scripts\build-inochi2d-windows.ps1
    node .\scripts\assert-windows-inochi2d-artifacts.mjs
    node .\scripts\normalize-tauri-windows-build-config.mjs --config .\src-tauri\tauri.windows.build.conf.json --output $generatedTauriConfigPath

    $vitePath = node -e "const path=require('node:path'); process.stdout.write(path.join(path.dirname(require.resolve('vite/package.json')), 'bin', 'vite.js'));"
    $tauriPath = node -e "const path=require('node:path'); process.stdout.write(path.join(path.dirname(require.resolve('@tauri-apps/cli/package.json')), 'tauri.js'));"

    node $vitePath build
    $env:XWIN_ARCH = "x86_64"
    Invoke-WithCargoXwinToolchain {
        node $tauriPath build --no-bundle --config $generatedTauriConfigPath --runner cargo-xwin -- --target x86_64-pc-windows-msvc
    }
}
finally {
    if ($generatedTauriConfigPath) {
        Remove-Item -LiteralPath $generatedTauriConfigPath -Force -ErrorAction SilentlyContinue
    }
    if ($driveName) {
        Remove-PSDrive -Name $driveName -Force -ErrorAction SilentlyContinue
    }
}
