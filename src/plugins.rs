use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
#[cfg(feature = "legacy-ffi")]
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "legacy-ffi")]
mod legacy;

/// Minimal subset of app metadata needed for the InstallGrid UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSummary {
    pub app_id: String,
    pub name: String,
    pub summary: String,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginKind {
    Legacy,
    Native,
}

#[derive(Debug, Clone)]
pub struct PluginDescriptor {
    pub id: String,
    pub kind: PluginKind,
}

#[derive(Debug, Error, Clone)]
pub enum PluginExecutionError {
    #[error("legacy backend unavailable")]
    LegacyUnavailable,
    #[error("operation failed: {0}")]
    Operation(String),
    #[error("timed out after {0:?}")]
    Timeout(Duration),
}

#[derive(Debug, Error, Clone)]
pub enum PluginFailureKind {
    #[error("{0}")]
    Execution(#[from] PluginExecutionError),
    #[error("panic in plugin")]
    Panic,
}

#[derive(Debug, Clone)]
pub struct PluginFailure {
    pub plugin: String,
    pub kind: PluginFailureKind,
    pub plugin_kind: PluginKind,
}

#[async_trait]
pub trait PluginBackend: Send + Sync {
    fn descriptor(&self) -> &PluginDescriptor;
    async fn list_popular_apps(&self) -> Result<Vec<AppSummary>, PluginExecutionError>;
}

pub struct LegacyPluginAdapter {
    descriptor: PluginDescriptor,
    #[cfg_attr(not(feature = "legacy-ffi"), allow(dead_code))]
    plugin_name: Arc<String>,
    #[cfg(feature = "legacy-ffi")]
    loader: OnceCell<Result<Arc<legacy::FlatpakLoader>, PluginExecutionError>>,
}

impl LegacyPluginAdapter {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            descriptor: PluginDescriptor {
                id: format!("legacy::{name}"),
                kind: PluginKind::Legacy,
            },
            plugin_name: Arc::new(name),
            #[cfg(feature = "legacy-ffi")]
            loader: OnceCell::new(),
        }
    }
}

#[async_trait]
impl PluginBackend for LegacyPluginAdapter {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    async fn list_popular_apps(&self) -> Result<Vec<AppSummary>, PluginExecutionError> {
        #[cfg(feature = "legacy-ffi")]
        {
            let loader_entry = self.loader.get_or_init(|| {
                legacy::FlatpakLoader::new(self.plugin_name.as_ref()).map(Arc::new)
            });

            let loader = match loader_entry {
                Ok(handle) => handle.clone(),
                Err(err) => return Err(err.clone()),
            };

            return legacy::list_all_apps(loader).await;
        }

        #[cfg(not(feature = "legacy-ffi"))]
        {
            Err(PluginExecutionError::LegacyUnavailable)
        }
    }
}

pub struct NativeMockPlugin {
    descriptor: PluginDescriptor,
    delay: Duration,
}

impl NativeMockPlugin {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            descriptor: PluginDescriptor {
                id: id.into(),
                kind: PluginKind::Native,
            },
            delay: Duration::from_millis(250),
        }
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

#[async_trait]
impl PluginBackend for NativeMockPlugin {
    fn descriptor(&self) -> &PluginDescriptor {
        &self.descriptor
    }

    async fn list_popular_apps(&self) -> Result<Vec<AppSummary>, PluginExecutionError> {
        tokio::time::sleep(self.delay).await;
        Ok(vec![
            AppSummary {
                app_id: "org.gnome.Fractal".to_string(),
                name: "Fractal".to_string(),
                summary: "Matrix messaging client for GNOME.".to_string(),
                source: "mock::flatpak".to_string(),
            },
            AppSummary {
                app_id: "org.gimp.GIMP".to_string(),
                name: "GNU Image Manipulation Program".to_string(),
                summary: "Powerful graphics editor.".to_string(),
                source: "mock::flatpak".to_string(),
            },
            AppSummary {
                app_id: "org.mozilla.firefox".to_string(),
                name: "Firefox".to_string(),
                summary: "Web browser focused on privacy.".to_string(),
                source: "mock::packagekit".to_string(),
            },
        ])
    }
}
