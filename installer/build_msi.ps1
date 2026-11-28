param(
    [string]$Version = "",
    [switch]$SkipFlutterBuild
)

$ErrorActionPreference = "Stop"

# Generate MSI-safe Japanese RTF 1.x (CP932 + Unicode \uN? escapes) from UTF-8 License.md
function Convert-Utf8TextToJapaneseRtf {
    param(
        [Parameter(Mandatory = $true)][string]$InputPath,
        [Parameter(Mandatory = $true)][string]$OutputPath
    )

    $lines = Get-Content -Path $InputPath -Encoding UTF8
    $backtick = [string][char]96
    $sb = New-Object System.Text.StringBuilder
    [void]$sb.Append('{\rtf1\ansi\ansicpg932\deff0\nouicompat{\fonttbl{\f0\fnil\fcharset128 Meiryo UI;}{\f1\fnil\fcharset128 MS Gothic;}}\viewkind4\uc1')
    [void]$sb.Append("`r`n")
    [void]$sb.Append('\pard\sa80\sl252\slmult1\f0\fs19 ')

    for ($idx = 0; $idx -lt $lines.Count; $idx++) {
        $rawLine = $lines[$idx]
        $isTitle = ($rawLine -match '^#\s+')
        $isHeading = ($rawLine -match '^#{2,3}\s+')

        # Strip simple Markdown heading / list / inline-code markers for clean RTF output
        $line = $rawLine -replace '^#{1,3}\s+', ''
        $line = $line -replace '^\-\s+', ''
        $line = $line.Replace($backtick, '')

        if ($isTitle) {
            [void]$sb.Append('\b\fs24 ')
        } elseif ($isHeading) {
            [void]$sb.Append('\b\fs20 ')
        }

        foreach ($ch in $line.ToCharArray()) {
            $code = [int][char]$ch
            if ($ch -eq '\' -or $ch -eq '{' -or $ch -eq '}') {
                [void]$sb.Append('\')
                [void]$sb.Append($ch)
            } elseif ($code -ge 0x20 -and $code -le 0x7E) {
                [void]$sb.Append($ch)
            } else {
                # RTF \uN? uses signed 16-bit integer (values >= 32768 wrap by subtracting 65536)
                $signed = if ($code -gt 32767) { $code - 65536 } else { $code }
                [void]$sb.Append("\u$($signed)?")
            }
        }

        if ($isTitle -or $isHeading) {
            [void]$sb.Append('\b0\fs19')
        }
        [void]$sb.Append("\par`r`n")
    }

    [void]$sb.Append('}')
    [System.IO.File]::WriteAllText($OutputPath, $sb.ToString(), [System.Text.Encoding]::ASCII)
}

$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    if ([string]::IsNullOrWhiteSpace($Version)) {
        $pubspec = Get-Content "pubspec.yaml" -Raw -Encoding UTF8
        if ($pubspec -match '(?m)^version:\s*([0-9]+\.[0-9]+\.[0-9]+)') {
            $Version = $Matches[1]
        } else {
            throw "Failed to detect version from pubspec.yaml."
        }
    }
    $Version = $Version.Trim().TrimStart('v')
    Write-Host "=== Building Stosup MSI (v$Version) ==="

    if (-not $SkipFlutterBuild) {
        flutter build windows --release --build-name=$Version
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }

    $buildDir = Join-Path $repoRoot "build\windows\x64\runner\Release"
    if (-not (Test-Path (Join-Path $buildDir "stosup.exe"))) {
        throw "Release build not found: $buildDir"
    }

    $distDir = Join-Path $repoRoot "dist"
    New-Item -ItemType Directory -Force -Path $distDir | Out-Null

    # Restore local WiX Toolset v5 and ensure UI extension is installed
    dotnet tool restore
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $extList = (& dotnet wix extension list 2>&1) | Out-String
    if ($extList -notmatch 'WixToolset\.UI\.wixext\s+5\.0\.2') {
        Write-Host "Installing WixToolset.UI.wixext/5.0.2..."
        dotnet wix extension add WixToolset.UI.wixext/5.0.2
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }

    # Convert root License.md to Japanese RTF for MSI LicenseAgreementDlg
    $licenseMdPath = Join-Path $repoRoot "License.md"
    $licenseRtfPath = Join-Path $repoRoot "installer\license_ja.rtf"
    if (-not (Test-Path $licenseMdPath)) {
        throw "License file not found: $licenseMdPath"
    }
    Convert-Utf8TextToJapaneseRtf -InputPath $licenseMdPath -OutputPath $licenseRtfPath

    $msiName = "Stosup-v$Version-x64.msi"
    $msiPath = Join-Path $distDir $msiName
    $iconPath = Join-Path $repoRoot "windows\runner\resources\app_icon.ico"
    # WiX Package Version は major.minor.patch 数値のみ受付のため、pre 等の接尾辞を除いた数値部を渡す
    $msiProductVersion = if ($Version -match '^([0-9]+\.[0-9]+\.[0-9]+)') { $Matches[1] } else { $Version }

    dotnet wix build `
        -arch x64 `
        -ext WixToolset.UI.wixext `
        -culture ja-JP `
        -d "AppVersion=$msiProductVersion" `
        -d "BuildDir=$buildDir" `
        -d "IconPath=$iconPath" `
        -d "LicenseRtfPath=$licenseRtfPath" `
        -o $msiPath `
        "installer\stosup.wxs"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $hash = (Get-FileHash -Path $msiPath -Algorithm SHA256).Hash.ToLower()
    $checksumPath = Join-Path $distDir "SHA256SUMS.txt"
    "$hash  $msiName" | Set-Content -Path $checksumPath -Encoding UTF8

    Write-Host "=== MSI Build Completed ==="
    Write-Host "MSI: $msiPath ($((Get-Item $msiPath).Length) bytes)"
    Write-Host "SHA256: $hash"
} finally {
    Pop-Location
}
