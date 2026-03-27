$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

. (Join-Path $PSScriptRoot "cargo-xwin-toolchain-env.ps1")

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
    $generatedTauriConfigPath = Join-Path $rootPath "src-tauri\tauri.windows.build.generated.json"
    $certumSignScriptPath = Join-Path $rootPath "scripts\invoke-certum-signtool.ps1"

    $signingKeyPath = Join-Path $rootPath "production.key"
    if (-not (Test-Path -LiteralPath $signingKeyPath -PathType Leaf)) {
        throw "Missing Tauri signing key: $signingKeyPath"
    }
    if (-not (Test-Path -LiteralPath $certumSignScriptPath -PathType Leaf)) {
        throw "Missing Certum signtool wrapper: $certumSignScriptPath"
    }
    if (-not (Test-Path Env:CERTUM_CERT_SHA1) -or [string]::IsNullOrWhiteSpace($env:CERTUM_CERT_SHA1)) {
        throw "CERTUM_CERT_SHA1 is required for Windows Authenticode signing."
    }

    $env:TAURI_SIGNING_PRIVATE_KEY = [System.IO.File]::ReadAllText($signingKeyPath)
    $env:TAURI_SIGNING_PRIVATE_KEY_PATH = $signingKeyPath
    if (-not (Test-Path Env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD)) {
        $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ""
    }

    & .\scripts\ensure-windows-host-build-env.ps1
    $env:WAIFUDEX_WINDOWS_HOST_BUILD_ENV_READY = "1"
    node .\scripts\sync-app-version.mjs
    node .\scripts\generate-dependency-inventory.mjs
    & .\scripts\build-inochi2d-windows.ps1
    node .\scripts\assert-windows-inochi2d-artifacts.mjs
    node .\scripts\normalize-tauri-windows-build-config.mjs --config .\src-tauri\tauri.windows.build.conf.json --output $generatedTauriConfigPath

    $generatedConfig = Get-Content -LiteralPath $generatedTauriConfigPath -Raw | ConvertFrom-Json
    if (-not $generatedConfig.bundle) {
        $generatedConfig | Add-Member -MemberType NoteProperty -Name bundle -Value ([pscustomobject]@{})
    }
    if (-not $generatedConfig.bundle.windows) {
        $generatedConfig.bundle | Add-Member -MemberType NoteProperty -Name windows -Value ([pscustomobject]@{})
    }

    $signCommand = "powershell -NoProfile -ExecutionPolicy Bypass -File `"$certumSignScriptPath`" `"%1`""
    if (Get-Member -InputObject $generatedConfig.bundle.windows -Name signCommand -MemberType NoteProperty -ErrorAction SilentlyContinue) {
        $generatedConfig.bundle.windows.signCommand = $signCommand
    }
    else {
        $generatedConfig.bundle.windows | Add-Member -MemberType NoteProperty -Name signCommand -Value $signCommand
    }

    $generatedConfig | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $generatedTauriConfigPath -Encoding UTF8
    Write-Host "Injected Windows signCommand into $generatedTauriConfigPath"

    $vitePath = node -e "const path=require('node:path'); process.stdout.write(path.join(path.dirname(require.resolve('vite/package.json')), 'bin', 'vite.js'));"
    $tauriPath = node -e "const path=require('node:path'); process.stdout.write(path.join(path.dirname(require.resolve('@tauri-apps/cli/package.json')), 'tauri.js'));"

    node $vitePath build
    $env:XWIN_ARCH = "x86_64"
    Invoke-WithCargoXwinToolchain {
        node $tauriPath build --ci --config $generatedTauriConfigPath --target x86_64-pc-windows-msvc --runner cargo-xwin
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
