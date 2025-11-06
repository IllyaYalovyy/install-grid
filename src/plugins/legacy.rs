use std::env;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uint};
use std::ptr::{self, NonNull};
use std::sync::Arc;

use gio::ffi::{g_bus_get_sync, G_BUS_TYPE_SYSTEM};
use glib::ffi::g_error_free;
use glib::gobject_ffi::{g_object_unref, GObject};
use parking_lot::Mutex;
use tokio::task;

use crate::ffi;

use super::{AppSummary, PluginExecutionError};

const DEFAULT_LIST_LIMIT: u32 = 0; // 0 means "no limit" in gs_app_query

pub struct FlatpakLoader {
    loader: NonNull<ffi::GsPluginLoader>,
    lock: Mutex<()>,
    plugin_name: String,
}

unsafe impl Send for FlatpakLoader {}
unsafe impl Sync for FlatpakLoader {}

impl FlatpakLoader {
    pub fn new(plugin_name: &str) -> Result<Self, PluginExecutionError> {
        Self::check_environment()?;

        let raw_loader =
            unsafe { ffi::gs_plugin_loader_new(ptr::null_mut(), ptr::null_mut()) };
        let loader = NonNull::new(raw_loader).ok_or_else(|| {
            PluginExecutionError::Operation(
                "gs_plugin_loader_new returned null".to_string(),
            )
        })?;

        let result = Self::initialise_loader(loader);
        if let Err(err) = result {
            unsafe {
                g_object_unref(loader.as_ptr() as *mut GObject);
            }
            return Err(err);
        }

        let instance = Self {
            loader,
            lock: Mutex::new(()),
            plugin_name: plugin_name.to_string(),
        };

        instance.refresh_metadata_blocking()?;

        Ok(instance)
    }

    fn check_environment() -> Result<(), PluginExecutionError> {
        unsafe {
            let mut error: *mut glib::ffi::GError = std::ptr::null_mut();
            let conn = g_bus_get_sync(G_BUS_TYPE_SYSTEM, std::ptr::null_mut(), &mut error);
            if conn.is_null() {
                let message = gerror_to_message(error);
                if !error.is_null() {
                    g_error_free(error);
                }
                return Err(PluginExecutionError::Operation(format!(
                    "Unable to connect to the system D-Bus (required by gnome-software Flatpak plugin): {}\n\
Confirm that you are running inside a desktop session or export DBUS_SYSTEM_BUS_ADDRESS=unix:path=/run/dbus/system_bus_socket.",
                    message
                )));
            } else {
                g_object_unref(conn as *mut GObject);
            }
        }
        Ok(())
    }

    pub async fn list_all_async(
        self: Arc<Self>,
    ) -> Result<Vec<AppSummary>, PluginExecutionError> {
        task::spawn_blocking(move || self.list_all_blocking(DEFAULT_LIST_LIMIT))
            .await
            .map_err(|err| {
                PluginExecutionError::Operation(format!(
                    "legacy Flatpak worker join error: {err}"
                ))
            })?
    }

