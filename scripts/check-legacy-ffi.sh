#!/usr/bin/env bash
set -euo pipefail

if ! command -v pkg-config >/dev/null 2>&1; then
  echo "pkg-config is required but not found in PATH." >&2
  exit 2
fi

if ! pkg-config --exists gnome-software; then
  echo "gnome-software development files are missing. Install libgnome-software dev packages or build GNOME Software from source." >&2
  exit 3
fi

echo "Found gnome-software $(pkg-config --modversion gnome-software)"

if [[ -n "${INSTALLGRID_GS_PLUGIN_DIR:-}" ]]; then
  status=0
  IFS=':' read -ra paths <<<"${INSTALLGRID_GS_PLUGIN_DIR}"
  for path in "${paths[@]}"; do
    if [[ -d "$path" ]]; then
      printf 'plugin dir ok: %s\n' "$path"
    else
      printf 'plugin dir missing: %s\n' "$path" >&2
      status=4
    fi
  done
  if [[ $status -ne 0 ]]; then
    exit "$status"
  fi
else
  cat <<'EOF'
INSTALLGRID_GS_PLUGIN_DIR is not set.
If you built GNOME Software locally, export INSTALLGRID_GS_PLUGIN_DIR to point at the built plugins directory, e.g.:
  export INSTALLGRID_GS_PLUGIN_DIR="$HOME/Projects/gnome-software/builddir/plugins"
EOF
fi

if [[ -n "${INSTALLGRID_GS_ALLOWLIST:-}" ]]; then
  echo "Using custom plugin allowlist: ${INSTALLGRID_GS_ALLOWLIST}"
else
  echo "Using default plugin allowlist: core, appstream, icons, flatpak"
fi
