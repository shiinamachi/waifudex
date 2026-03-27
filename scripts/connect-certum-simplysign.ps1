$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true

function Get-RequiredEnvironmentVariable {
    param([string]$Name)

    $value = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "Required environment variable is missing: $Name"
    }

    return $value
}

function Get-OptionalIntegerEnvironmentVariable {
    param(
        [string]$Name,
        [int]$DefaultValue
    )

    $value = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        return $DefaultValue
    }

    $parsedValue = 0
    if (-not [int]::TryParse($value, [ref]$parsedValue)) {
        throw "Environment variable $Name must be an integer when provided."
    }

    return $parsedValue
}

function Get-OptionalBooleanEnvironmentVariable {
    param(
        [string]$Name,
        [bool]$DefaultValue
    )

    $value = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        return $DefaultValue
    }

    switch ($value.Trim().ToLowerInvariant()) {
        "1" { return $true }
        "true" { return $true }
        "yes" { return $true }
        "0" { return $false }
        "false" { return $false }
        "no" { return $false }
        default { throw "Environment variable $Name must be a boolean-like value when provided." }
    }
}

function Get-OptionalStringEnvironmentVariable {
    param(
        [string]$Name,
        [string]$DefaultValue
    )

    $value = [Environment]::GetEnvironmentVariable($Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        return $DefaultValue
    }

    return $value
}

function Get-QueryValue {
    param(
        [string]$Query,
        [string]$Name
    )

    foreach ($pair in $Query.TrimStart("?").Split("&", [System.StringSplitOptions]::RemoveEmptyEntries)) {
        $parts = $pair.Split("=", 2)
        $key = [System.Uri]::UnescapeDataString($parts[0])
        if ($key -ne $Name) {
            continue
        }

        if ($parts.Length -eq 1) {
            return ""
        }

        return [System.Uri]::UnescapeDataString($parts[1])
    }

    return $null
}

function ConvertFrom-Base32 {
    param([string]$InputString)

    $normalized = ($InputString.ToUpperInvariant() -replace "\s", "").TrimEnd("=")
    if ([string]::IsNullOrWhiteSpace($normalized)) {
        throw "TOTP secret is empty."
    }

    $alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567"
    $bytes = [System.Collections.Generic.List[byte]]::new()
    $buffer = 0
    $bitsLeft = 0

    foreach ($character in $normalized.ToCharArray()) {
        $value = $alphabet.IndexOf($character)
        if ($value -lt 0) {
            throw "Unsupported Base32 character in TOTP secret: $character"
        }

        $buffer = ($buffer -shl 5) -bor $value
        $bitsLeft += 5

        while ($bitsLeft -ge 8) {
            $bitsLeft -= 8
            $bytes.Add([byte](($buffer -shr $bitsLeft) -band 0xFF))
        }
    }

    return $bytes.ToArray()
}

function Get-HmacHash {
    param(
        [byte[]]$Key,
        [byte[]]$CounterBytes,
        [string]$Algorithm
    )

    switch ($Algorithm.ToUpperInvariant()) {
        "SHA1" {
            $hmac = [System.Security.Cryptography.HMACSHA1]::new($Key)
        }
        "SHA256" {
            $hmac = [System.Security.Cryptography.HMACSHA256]::new($Key)
        }
        default {
            throw "Unsupported TOTP algorithm: $Algorithm"
        }
    }

    try {
        return $hmac.ComputeHash($CounterBytes)
    }
    finally {
        $hmac.Dispose()
    }
}

