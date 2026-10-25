param(
    [ValidateSet('x64', 'x86', 'all')]
    [string]$Architecture = 'x64',
    [switch]$LocalAsio
)

$ErrorActionPreference = 'Stop'
$runningOpenTrack = Get-Process -Name OpenTrack -ErrorAction SilentlyContinue
if ($runningOpenTrack) { throw 'Close OpenTrack through its UI before building.' }
$manifest = Join-Path $PSScriptRoot 'Cargo.toml'
$cache = Join-Path $PSScriptRoot '.build-cache'
New-Item -ItemType Directory -Path $cache -Force | Out-Null
$cacheDirectory = Get-Item -LiteralPath $cache -Force
$cacheDirectory.Attributes = $cacheDirectory.Attributes -bor [System.IO.FileAttributes]::Hidden
$architectures = if ($Architecture -eq 'all') { @('x64', 'x86') } else { @($Architecture) }

foreach ($arch in $architectures) {
    $triple = if ($arch -eq 'x64') { 'x86_64-pc-windows-msvc' } else { 'i686-pc-windows-msvc' }
    $audioFeatures = if ($LocalAsio) { @('--features', 'local-asio') } else { @() }
    & cargo build @audioFeatures --release -p opentrack --manifest-path $manifest --target $triple --target-dir $cache
    if ($LASTEXITCODE -ne 0) { throw "Build failed for $arch." }
    $destination = Join-Path $PSScriptRoot "builds\$arch"
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $cache "$triple\release\opentrack.exe") -Destination (Join-Path $destination 'OpenTrack.exe') -Force
    if ($LocalAsio) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'vendor\maolan-asio\LICENSE') -Destination (Join-Path $destination 'maolan-asio-LICENSE.txt') -Force
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'docs\third-party-notices.md') -Destination (Join-Path $destination 'third-party-notices.md') -Force
    }
    Write-Output "Current $arch build: $(Join-Path $destination 'OpenTrack.exe')"
}