    fn initialise_loader(loader: NonNull<ffi::GsPluginLoader>) -> Result<(), PluginExecutionError> {
        // Optional plugin search paths.
        if let Some(paths) = env::var_os("INSTALLGRID_GS_PLUGIN_DIR") {
            for path in env::split_paths(&paths) {
                if let Ok(path_str) = path.into_os_string().into_string() {
                    if path_str.is_empty() {
                        continue;
                    }
                    let c_path = CString::new(path_str.clone()).map_err(|_| {
                        PluginExecutionError::Operation(format!(
                            "plugin path contains interior NUL: {path_str}"
                        ))
                    })?;
                    unsafe {
                        ffi::gs_plugin_loader_add_location(
                            loader.as_ptr(),
                            c_path.as_ptr(),
                        );
                    }
                }
            }
        }

        let (_allowlist_storage, allowlist_ptrs) = match env::var("INSTALLGRID_GS_ALLOWLIST") {
            Ok(value) => {
                let entries = value
                    .split(',')
                    .map(str::trim)
                    .filter(|entry| !entry.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<String>>();
                if entries.is_empty() {
                    (Vec::new(), None)
                } else {
                    let storage = to_c_string_array(&entries)
                        .map_err(|msg| PluginExecutionError::Operation(msg))?;
                    let mut ptrs = storage.iter().map(|value| value.as_ptr()).collect::<Vec<_>>();
                    ptrs.push(ptr::null());
                    (storage, Some(ptrs))
                }
            }
            Err(_) => (Vec::new(), None),
        };

        let (_blocklist_storage, blocklist_ptrs) = match env::var("INSTALLGRID_GS_BLOCKLIST") {
            Ok(value) => {
                let entries = value
                    .split(',')
                    .map(str::trim)
                    .filter(|entry| !entry.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<String>>();
                if entries.is_empty() {
                    (Vec::new(), None)
                } else {
                    let storage = to_c_string_array(&entries)
                        .map_err(|msg| PluginExecutionError::Operation(msg))?;
                    let mut ptrs = storage.iter().map(|value| value.as_ptr()).collect::<Vec<_>>();
                    ptrs.push(ptr::null());
                    (storage, Some(ptrs))
                }
            }
            Err(_) => (Vec::new(), None),
        };

        let allowlist_ptr = allowlist_ptrs
            .as_ref()
            .map(|ptrs| ptrs.as_ptr())
            .unwrap_or(ptr::null());
        let blocklist_ptr = blocklist_ptrs
            .as_ref()
            .map(|ptrs| ptrs.as_ptr())
            .unwrap_or(ptr::null());

        let mut error: *mut ffi::GError = ptr::null_mut();
        let ok = unsafe {
            ffi::gs_plugin_loader_setup(
                loader.as_ptr(),
                allowlist_ptr,
                blocklist_ptr,
                ptr::null_mut(),
                &mut error,
            )
        };
        if ok == 0 {
            let message = unsafe { gerror_to_message(error) };
            unsafe {
                if !error.is_null() {
                    g_error_free(error);
                }
            }
            return Err(PluginExecutionError::Operation(message));
        }

        if env::var_os("INSTALLGRID_DEBUG_GS_STATE").is_some() {
            unsafe {
                ffi::gs_plugin_loader_dump_state(loader.as_ptr());
            }
        }

        Ok(())
    }

    fn list_all_blocking(&self, max_results: u32) -> Result<Vec<AppSummary>, PluginExecutionError> {
        let _guard = self.lock.lock();

        let query = self.create_list_query(max_results)?;
        let _query_guard = GObjectGuard(query.as_ptr() as *mut GObject);

        let job_ptr = unsafe {
            ffi::gs_plugin_job_list_apps_new(
                query.as_ptr(),
                ffi::GS_PLUGIN_LIST_APPS_FLAGS_INTERACTIVE,
            )
        };
        let job = NonNull::new(job_ptr).ok_or_else(|| {
            PluginExecutionError::Operation("gs_plugin_job_list_apps_new returned null".to_string())
        })?;
        let _job_guard = GObjectGuard(job.as_ptr() as *mut GObject);

        let mut error: *mut ffi::GError = ptr::null_mut();
        let ok = unsafe {
            ffi::gs_plugin_loader_job_process(
                self.loader.as_ptr(),
                job.as_ptr(),
                ptr::null_mut(),
                &mut error,
            )
        };
        if ok == 0 {
            let message = unsafe { gerror_to_message(error) };
            unsafe {
                if !error.is_null() {
                    g_error_free(error);
                }
            }
            return Err(PluginExecutionError::Operation(message));
        }

        let list_ptr = unsafe { ffi::gs_plugin_job_list_apps_get_result_list(job.as_ptr()) };
        if list_ptr.is_null() {
            return Ok(Vec::new());
        }

        let length = unsafe { ffi::gs_app_list_length(list_ptr) };
        let mut apps = Vec::with_capacity(length as usize);
        for index in 0..length {
            let app_ptr = unsafe { ffi::gs_app_list_index(list_ptr, index) };
            if app_ptr.is_null() {
                continue;
            }

            let id = unsafe { cstring_ptr_to_string(ffi::gs_app_get_id(app_ptr)) }
                .unwrap_or_else(|| "unknown".to_string());
            let name =
                unsafe { cstring_ptr_to_string(ffi::gs_app_get_name(app_ptr)) }.unwrap_or_else(|| id.clone());
            let summary =
                unsafe { cstring_ptr_to_string(ffi::gs_app_get_summary(app_ptr)) }.unwrap_or_default();
            let source = unsafe { cstring_ptr_to_string(ffi::gs_app_get_origin(app_ptr)) }
                .unwrap_or_else(|| self.plugin_name.clone());

            apps.push(AppSummary {
                app_id: id,
                name,
                summary,
                source,
            });
        }

        if apps.is_empty() {
            return Ok(apps);
        }

        Ok(apps)
    }

    fn refresh_metadata_blocking(&self) -> Result<(), PluginExecutionError> {
        let _guard = self.lock.lock();

        let job_ptr = unsafe {
            ffi::gs_plugin_job_refresh_metadata_new(u64::MAX, ffi::GS_PLUGIN_REFRESH_METADATA_FLAGS_NONE)
        };
        let job = NonNull::new(job_ptr).ok_or_else(|| {
            PluginExecutionError::Operation(
                "gs_plugin_job_refresh_metadata_new returned null".to_string(),
            )
        })?;
        let _job_guard = GObjectGuard(job.as_ptr() as *mut GObject);

        let mut error: *mut ffi::GError = ptr::null_mut();
        let ok = unsafe {
            ffi::gs_plugin_loader_job_process(
                self.loader.as_ptr(),
                job.as_ptr(),
                ptr::null_mut(),
                &mut error,
            )
        };

        if ok == 0 {
            let message = unsafe { gerror_to_message(error) };
            unsafe {
                if !error.is_null() {
                    g_error_free(error);
                }
            }
            return Err(PluginExecutionError::Operation(message));
        }

        Ok(())
    }

    fn create_list_query(
        &self,
        max_results: u32,
    ) -> Result<NonNull<ffi::GsAppQuery>, PluginExecutionError> {
        let is_curated = CString::new("is-curated").unwrap();
        let max_results_key = CString::new("max-results").unwrap();
        let refine_flags_key = CString::new("refine-flags").unwrap();
        let dedupe_flags_key = CString::new("dedupe-flags").unwrap();

        let license_type_key = CString::new("license-type").unwrap();

        let refine_flags: c_uint = ffi::GS_PLUGIN_REFINE_FLAGS_REQUIRE_RATING
            | ffi::GS_PLUGIN_REFINE_FLAGS_REQUIRE_CATEGORIES
            | ffi::GS_PLUGIN_REFINE_FLAGS_REQUIRE_ICON
            | ffi::GS_PLUGIN_REFINE_FLAGS_REQUIRE_ORIGIN;

        let dedupe_flags: c_uint = ffi::GS_APP_LIST_FILTER_FLAG_PREFER_INSTALLED
            | ffi::GS_APP_LIST_FILTER_FLAG_KEY_ID_PROVIDES;

        let query_ptr = unsafe {
            ffi::gs_app_query_new(
                is_curated.as_ptr(),
                ffi::GS_APP_QUERY_TRISTATE_TRUE,
                max_results_key.as_ptr(),
                max_results as c_uint,
                refine_flags_key.as_ptr(),
                refine_flags,
                dedupe_flags_key.as_ptr(),
                dedupe_flags,
                license_type_key.as_ptr(),
                ffi::GS_APP_QUERY_LICENSE_ANY,
                ptr::null::<c_char>(),
            )
        };

        let query = NonNull::new(query_ptr).ok_or_else(|| {
            PluginExecutionError::Operation(
                "gs_app_query_new returned null".to_string(),
            )
        })?;

        Ok(query)
    }
}

impl Drop for FlatpakLoader {
    fn drop(&mut self) {
        unsafe {
            g_object_unref(self.loader.as_ptr() as *mut GObject);
        }
    }
}

pub async fn list_all_apps(
    loader: Arc<FlatpakLoader>,
) -> Result<Vec<AppSummary>, PluginExecutionError> {
    loader.list_all_async().await
}

fn to_c_string_array(values: &[String]) -> Result<Vec<CString>, String> {
    values
        .iter()
        .map(|value| {
            CString::new(value.as_str()).map_err(|_| {
                format!("value contains interior NUL byte: {value}")
            })
        })
        .collect()
}

unsafe fn gerror_to_message(error: *mut ffi::GError) -> String {
    if error.is_null() {
        return "legacy plugin raised an unknown error".to_string();
    }

    let message_ptr = (*error).message;
    if message_ptr.is_null() {
        return format!("legacy plugin error code {}", (*error).code);
    }

    CStr::from_ptr(message_ptr)
        .to_string_lossy()
        .into_owned()
}

unsafe fn cstring_ptr_to_string(ptr: *const std::os::raw::c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

struct GObjectGuard(*mut GObject);

impl Drop for GObjectGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.0.is_null() {
                g_object_unref(self.0);
            }
        }
    }
}