function Get-TotpCode {
    param([uri]$OtpUri)

    if ($OtpUri.Scheme -ne "otpauth") {
        throw "CERTUM_OTP_URI must use the otpauth scheme."
    }

    $secret = Get-QueryValue -Query $OtpUri.Query -Name "secret"
    if ([string]::IsNullOrWhiteSpace($secret)) {
        throw "CERTUM_OTP_URI is missing the secret query parameter."
    }

    $digitsValue = Get-QueryValue -Query $OtpUri.Query -Name "digits"
    $digits = if ([string]::IsNullOrWhiteSpace($digitsValue)) { 6 } else { [int]$digitsValue }

    $periodValue = Get-QueryValue -Query $OtpUri.Query -Name "period"
    $period = if ([string]::IsNullOrWhiteSpace($periodValue)) { 30 } else { [int]$periodValue }

    $algorithmValue = Get-QueryValue -Query $OtpUri.Query -Name "algorithm"
    $algorithm = if ([string]::IsNullOrWhiteSpace($algorithmValue)) { "SHA1" } else { $algorithmValue.ToUpperInvariant() }

    Write-Host "Using TOTP algorithm: $algorithm"

    $secretBytes = ConvertFrom-Base32 -InputString $secret
    $epochSeconds = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
    $counter = [int64][Math]::Floor($epochSeconds / $period)
    $counterBytes = [BitConverter]::GetBytes([System.Net.IPAddress]::HostToNetworkOrder($counter))
    $hash = Get-HmacHash -Key $secretBytes -CounterBytes $counterBytes -Algorithm $algorithm
    $offset = $hash[$hash.Length - 1] -band 0x0F
    $binary =
    (($hash[$offset] -band 0x7F) -shl 24) -bor
    (($hash[$offset + 1] -band 0xFF) -shl 16) -bor
    (($hash[$offset + 2] -band 0xFF) -shl 8) -bor
    ($hash[$offset + 3] -band 0xFF)
    $modulus = [int64]1
    for ($index = 0; $index -lt $digits; $index += 1) {
        $modulus *= 10
    }
    $otpValue = [int64]$binary % $modulus

    return $otpValue.ToString(("D{0}" -f $digits))
}

function ConvertTo-SendKeysLiteral {
    param([string]$Value)

    $escaped = $Value
    foreach ($character in @("+", "^", "%", "~", "(", ")", "[", "]", "{", "}")) {
        $escaped = $escaped.Replace($character, "{$character}")
    }

    return $escaped
}

function Restart-SimplySignDesktop {
    $existing = Get-Process -Name "SimplySignDesktop" -ErrorAction SilentlyContinue
    if ($existing) {
        Write-Host "Stopping existing SimplySignDesktop process..."
        foreach ($process in @($existing)) {
            $process.Kill()
            $process.WaitForExit(15000)
        }
    }
}

function Get-SimplySignProcesses {
    Get-Process -Name "SimplySignDesktop" -ErrorAction SilentlyContinue |
        Sort-Object StartTime
}

function Write-SimplySignProcessSnapshot {
    $processes = @(Get-SimplySignProcesses)
    if ($processes.Count -eq 0) {
        Write-Host "No SimplySignDesktop processes are currently running."
        return
    }

    Write-Host "Detected SimplySignDesktop processes:"
    foreach ($process in $processes) {
        $windowTitle = ""
        $windowHandle = 0
        try {
            $windowTitle = $process.MainWindowTitle
            $windowHandle = $process.MainWindowHandle
        }
        catch {
            $windowTitle = ""
            $windowHandle = 0
        }

        Write-Host ("- Id={0} MainWindowHandle={1} Title='{2}'" -f $process.Id, $windowHandle, $windowTitle)
    }
}

function Try-ActivateWindowHandle {
    param([System.Diagnostics.Process]$Process)

    if (-not $Process) {
        return $false
    }

    try {
        $Process.Refresh()
        if (-not $Process.MainWindowHandle -or $Process.MainWindowHandle -eq 0) {
            return $false
        }

        if (-not ("Win32ForegroundWindow" -as [type])) {
            Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class Win32ForegroundWindow {
    [DllImport("user32.dll")]
    public static extern bool ShowWindowAsync(IntPtr hWnd, int nCmdShow);

    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
}
"@
        }

        [void][Win32ForegroundWindow]::ShowWindowAsync($Process.MainWindowHandle, 5)
        Start-Sleep -Milliseconds 200
        return [Win32ForegroundWindow]::SetForegroundWindow($Process.MainWindowHandle)
    }
    catch {
        return $false
    }
}

