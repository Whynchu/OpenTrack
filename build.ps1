param(
    [ValidateSet('x64', 'x86', 'all')]
    [string]$Architecture = 'x64'
)

$ErrorActionPreference = 'Stop'
$manifest = Join-Path $PSScriptRoot 'Cargo.toml'
$cache = Join-Path $PSScriptRoot '.build-cache'
New-Item -ItemType Directory -Path $cache -Force | Out-Null
$cacheDirectory = Get-Item -LiteralPath $cache -Force
$cacheDirectory.Attributes = $cacheDirectory.Attributes -bor [System.IO.FileAttributes]::Hidden
$architectures = if ($Architecture -eq 'all') { @('x64', 'x86') } else { @($Architecture) }

foreach ($arch in $architectures) {
    $triple = if ($arch -eq 'x64') { 'x86_64-pc-windows-msvc' } else { 'i686-pc-windows-msvc' }
    & cargo build --release -p opentrack --manifest-path $manifest --target $triple --target-dir $cache
    if ($LASTEXITCODE -ne 0) { throw "Build failed for $arch." }
    $destination = Join-Path $PSScriptRoot "builds\$arch"
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $cache "$triple\release\opentrack.exe") -Destination (Join-Path $destination 'OpenTrack.exe') -Force
    Write-Output "Current $arch build: $(Join-Path $destination 'OpenTrack.exe')"
}
