# InstallGrid Architecture Draft

## Goals
- Provide a Rust-first façade over the existing `GsPluginLoader` so the new application core can stay in Rust while legacy C plugins continue to function unchanged.
- Isolate plugin failures by executing plugin jobs on dedicated worker contexts with panic/unwind boundaries and structured error mapping.
- Enable native Rust plugins to be written side-by-side with existing C plugins behind the same abstraction.
- Support asynchronous, cancelable operations to keep the GTK4/libadwaita UI responsive and allow background refresh with local caching.

## Layered Model
1. **ffi (Unsafe Layer)**  
   - Thin declarations for the subset of `gs_plugin_loader_*` and `gs_plugin_job_*` APIs needed for the InstallGrid prototype.  
   - Responsible for translating between C types (`GsPluginLoader`, `GsPluginJob`, `GsAppList`) and safe Rust handles.
2. **core::PluginHost (Safe Host Layer)**  
   - Lazily initialises `GsPluginLoader` instances for legacy plugins and guards them with a `Mutex` so calls execute sequentially.  
   - Exposes async methods (`list_apps`, `refresh_metadata`, `install_apps`, etc.) returning `Result<T, PluginError>`.  
   - Offloads blocking C calls onto `tokio::task::spawn_blocking`, keeping the UI executor responsive.  
   - Guards each request with error mapping; crashes (SIGABRT, panic) propagate as `PluginError::Fatal`.
3. **core::PluginRegistry**  
   - Maintains metadata for each loaded plugin (name, capabilities, health state).  
   - Allows mixing `LegacyPlugin` (backed by the FFI loader) and `RustPlugin` implementations that implement a `PluginBackend` trait.
4. **domain::AppStoreService**  
   - High-level service combining plugin operations with local cache (SQLite via `rusqlite` for the prototype).  
   - Normalizes data into Rust domain structs (`AppSummary`, `Category`, etc.) and provides stream-based background refresh.
5. **ui::InstallGridWindow (planned)**  
   - Minimal libadwaita window showing cached apps in a `gtk::ListView`, refresh button, and background status indicator driven by async tasks.

## Concurrency Model
- Legacy calls use `gs_plugin_loader_job_process()`, which spins its own temporary `GMainLoop`; we serialize access with a `Mutex` and offload the work to `spawn_blocking`.
- Rust async runtime (the prototype uses the multi-threaded `tokio` runtime) orchestrates background refresh.
- Communication between UI (GTK main thread) and runtime uses `glib::MainContext::channel`.

## Failure Isolation
- Each plugin request is executed through `PluginTask`, which wraps the FFI call in `catch_unwind` (for Rust plugins) and monitors GLib warnings.  
- If a plugin crashes or returns an error deemed fatal, the registry marks it unhealthy and surfaces a degraded-but-running state to the UI.
- Optional future extension: move plugin execution to helper processes via D-Bus IPC; the architecture keeps that door open by funneling operations through the `PluginBackend` trait.

## InstallGrid Prototype Scope
- Implement `PluginHost::list_popular_apps()` calling `gs_plugin_loader_job_process()` to fetch curated Flatpak apps through the real GNOME Software plugins.  
- Stub local cache in-memory with `Arc<RwLock<Vec<AppSummary>>>`, leaving hooks for SQLite.  
- UI displays the cached list and runs refresh in the background without blocking.

## Next Steps
1. Add timeouts, cancellation, and richer diagnostics around the Flatpak bridge.  
2. Swap the in-memory cache for persistent storage (e.g., SQLite).  
3. Expand host coverage to install/update jobs and surface plugin health metrics.  
4. Introduce Rust-native plugin skeleton to validate the trait path.
