#!/usr/bin/env bash
# Installs the OpenTrack launcher and application icon for the current user.
#
# Linux desktops match a window to its icon through the application id that the
# binary advertises ("opentrack") and a .desktop file of the same name. Without
# both, KDE falls back to a generic placeholder.
#
# The binary is not built here. Build it first, or point OPENTRACK_EXECUTABLE at
# an existing one.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
executable="${OPENTRACK_EXECUTABLE:-$repo_root/builds/x64/OpenTrack}"
icon_dir="$HOME/.local/share/icons/hicolor/scalable/apps"
application_dir="$HOME/.local/share/applications"

if [[ ! -x "$executable" ]]; then
    printf 'No executable at %s\nBuild one first, or set OPENTRACK_EXECUTABLE.\n' "$executable" >&2
    exit 1
fi

mkdir -p "$icon_dir" "$application_dir"
install -m 0644 \
    "$repo_root/crates/opentrack-app/assets/opentrack-icon.svg" \
    "$icon_dir/opentrack.svg"

cat >"$application_dir/opentrack.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=OpenTrack
GenericName=Digital Audio Workstation
Comment=Open-source DAW for recording, editing, arranging and mixing
Exec="$executable"
Icon=opentrack
Terminal=false
StartupNotify=true
StartupWMClass=opentrack
Categories=AudioVideo;Audio;Midi;
Keywords=audio;music;daw;midi;recording;mixer;
EOF

# Refresh the caches the desktop uses to find the entry and the icon.
update-desktop-database "$application_dir" 2>/dev/null || true
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
if command -v kbuildsycoca6 >/dev/null; then
    kbuildsycoca6 >/dev/null 2>&1 || true
fi

printf 'Launcher: %s\nIcon: %s\nExecutable: %s\n' \
    "$application_dir/opentrack.desktop" "$icon_dir/opentrack.svg" "$executable"
