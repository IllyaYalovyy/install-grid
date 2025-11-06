use install_grid::host::{AppStoreService, HostError, PluginHostBuilder};
use install_grid::plugins::{LegacyPluginAdapter, NativeMockPlugin};
use install_grid::ui;

fn main() {
    let host = PluginHostBuilder::new()
        .with_backend(LegacyPluginAdapter::new("flatpak"))
        .with_backend(NativeMockPlugin::new("native::mock").with_delay(
            std::time::Duration::from_millis(120),
        ))
        .build()
        .expect("failed to initialise plugin host");

    let service = AppStoreService::new(host);

    let env_display =
        std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some();
    let force_headless = std::env::var_os("INSTALLGRID_HEADLESS").is_some();
    let display_available = env_display && !force_headless;

    if !display_available {
        eprintln!("InstallGrid: no DISPLAY/WAYLAND_DISPLAY found, running in headless mode");
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to initialise headless runtime");

        match runtime.block_on(service.refresh_popular()) {
            Ok(outcome) => {
                println!("Fetched {} applications:", outcome.apps.len());
                for app in outcome.apps.iter() {
                    println!("- {} ({}) :: {}", app.name, app.app_id, app.source);
                }
                if !outcome.warnings.is_empty() {
                    eprintln!("Warnings:");
                    for warning in outcome.warnings {
                        eprintln!("  {}: {}", warning.plugin, warning.kind);
                    }
                }
            }
            Err(err) => {
                eprintln!("Failed to refresh apps: {}", describe_host_error(err));
                std::process::exit(1);
            }
        }
        return;
    }

    let _exit = ui::run(service);
}

fn describe_host_error(err: HostError) -> String {
    match err {
        HostError::AllFailed(failures) => {
            if failures.is_empty() {
                "All plugins failed without detailed errors".to_string()
            } else {
                failures
                    .into_iter()
                    .map(|failure| format!("{}: {}", failure.plugin, failure.kind))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        HostError::RuntimeUnavailable => "Background runtime unavailable".to_string(),
    }
}
