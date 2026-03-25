function Resolve-ApplicationCommandPath {
    param([Parameter(Mandatory = $true)][string]$Name)

    $command = Get-Command $Name -ErrorAction SilentlyContinue |
        Where-Object {
            $_.CommandType -eq "Application" -and
            @(".exe", ".com") -contains ([System.IO.Path]::GetExtension($_.Source).ToLowerInvariant())
        } |
        Select-Object -First 1

    if (-not $command) {
        throw "missing required tool: $Name"
    }

    $command.Source
}

function Get-CargoXwinCacheDir {
    if ($env:LOCALAPPDATA) {
        return Join-Path $env:LOCALAPPDATA "cargo-xwin"
    }

    $homeDir = if ($env:HOME) { $env:HOME } else { $env:USERPROFILE }
    Join-Path $homeDir ".cache\cargo-xwin"
}

function Copy-ToolIfNeeded {
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$Destination
    )

    $shouldCopy = -not (Test-Path -LiteralPath $Destination -PathType Leaf)

    if (-not $shouldCopy) {
        $sourceInfo = Get-Item -LiteralPath $Source
        $destinationInfo = Get-Item -LiteralPath $Destination
        $shouldCopy = $sourceInfo.Length -ne $destinationInfo.Length -or
            $sourceInfo.LastWriteTimeUtc -gt $destinationInfo.LastWriteTimeUtc
    }

    if ($shouldCopy) {
        Copy-Item -LiteralPath $Source -Destination $Destination -Force
    }
}

function Ensure-CargoXwinToolchainBin {
    $toolchainDir = Get-CargoXwinCacheDir
    New-Item -ItemType Directory -Force -Path $toolchainDir | Out-Null

    foreach ($tool in @(
        @{ Source = "clang-cl"; Destination = "clang-cl.exe" },
        @{ Source = "lld-link"; Destination = "lld-link.exe" },
        @{ Source = "llvm-lib"; Destination = "llvm-lib.exe" },
        @{ Source = "llvm-ar"; Destination = "llvm-ar.exe" },
        @{ Source = "llvm-ar"; Destination = "llvm-dlltool.exe" }
    )) {
        $sourcePath = Resolve-ApplicationCommandPath -Name $tool.Source
        $destinationPath = Join-Path $toolchainDir $tool.Destination
        Copy-ToolIfNeeded -Source $sourcePath -Destination $destinationPath
    }

    return $toolchainDir
}

function Get-CargoXwinSanitizedPath {
    param([string]$ExistingPath = $env:PATH)

    $toolchainDir = Ensure-CargoXwinToolchainBin
    $pathEntries = [System.Collections.Generic.List[string]]::new()
    $pathEntries.Add($toolchainDir)

    foreach ($entry in ($ExistingPath -split ";")) {
        if ([string]::IsNullOrWhiteSpace($entry)) {
            continue
        }

        $trimmedEntry = $entry.Trim()
        if ($trimmedEntry -eq $toolchainDir) {
            continue
        }

        if (Test-Path -LiteralPath (Join-Path $trimmedEntry "clang.exe") -PathType Leaf) {
            continue
        }

        $pathEntries.Add($trimmedEntry)
    }

    ($pathEntries | Select-Object -Unique) -join ";"
}

function Invoke-WithCargoXwinToolchain {
    param([Parameter(Mandatory = $true)][scriptblock]$ScriptBlock)

    $previousPath = $env:PATH
    $previousCrossCompiler = [Environment]::GetEnvironmentVariable("XWIN_CROSS_COMPILER")

    try {
        $env:PATH = Get-CargoXwinSanitizedPath -ExistingPath $env:PATH
        $env:XWIN_CROSS_COMPILER = "clang-cl"
        & $ScriptBlock
    }
    finally {
        $env:PATH = $previousPath
        [Environment]::SetEnvironmentVariable("XWIN_CROSS_COMPILER", $previousCrossCompiler)
    }
}
