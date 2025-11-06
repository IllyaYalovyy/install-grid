use std::future::Future;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use anyhow::Context;
use async_channel::bounded;
use futures::future::{join_all, BoxFuture};
use futures::FutureExt;
use parking_lot::RwLock;
use thiserror::Error;

use crate::plugins::{AppSummary, PluginBackend, PluginDescriptor, PluginFailure, PluginFailureKind};

#[derive(Debug, Error)]
pub enum HostError {
    #[error("all plugins failed")]
    AllFailed(Vec<PluginFailure>),
    #[error("host runtime unavailable")]
    RuntimeUnavailable,
}

pub struct PluginHostBuilder {
    plugins: Vec<Arc<dyn PluginBackend>>,
}

impl Default for PluginHostBuilder {
    fn default() -> Self {
        Self { plugins: Vec::new() }
    }
}

impl PluginHostBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_backend<T>(mut self, backend: T) -> Self
    where
        T: PluginBackend + 'static,
    {
        self.plugins.push(Arc::new(backend));
        self
    }

    pub fn build(self) -> anyhow::Result<PluginHost> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("failed to build tokio runtime")?;
        let handle = runtime.handle().clone();

        Ok(PluginHost {
            runtime: Arc::new(runtime),
            handle,
            plugins: Arc::new(self.plugins),
        })
    }
}

#[derive(Clone)]
pub struct PluginHost {
    runtime: Arc<tokio::runtime::Runtime>,
    handle: tokio::runtime::Handle,
    plugins: Arc<Vec<Arc<dyn PluginBackend>>>,
}

pub struct HostResponse<T> {
    pub data: T,
    pub warnings: Vec<PluginFailure>,
}

impl PluginHost {
    pub fn list_popular(
        &self,
    ) -> impl Future<Output = Result<HostResponse<Vec<AppSummary>>, HostError>> {
        let plugins = self.plugins.clone();
        let handle = self.handle.clone();

        async move {
            let (tx, rx) = bounded(1);

            handle.spawn(async move {
                let result = collect_popular(plugins).await;
                let _ = tx.send(result).await;
            });

            rx.recv().await.unwrap_or(Err(HostError::RuntimeUnavailable))
        }
    }
}

async fn collect_popular(
    plugins: Arc<Vec<Arc<dyn PluginBackend>>>,
) -> Result<HostResponse<Vec<AppSummary>>, HostError> {
    let plugin_count = plugins.len();
    let mut tasks: Vec<BoxFuture<'static, Result<Vec<AppSummary>, PluginFailure>>> = Vec::new();

    for backend in plugins.iter().cloned() {
        tasks.push(run_plugin(backend));
    }

    let results = join_all(tasks).await;

    let mut apps = Vec::new();
    let mut warnings = Vec::new();

    for result in results {
        match result {
            Ok(mut chunk) => apps.append(&mut chunk),
            Err(failure) => warnings.push(failure),
        }
    }

    if apps.is_empty() && warnings.len() == plugin_count && plugin_count > 0 {
        return Err(HostError::AllFailed(warnings));
    }

    Ok(HostResponse { data: apps, warnings })
}

fn run_plugin(
    backend: Arc<dyn PluginBackend>,
) -> BoxFuture<'static, Result<Vec<AppSummary>, PluginFailure>> {
    async move {
        let descriptor: PluginDescriptor = backend.descriptor().clone();
        let plugin_name = descriptor.id.clone();
        let plugin_kind = descriptor.kind;

        let result = AssertUnwindSafe(backend.list_popular_apps())
            .catch_unwind()
            .await;

        match result {
            Ok(Ok(apps)) => Ok(apps),
            Ok(Err(kind)) => Err(PluginFailure {
                plugin: plugin_name,
                plugin_kind,
                kind: PluginFailureKind::Execution(kind),
            }),
            Err(_) => Err(PluginFailure {
                plugin: plugin_name,
                plugin_kind,
                kind: PluginFailureKind::Panic,
            }),
        }
    }
    .boxed()
}

#[derive(Clone)]
pub struct AppStoreService {
    host: PluginHost,
    cache: Arc<RwLock<Vec<AppSummary>>>,
    warnings: Arc<RwLock<Vec<PluginFailure>>>,
}

#[derive(Clone)]
pub struct RefreshOutcome {
    pub apps: Vec<AppSummary>,
    pub warnings: Vec<PluginFailure>,
}

impl AppStoreService {
    pub fn new(host: PluginHost) -> Self {
        Self {
            host,
            cache: Arc::new(RwLock::new(Vec::new())),
            warnings: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn cache_snapshot(&self) -> RefreshOutcome {
        RefreshOutcome {
            apps: self.cache.read().clone(),
            warnings: self.warnings.read().clone(),
        }
    }

    pub async fn refresh_popular(&self) -> Result<RefreshOutcome, HostError> {
        let response = self.host.list_popular().await?;

        {
            let mut cache = self.cache.write();
            *cache = response.data.clone();
        }

        {
            let mut warnings = self.warnings.write();
            *warnings = response.warnings.clone();
        }

        Ok(RefreshOutcome {
            apps: response.data,
            warnings: response.warnings,
        })
    }

    pub fn plugin_count(&self) -> usize {
        self.host.plugins.len()
    }
}
