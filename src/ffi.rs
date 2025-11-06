#![allow(dead_code)]

#[cfg(feature = "legacy-ffi")]
use std::os::raw::{c_char, c_int, c_uint};

#[cfg(feature = "legacy-ffi")]
pub type GObject = glib::gobject_ffi::GObject;
#[cfg(not(feature = "legacy-ffi"))]
#[repr(C)]
pub struct GObject {
    _data: [u8; 0],
}

#[cfg(feature = "legacy-ffi")]
pub type GCancellable = gio::ffi::GCancellable;
#[cfg(not(feature = "legacy-ffi"))]
#[repr(C)]
pub struct GCancellable {
    _data: [u8; 0],
}

#[cfg(feature = "legacy-ffi")]
pub type GError = glib::ffi::GError;
#[cfg(not(feature = "legacy-ffi"))]
#[repr(C)]
pub struct GError {
    _data: [u8; 0],
}

/// Opaque handle to the legacy plugin loader.
pub enum GsPluginLoader {}
/// Opaque plugin job handle.
pub enum GsPluginJob {}
/// Opaque app list handle.
pub enum GsAppList {}
/// Opaque plugin handle.
pub enum GsPlugin {}
/// Opaque plugin event handle.
pub enum GsPluginEvent {}

#[repr(C)]
pub struct GsAppQuery {
    _data: [u8; 0],
}

#[repr(C)]
pub struct GsApp {
    _data: [u8; 0],
}

#[cfg(feature = "legacy-ffi")]
pub const GS_APP_QUERY_TRISTATE_TRUE: c_int = 1;
#[cfg(feature = "legacy-ffi")]
pub const GS_APP_QUERY_LICENSE_ANY: c_uint = 0;
#[cfg(feature = "legacy-ffi")]
pub const GS_PLUGIN_LIST_APPS_FLAGS_NONE: c_uint = 0;
#[cfg(feature = "legacy-ffi")]
pub const GS_PLUGIN_LIST_APPS_FLAGS_INTERACTIVE: c_uint = 1 << 0;
#[cfg(feature = "legacy-ffi")]
pub const GS_PLUGIN_REFINE_FLAGS_NONE: c_uint = 0;
#[cfg(feature = "legacy-ffi")]
pub const GS_PLUGIN_REFINE_FLAGS_REQUIRE_DESCRIPTION: c_uint = 1 << 3;
#[cfg(feature = "legacy-ffi")]
pub const GS_PLUGIN_REFINE_FLAGS_REQUIRE_ICON: c_uint = 1 << 21;
#[cfg(feature = "legacy-ffi")]
pub const GS_PLUGIN_REFINE_FLAGS_REQUIRE_RATING: c_uint = 1 << 5;
#[cfg(feature = "legacy-ffi")]
pub const GS_PLUGIN_REFINE_FLAGS_REQUIRE_CATEGORIES: c_uint = 1 << 27;
#[cfg(feature = "legacy-ffi")]
pub const GS_PLUGIN_REFINE_FLAGS_REQUIRE_ORIGIN: c_uint = 1 << 10;
#[cfg(feature = "legacy-ffi")]
pub const GS_PLUGIN_REFRESH_METADATA_FLAGS_NONE: c_uint = 0;
#[cfg(feature = "legacy-ffi")]
pub const GS_APP_LIST_FILTER_FLAG_NONE: c_uint = 0;
#[cfg(feature = "legacy-ffi")]
pub const GS_APP_LIST_FILTER_FLAG_PREFER_INSTALLED: c_uint = 1 << 3;
#[cfg(feature = "legacy-ffi")]
pub const GS_APP_LIST_FILTER_FLAG_KEY_ID_PROVIDES: c_uint = 1 << 4;

#[cfg(feature = "legacy-ffi")]
extern "C" {
    pub fn gs_plugin_loader_new(
        session_bus: *mut GObject,
        system_bus: *mut GObject,
    ) -> *mut GsPluginLoader;
    pub fn gs_plugin_loader_add_location(loader: *mut GsPluginLoader, location: *const c_char);
    pub fn gs_plugin_loader_setup(
        loader: *mut GsPluginLoader,
        allowlist: *const *const c_char,
        blocklist: *const *const c_char,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> glib::ffi::gboolean;
    pub fn gs_plugin_loader_dump_state(loader: *mut GsPluginLoader);
    pub fn gs_plugin_loader_job_process(
        loader: *mut GsPluginLoader,
        job: *mut GsPluginJob,
        cancellable: *mut GCancellable,
        error: *mut *mut GError,
    ) -> glib::ffi::gboolean;
    pub fn gs_plugin_job_list_apps_new(
        query: *mut GsAppQuery,
        flags: c_uint,
    ) -> *mut GsPluginJob;
    pub fn gs_plugin_job_list_apps_get_result_list(job: *mut GsPluginJob) -> *mut GsAppList;
    pub fn gs_plugin_job_refresh_metadata_new(
        cache_age_secs: u64,
        flags: c_uint,
    ) -> *mut GsPluginJob;
    pub fn gs_app_query_new(first_property_name: *const c_char, ...) -> *mut GsAppQuery;
    pub fn gs_app_list_length(apps: *mut GsAppList) -> c_uint;
    pub fn gs_app_list_index(apps: *mut GsAppList, index: c_uint) -> *mut GsApp;
    pub fn gs_app_get_id(app: *mut GsApp) -> *const c_char;
    pub fn gs_app_get_name(app: *mut GsApp) -> *const c_char;
    pub fn gs_app_get_summary(app: *mut GsApp) -> *const c_char;
    pub fn gs_app_get_origin(app: *mut GsApp) -> *const c_char;
}

#[cfg(not(feature = "legacy-ffi"))]
pub mod stubs {
    use super::*;

    pub unsafe fn gs_plugin_loader_new(
        _session_bus: *mut GObject,
        _system_bus: *mut GObject,
    ) -> *mut GsPluginLoader {
        std::ptr::null_mut()
    }
}
