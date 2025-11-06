# InstallGrid

InstallGrid is the Rust-centric core we’re shaping for GNOME Software’s next-generation experience.  
It focuses on three aspects:

1. **Plugin Host Abstraction** – runs legacy (C) and native (Rust) plugins behind a single async API, capturing crashes and surfacing warnings.  
2. **Background-Friendly Service Layer** – caches application metadata in memory (pluggable storage coming next) and delivers refresh results without blocking the UI thread.  
3. **GTK4/libadwaita UI** – minimal window that shows cached data, triggers background refresh, and exposes plugin health.

The goal is to validate the architecture before wiring real plugins and a persistent cache.

## Project Layout

```
install-grid/
├── Cargo.toml          # Crate definition and dependencies
├── README.md           # This file
├── scripts/            # Tooling to verify native dependencies
├── src/
│   ├── bin/install_grid.rs  # Entry point launching the libadwaita demo
│   ├── ffi.rs            # Optional bindings to the C plugin loader (gated)
│   ├── host.rs           # Runtime, caching, and isolation logic
│   ├── lib.rs            # Module wiring
│   ├── plugins.rs        # Plugin trait + mock/native adapters
│   └── ui.rs             # GTK4 user interface
└── doc/
    └── install-grid-architecture.md  # High-level design notes
```

## System Requirements

- Rust toolchain (1.74+ recommended)
- Development packages for GTK 4 and libadwaita (for example on Fedora:  
  `sudo dnf install gtk4-devel libadwaita-devel`  
  on Debian/Ubuntu: `sudo apt install libgtk-4-dev libadwaita-1-dev`)
- `pkg-config` must be available so `gtk4` and `libadwaita` crates can locate native headers and libraries.
- **Optional (legacy Flatpak bridge)**: GNOME Software development files (`pkg-config --exists gnome-software`) and the Flatpak plugin binaries. See the instructions below for a reproducible setup.

## Building & Running

```bash
cd install-grid
cargo run
```

This launches the UI backed by the native Rust mock plugin. The real Flatpak data path is available behind the `legacy-ffi` feature.

### Legacy Flatpak Integration (optional)

InstallGrid can call GNOME Software’s Flatpak plugin through `GsPluginLoader` when the `legacy-ffi` feature is enabled. The flow below keeps the setup reproducible for contributors.

1. Install GNOME Software development headers. On Fedora:  
   `sudo dnf install gnome-software-devel`  
   On Debian/Ubuntu:  
   `sudo apt install libgnome-software-dev`

   Alternatively, build the local checkout under `/home/etf/Projects/gnome-software`:
   ```bash
   cd /home/etf/Projects/gnome-software
   meson setup builddir
   meson compile -C builddir
   ```

2. Point InstallGrid at the plugin binaries (skip if you installed distro packages):
   ```bash
   export INSTALLGRID_GS_PLUGIN_DIR="$HOME/Projects/gnome-software/builddir/plugins"
   ```
   Optional allow/block lists  
   `export INSTALLGRID_GS_ALLOWLIST="core,appstream,icons,flatpak"`  
   `export INSTALLGRID_GS_BLOCKLIST="packagekit"`

3. Verify the environment:
   ```bash
   cd install-grid
   ./scripts/check-legacy-ffi.sh
   ```

4. Build and run with the legacy feature:
   ```bash
   cargo run --features legacy-ffi
   ```

   When running headless (CI, SSH, or containers without a display server) set `INSTALLGRID_HEADLESS=1` to force the text-mode refresh:
   ```bash
   INSTALLGRID_HEADLESS=1 cargo run --features legacy-ffi
   ```

   The Flatpak plugin expects to talk to the system D-Bus and Flatpak daemon. Run InstallGrid from a GNOME desktop session (or any environment where the system bus is reachable); otherwise, the legacy backend will warn that it cannot connect and only the mock data will be shown.

#### Troubleshooting Flatpak integration

If `cargo run --features legacy-ffi` only shows the mock applications, check the following:

1. **Verify the development headers and plugin directory** – run `./scripts/check-legacy-ffi.sh`. If you built GNOME Software locally, export `INSTALLGRID_GS_PLUGIN_DIR` to the Meson `builddir/plugins`.
2. **Confirm Flatpak and AppStream metadata exist** – `flatpak remotes` should list remotes and `/var/lib/flatpak/appstream` (or your distro equivalent) should contain data.
3. **Ensure the system D-Bus is accessible** – the process must be able to connect to `/run/dbus/system_bus_socket`. Inside containers or SSH sessions, export `DBUS_SYSTEM_BUS_ADDRESS=unix:path=/run/dbus/system_bus_socket` before launching InstallGrid. Without this connection you’ll see the warning `Unable to connect to the system D-Bus ...` and the legacy plugin will be skipped.
4. **Run inside an active desktop session** – the Flatpak plugin also requires the Flatpak system service. On headless hosts ensure `flatpak` is installed and running, or test the environment by launching `gnome-software --headless` from the same session.

When the bridge is active InstallGrid lists curated Flatpak apps from the real plugin while keeping the UI responsive.

## Next Steps

- Harden the Flatpak bridge with timeouts, cancellation, and richer error mapping.
- Add a persistent cache (e.g. SQLite) so startup is instant even without network access.
- Expand the UI to cover install/update flows and expose plugin health metrics.
