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
    $destination = Join-Path $PSScriptRoot "builds\$arch"
    $buildInfoPath = Join-Path $destination 'build-info.json'
    $useLocalAsio = [bool]$LocalAsio
    if (-not $PSBoundParameters.ContainsKey('LocalAsio')) {
        if (Test-Path -LiteralPath $buildInfoPath) {
            $previousBuildInfo = Get-Content -LiteralPath $buildInfoPath -Raw | ConvertFrom-Json
            $useLocalAsio = [bool]$previousBuildInfo.local_asio
        } else {
            # Older local-ASIO builds predate build-info.json but include this notice.
            $useLocalAsio = Test-Path -LiteralPath (Join-Path $destination 'maolan-asio-LICENSE.txt')
        }
    }
    $audioFeatures = if ($useLocalAsio) { @('--features', 'local-asio') } else { @() }
    Write-Output "Building $arch with local ASIO support: $useLocalAsio (WASAPI remains available)"
    # Windows PowerShell promotes native stderr to ErrorRecord; cargo writes
    # normal progress to stderr, so keep Stop preference from aborting the build
    # before the exit code is checked.
    $buildErrorPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & cargo build @audioFeatures --release -p opentrack --manifest-path $manifest --target $triple --target-dir $cache
    $buildExitCode = $LASTEXITCODE
    $ErrorActionPreference = $buildErrorPreference
    if ($buildExitCode -ne 0) { throw "Build failed for $arch." }
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $cache "$triple\release\opentrack.exe") -Destination (Join-Path $destination 'OpenTrack.exe') -Force
    if ($useLocalAsio) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'vendor\maolan-asio\LICENSE') -Destination (Join-Path $destination 'maolan-asio-LICENSE.txt') -Force
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'docs\third-party-notices.md') -Destination (Join-Path $destination 'third-party-notices.md') -Force
    }
    [ordered]@{
        architecture = $arch
        local_asio = $useLocalAsio
        executable_sha256 = (Get-FileHash -LiteralPath (Join-Path $destination 'OpenTrack.exe') -Algorithm SHA256).Hash
    } | ConvertTo-Json | Set-Content -LiteralPath $buildInfoPath -Encoding UTF8
    Write-Output "Current $arch build: $(Join-Path $destination 'OpenTrack.exe')"
}
