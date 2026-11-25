param(
    [string]$Version = "",
    [switch]$SkipFlutterBuild
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    # バージョン未指定時は pubspec.yaml から自動取得
    if ([string]::IsNullOrWhiteSpace($Version)) {
        $pubspec = Get-Content "pubspec.yaml" -Raw -Encoding UTF8
        if ($pubspec -match '(?m)^version:\s*([0-9]+\.[0-9]+\.[0-9]+)') {
            $Version = $Matches[1]
        } else {
            throw "pubspec.yaml からバージョンを取得できませんでした。"
        }
    }
    $Version = $Version.Trim().TrimStart('v')
    Write-Host "=== Stosup MSI ビルド開始 (v$Version) ==="

    if (-not $SkipFlutterBuild) {
        flutter build windows --release --build-name=$Version
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }

    $buildDir = Join-Path $repoRoot "build\windows\x64\runner\Release"
    if (-not (Test-Path (Join-Path $buildDir "stosup.exe"))) {
        throw "リリースビルドが見つかりません: $buildDir"
    }

    $distDir = Join-Path $repoRoot "dist"
    New-Item -ItemType Directory -Force -Path $distDir | Out-Null

    # プロジェクトローカルの WiX v5 を復元してビルド
    dotnet tool restore
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $msiName = "Stosup-v$Version-x64.msi"
    $msiPath = Join-Path $distDir $msiName
    $iconPath = Join-Path $repoRoot "windows\runner\resources\app_icon.ico"

    dotnet wix build `
        -arch x64 `
        -d "AppVersion=$Version" `
        -d "BuildDir=$buildDir" `
        -d "IconPath=$iconPath" `
        -o $msiPath `
        "installer\stosup.wxs"
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    # SHA-256 チェックサム算出
    $hash = (Get-FileHash -Path $msiPath -Algorithm SHA256).Hash.ToLower()
    $checksumPath = Join-Path $distDir "SHA256SUMS.txt"
    "$hash  $msiName" | Set-Content -Path $checksumPath -Encoding UTF8

    Write-Host "=== MSI ビルド完了 ==="
    Write-Host "MSI: $msiPath ($((Get-Item $msiPath).Length) bytes)"
    Write-Host "SHA256: $hash"
} finally {
    Pop-Location
}