function Wait-ForWindowActivation {
    param(
        [object]$Shell,
        [System.Diagnostics.Process]$Process,
        [string]$Caption,
        [int]$TimeoutSeconds = 30
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    $diagnosticsWritten = $false

    while ((Get-Date) -lt $deadline) {
        $activatedById = $false
        $activatedByCaption = $false
        $activatedByHandle = $false
        $matchingProcess = $null

        $candidateProcesses = @()
        if ($Process) {
            $candidateProcesses += $Process
        }
        $candidateProcesses += @(Get-SimplySignProcesses)
        $candidateProcesses = $candidateProcesses |
            Group-Object Id |
            ForEach-Object { $_.Group[0] }

        try {
            $activatedById = $Shell.AppActivate($Process.Id)
        }
        catch {
            $activatedById = $false
        }

        if (-not $activatedById) {
            foreach ($candidateProcess in $candidateProcesses) {
                try {
                    if ($Shell.AppActivate($candidateProcess.Id)) {
                        $activatedById = $true
                        $matchingProcess = $candidateProcess
                        break
                    }
                }
                catch {
                }
            }
        }

        if (-not $activatedById) {
            foreach ($candidateProcess in $candidateProcesses) {
                try {
                    $candidateProcess.Refresh()
                    if ([string]::IsNullOrWhiteSpace($candidateProcess.MainWindowTitle)) {
                        continue
                    }

                    if ($Shell.AppActivate($candidateProcess.MainWindowTitle)) {
                        $activatedByCaption = $true
                        $matchingProcess = $candidateProcess
                        break
                    }
                }
                catch {
                }
            }
        }

        if (-not $activatedById -and -not $activatedByCaption -and -not [string]::IsNullOrWhiteSpace($Caption)) {
            try {
                $activatedByCaption = $Shell.AppActivate($Caption)
            }
            catch {
                $activatedByCaption = $false
            }
        }

        if (-not $activatedById -and -not $activatedByCaption) {
            foreach ($candidateProcess in $candidateProcesses) {
                if (Try-ActivateWindowHandle -Process $candidateProcess) {
                    $activatedByHandle = $true
                    $matchingProcess = $candidateProcess
                    break
                }
            }
        }

        if ($activatedById -or $activatedByCaption -or $activatedByHandle) {
            if ($activatedById -and $matchingProcess) {
                Write-Host ("Activated SimplySign Desktop window by process id: {0}" -f $matchingProcess.Id)
            }
            elseif ($activatedByCaption -and $matchingProcess) {
                Write-Host ("Activated SimplySign Desktop window by process title: {0}" -f $matchingProcess.MainWindowTitle)
            }
            elseif ($activatedByCaption) {
                Write-Host ("Activated SimplySign Desktop window by caption fallback: {0}" -f $Caption)
            }
            else {
                Write-Host ("Activated SimplySign Desktop window by handle for process id: {0}" -f $matchingProcess.Id)
            }
            Start-Sleep -Milliseconds 500
            return
        }

        if (-not $diagnosticsWritten) {
            Write-SimplySignProcessSnapshot
            $diagnosticsWritten = $true
        }

        Write-Host "Waiting for SimplySign Desktop window focus..."
        Start-Sleep -Seconds 1
    }

    throw "Timed out waiting for the SimplySign Desktop window to become active."
}

function Wait-ForCertificateReady {
    param([int]$TimeoutSeconds = 30)

    $thumbprint = [Environment]::GetEnvironmentVariable("CERTUM_CERT_SHA1")
    if ([string]::IsNullOrWhiteSpace($thumbprint)) {
        Write-Host "CERTUM_CERT_SHA1 not provided. Waiting briefly after login instead of checking the certificate store."
        Start-Sleep -Seconds ([Math]::Min($TimeoutSeconds, 10))
        return
    }

    $normalizedThumbprint = ($thumbprint -replace "\s", "").ToUpperInvariant()
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)

    while ((Get-Date) -lt $deadline) {
        $match = Get-ChildItem -Path Cert:\CurrentUser\My -ErrorAction SilentlyContinue |
            Where-Object { $_.Thumbprint.ToUpperInvariant() -eq $normalizedThumbprint } |
            Select-Object -First 1

        if ($match) {
            Write-Host "Detected requested certificate thumbprint in CurrentUser\\My."
            return
        }

        Start-Sleep -Seconds 1
    }

    Write-Warning "Certificate thumbprint $normalizedThumbprint was not detected in CurrentUser\\My within $TimeoutSeconds seconds. Continuing and letting signtool be the source of truth."
}

