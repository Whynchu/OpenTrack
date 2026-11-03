#!/usr/bin/env bash
# Build OpenTrack for Linux and stage it in builds/<arch>.
#
# Mirrors build.ps1: the destination is a distribution folder, not a scratchpad,
# so it is replaced as a whole and described by build-info.json.
#
#   ./build.sh              # host architecture
#   ./build.sh --arch arm64 # cross-build (needs a cross linker)
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$root"

if ! command -v cargo >/dev/null 2>&1; then
    printf 'cargo not found; install Rust from https://rustup.rs\n' >&2
    exit 1
fi

requested_arch=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --arch)
            [[ $# -ge 2 ]] || { printf '%s\n' '--arch needs a value' >&2; exit 2; }
            requested_arch="$2"
            shift 2
            ;;
        -h|--help)
            printf 'usage: %s [--arch x64|arm64]\n' "${BASH_SOURCE[0]}"
            exit 0
            ;;
        *)
            printf 'unknown argument: %s\n' "$1" >&2
            exit 2
            ;;
    esac
done

host_triple="$(rustc -vV | awk '/^host: / { print $2 }')"
if [[ -z "$host_triple" ]]; then
    printf '%s\n' 'could not read the host target from rustc' >&2
    exit 1
fi

case "$host_triple" in
    x86_64-*) host_arch="x64" ;;
    aarch64-*) host_arch="arm64" ;;
    *) printf 'unsupported host target: %s\n' "$host_triple" >&2; exit 1 ;;
esac
arch="${requested_arch:-$host_arch}"

case "$arch" in
    x64) triple="x86_64-unknown-linux-gnu" ;;
    arm64) triple="aarch64-unknown-linux-gnu" ;;
    *) printf 'unsupported architecture: %s (expected x64 or arm64)\n' "$arch" >&2; exit 2 ;;
esac

destination="$root/builds/$arch"
build_info="$destination/build-info.json"

# Only rebuild what the destination already holds, and only inside builds/.
case "$destination" in
    "$root/builds/"*) ;;
    *) printf 'refusing to replace %s\n' "$destination" >&2; exit 1 ;;
esac

printf 'Building %s release for %s\n' "$arch" "$triple"
if [[ "$triple" == "$host_triple" ]]; then
    cargo build --release --locked -p opentrack -p opentrack-plugin-scan --features opentrack/linux-realtime
    binary=".build-cache/release/opentrack"
    scanner_binary=".build-cache/release/opentrack-plugin-scan"
else
    cargo build --release --locked -p opentrack -p opentrack-plugin-scan --target "$triple" --features opentrack/linux-realtime
    binary=".build-cache/$triple/release/opentrack"
    scanner_binary=".build-cache/$triple/release/opentrack-plugin-scan"
fi

if [[ ! -f "$binary" ]]; then
    printf 'expected build output at %s\n' "$binary" >&2
    exit 1
fi

rm -rf -- "$destination"
mkdir -p "$destination"
install -m 0755 "$binary" "$destination/OpenTrack"
# The application enumerates each VST3 module in this worker, so it must ship
# beside the executable; without it the app falls back to in-process scanning.
if [[ ! -f "$scanner_binary" ]]; then
    printf 'expected the scanner worker at %s\n' "$scanner_binary" >&2
    exit 1
fi
install -m 0755 "$scanner_binary" "$destination/opentrack-plugin-scan"
if [[ -f docs/third-party-notices.md ]]; then
    install -m 0644 docs/third-party-notices.md "$destination/third-party-notices.md"
fi

sha256="$(sha256sum "$destination/OpenTrack" | cut -d' ' -f1)"
cat >"$build_info" <<EOF
{
    "platform": "linux-$triple",
    "architecture": "$arch",
    "profile": "release",
    "linux_realtime_scheduling": true,
    "executable_sha256": "$sha256"
}
EOF

printf 'Current %s build: %s\n' "$arch" "$destination/OpenTrack"
printf 'SHA-256: %s\n' "$sha256"
