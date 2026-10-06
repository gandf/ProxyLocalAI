$ErrorActionPreference = 'Stop'

Set-Location -LiteralPath $PSScriptRoot

Write-Host 'Compilation de ProxyIA (release)...'
cargo build --release

if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$binaryPath = Join-Path $PSScriptRoot 'target\release\proxyia.exe'
Write-Host "Compilation terminée : $binaryPath"