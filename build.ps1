$ErrorActionPreference = 'Stop'

Set-Location -LiteralPath $PSScriptRoot

$sourceBinaryPath = Join-Path $PSScriptRoot 'target\release\proxyia.exe'
$binaryPath = Join-Path $PSScriptRoot 'proxyia.exe'

Write-Host 'Compilation de ProxyIA (release)...'
cargo build --release

if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

if (-not (Test-Path -LiteralPath $sourceBinaryPath)) {
    throw "Binaire de release introuvable : $sourceBinaryPath"
}

Copy-Item -LiteralPath $sourceBinaryPath -Destination $binaryPath -Force
Write-Host "Compilation terminée : $binaryPath"