$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$TargetPath
)

function Get-RequiredEnvironmentVariable {
    param([string]$Name)

    $value = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "Required environment variable is missing: $Name"
    }

    return $value
}

function Get-SignToolPath {
    $command = Get-Command "signtool.exe" -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    $roots = @()
    if (${env:ProgramFiles(x86)}) {
        $roots += (Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin")
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

$certThumbprint = (Get-RequiredEnvironmentVariable -Name "CERTUM_CERT_SHA1" -replace "\s", "").ToUpperInvariant()
$signtoolPath = Get-SignToolPath

if (-not (Test-Path -LiteralPath $TargetPath -PathType Leaf)) {
    throw "Signing target not found: $TargetPath"
}

Write-Host "Signing file with Certum certificate thumbprint $certThumbprint"
Write-Host "signtool: $signtoolPath"
Write-Host "Target: $TargetPath"

& $signtoolPath sign `
    /sha1 $certThumbprint `
    /tr http://time.certum.pl `
    /td sha256 `
    /fd sha256 `
    /v `
    $TargetPath

if ($LASTEXITCODE -ne 0) {
    throw "signtool sign failed with exit code $LASTEXITCODE"
}
