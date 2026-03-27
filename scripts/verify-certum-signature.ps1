$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$InstallerPath
)

function Get-SignToolPath {
    $command = Get-Command "signtool.exe" -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    $roots = @()
    if ($env:'ProgramFiles(x86)') {
        $roots += (Join-Path $env:'ProgramFiles(x86)' "Windows Kits\10\bin")
    }
    if ($env:ProgramFiles) {
        $roots += (Join-Path $env:ProgramFiles "Windows Kits\10\bin")
    }

    foreach ($root in $roots) {
        if (-not (Test-Path -LiteralPath $root -PathType Container)) {
            continue
        }

        $match = Get-ChildItem -Path $root -Filter "signtool.exe" -File -Recurse -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending |
            Select-Object -First 1

        if ($match) {
            return $match.FullName
        }
    }

    throw "signtool.exe was not found on PATH or under Windows Kits."
}

$signtoolPath = Get-SignToolPath

if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) {
    throw "Installer not found for signature verification: $InstallerPath"
}

Write-Host "Verifying Authenticode signature..."
Write-Host "signtool: $signtoolPath"
Write-Host "Installer: $InstallerPath"

& $signtoolPath verify /pa /all $InstallerPath

if ($LASTEXITCODE -ne 0) {
    throw "signtool verify failed with exit code $LASTEXITCODE"
}

Write-Host "Authenticode verification succeeded."