function Clear-ActiveField {
    param([object]$Shell)

    $Shell.SendKeys("^a")
    Start-Sleep -Milliseconds 150
    $Shell.SendKeys("{BACKSPACE}")
    Start-Sleep -Milliseconds 150
}

$otpUriRaw = Get-RequiredEnvironmentVariable -Name "CERTUM_OTP_URI"
$userId = Get-RequiredEnvironmentVariable -Name "CERTUM_USERID"
$exePath = Get-RequiredEnvironmentVariable -Name "CERTUM_EXE_PATH"
$otpTabCount = Get-OptionalIntegerEnvironmentVariable -Name "CERTUM_LOGIN_TABS_TO_OTP" -DefaultValue 1
$skipUserIdInput = Get-OptionalBooleanEnvironmentVariable -Name "CERTUM_SKIP_USERID_INPUT" -DefaultValue $false
$windowActivationTimeoutSeconds = Get-OptionalIntegerEnvironmentVariable -Name "CERTUM_WINDOW_ACTIVATION_TIMEOUT_SECONDS" -DefaultValue 60
$windowCaption = Get-OptionalStringEnvironmentVariable -Name "CERTUM_WINDOW_CAPTION" -DefaultValue "SimplySign"

if (-not (Test-Path -LiteralPath $exePath -PathType Leaf)) {
    throw "SimplySign Desktop executable not found: $exePath"
}

$otpUri = [uri]$otpUriRaw
$otpCode = Get-TotpCode -OtpUri $otpUri

Restart-SimplySignDesktop

Write-Host "Starting SimplySign Desktop..."
$process = Start-Process -FilePath $exePath -PassThru
try {
    try {
        $null = $process.WaitForInputIdle(15000)
    }
    catch {
        Write-Host "SimplySign Desktop did not report input-idle within 15 seconds. Continuing with active polling."
    }

    $shell = New-Object -ComObject WScript.Shell
    Wait-ForWindowActivation -Shell $shell -Process $process -Caption $windowCaption -TimeoutSeconds $windowActivationTimeoutSeconds

    Write-Host "Sending SimplySign Desktop credentials..."
    Write-Host "OTP field tab count: $otpTabCount"

    if (-not $skipUserIdInput) {
        Clear-ActiveField -Shell $shell
        $shell.SendKeys((ConvertTo-SendKeysLiteral -Value $userId))
        Start-Sleep -Milliseconds 250
    }
    else {
        Write-Host "Skipping user id input because CERTUM_SKIP_USERID_INPUT is enabled."
    }

    for ($index = 0; $index -lt $otpTabCount; $index += 1) {
        $shell.SendKeys("{TAB}")
        Start-Sleep -Milliseconds 250
    }

    Clear-ActiveField -Shell $shell
    $shell.SendKeys((ConvertTo-SendKeysLiteral -Value $otpCode))
    Start-Sleep -Milliseconds 250
    $shell.SendKeys("~")

    Wait-ForCertificateReady
    Write-Host "SimplySign Desktop login flow completed."
}
finally {
    if ($shell) {
        [System.Runtime.InteropServices.Marshal]::ReleaseComObject($shell) | Out-Null
    }
}
