use std::rc::Rc;

use adw::prelude::*;
use glib::clone;
use glib::ControlFlow;
use glib::Priority;
use gtk4 as gtk;
use gtk::prelude::*;
use gtk::{gio, glib};
use libadwaita as adw;

use crate::application::InstallGridApplication;
use crate::host::{AppStoreService, HostError, RefreshOutcome};

pub fn run(app_store: AppStoreService) -> glib::ExitCode {
    let application =
        InstallGridApplication::new("org.gnome.InstallGrid", gio::ApplicationFlags::NON_UNIQUE);

    let service = Rc::new(app_store);

    application.connect_startup(|_| {
        if let Err(err) = adw::init() {
            eprintln!("Failed to initialise libadwaita: {err}");
        }
    });

    application.connect_activate(clone!(@weak service => move |app: &InstallGridApplication| {
        build_ui(app, service.clone());
    }));

    application.run()
}

fn build_ui(app: &InstallGridApplication, service: Rc<AppStoreService>) {
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(480)
        .default_height(640)
        .title("InstallGrid")
        .build();

    let header_bar = adw::HeaderBar::new();
    let refresh_button = gtk::Button::from_icon_name("view-refresh-symbolic");
    refresh_button.set_tooltip_text(Some("Refresh application list"));

    let spinner = gtk::Spinner::new();
    spinner.set_spinning(false);
    spinner.set_visible(false);

    header_bar.pack_end(&spinner);
    header_bar.pack_end(&refresh_button);

    let list_box = gtk::ListBox::new();
    list_box.set_margin_top(12);
    list_box.set_margin_bottom(12);
    list_box.set_margin_start(12);
    list_box.set_margin_end(12);

    let status_label = gtk::Label::new(None);
    status_label.set_halign(gtk::Align::Start);

    let warning_label = gtk::Label::new(None);
    warning_label.set_halign(gtk::Align::Start);
    warning_label.add_css_class("dim-label");
    warning_label.set_visible(false);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.append(&header_bar);
    content.append(&status_label);
    content.append(&warning_label);
    content.append(&list_box);

    window.set_content(Some(&content));

    let (sender, receiver) =
        glib::MainContext::channel::<Result<RefreshOutcome, String>>(Priority::default());

    receiver.attach(
        None,
        clone!(@weak list_box, @weak status_label, @weak warning_label, @weak spinner, @weak service => @default-return ControlFlow::Break,
            move |message| {
                spinner.stop();
                spinner.set_visible(false);
                match message {
                    Ok(outcome) => {
                        rebuild_list(&list_box, &outcome.apps);
                        status_label.set_text(&format!(
                            "{} applications ({} plugins)",
                            outcome.apps.len(),
                            service.plugin_count()
                        ));

                        if outcome.warnings.is_empty() {
                            warning_label.set_text("");
                            warning_label.set_visible(false);
                        } else {
                            let joined = outcome
                                .warnings
                                .iter()
                                .map(|failure| format!("{}: {}", failure.plugin, failure.kind))
                                .collect::<Vec<_>>()
                                .join("\n");
                            warning_label.set_text(&joined);
                            warning_label.set_visible(true);
                        }
                    }
                    Err(err) => {
                        warning_label.set_text(&err);
                        warning_label.set_visible(true);
                        status_label.set_text("Refresh failed");
                    }
                }
                ControlFlow::Continue
            }
        ),
    );

    let trigger_refresh: Rc<dyn Fn()> =
        Rc::new(clone!(@weak service, @strong sender, @weak spinner => move || {
            spinner.set_visible(true);
            spinner.start();
            glib::MainContext::default().spawn_local(clone!(@weak service, @strong sender => async move {
                let result = service.refresh_popular().await;
                let _ = sender.send(result.map_err(format_host_error));
            }));
        }));

    refresh_button.connect_clicked(clone!(@strong trigger_refresh => move |_| trigger_refresh()));

    let initial = service.cache_snapshot();
    rebuild_list(&list_box, &initial.apps);
    if initial.warnings.is_empty() {
        warning_label.set_visible(false);
    } else {
        warning_label.set_visible(true);
        warning_label.set_text(
            &initial
                .warnings
                .iter()
                .map(|failure| format!("{}: {}", failure.plugin, failure.kind))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    status_label.set_text(&format!(
        "{} applications cached ({} plugins)",
        initial.apps.len(),
        service.plugin_count()
    ));

    trigger_refresh();

    window.present();
}

fn rebuild_list(list_box: &gtk::ListBox, apps: &[crate::plugins::AppSummary]) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    for app in apps {
        let row = gtk::ListBoxRow::new();
        let box_container = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let title = gtk::Label::new(Some(&app.name));
        title.set_xalign(0.0);
        title.add_css_class("title-3");
        let subtitle = gtk::Label::new(Some(&format!("{} • {}", app.summary, app.source)));
        subtitle.set_wrap(true);
        subtitle.set_xalign(0.0);
        subtitle.add_css_class("dim-label");

        box_container.append(&title);
        box_container.append(&subtitle);
        row.set_child(Some(&box_container));
        list_box.append(&row);
    }
}

fn format_host_error(err: HostError) -> String {
    match err {
        HostError::AllFailed(failures) => {
            if failures.is_empty() {
                "All plugins failed without detailed errors".to_string()
            } else {
                failures
                    .iter()
                    .map(|failure| format!("{}: {}", failure.plugin, failure.kind))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        HostError::RuntimeUnavailable => "Background runtime unavailable".to_string(),
    }
}
