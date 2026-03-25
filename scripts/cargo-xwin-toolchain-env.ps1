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

function Get-WaifudexToolCacheRoot {
    if ($env:LOCALAPPDATA) {
        return Join-Path $env:LOCALAPPDATA "waifudex-tools"
    }

    $homeDir = if ($env:HOME) { $env:HOME } else { $env:USERPROFILE }
    Join-Path $homeDir ".cache\waifudex-tools"
}

function Write-CommandWrapperIfNeeded {
    param(
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$Destination
    )

    $wrapperContent = @(
        "@echo off",
        "`"$Source`" %*",
        "exit /b %ERRORLEVEL%"
    ) -join "`r`n"

    $shouldWrite = -not (Test-Path -LiteralPath $Destination -PathType Leaf)

    if (-not $shouldWrite) {
        $existingContent = Get-Content -LiteralPath $Destination -Raw
        $shouldWrite = $existingContent -ne $wrapperContent
    }

    if ($shouldWrite) {
        Set-Content -LiteralPath $Destination -Value $wrapperContent -Encoding ascii
    }
}

function Ensure-CargoXwinToolchainBin {
    $toolchainDir = Join-Path (Get-WaifudexToolCacheRoot) "cargo-xwin-toolchain"
    New-Item -ItemType Directory -Force -Path $toolchainDir | Out-Null

    foreach ($commandName in @("clang-cl", "lld-link", "llvm-lib", "llvm-ar")) {
        $sourcePath = Resolve-ApplicationCommandPath -Name $commandName
        $destinationPath = Join-Path $toolchainDir "$commandName.cmd"
        Write-CommandWrapperIfNeeded -Source $sourcePath -Destination $destinationPath
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
