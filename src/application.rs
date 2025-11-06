use gtk4 as gtk;
use libadwaita as adw;

use adw::subclass::prelude::*;
use gio::ApplicationFlags;
use glib::subclass::Signal;
use glib::Object;
use glib::Type;
use once_cell::sync::Lazy;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct InstallGridApplication;

    #[glib::object_subclass]
    impl ObjectSubclass for InstallGridApplication {
        const NAME: &'static str = "InstallGridApplication";
        type Type = super::InstallGridApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for InstallGridApplication {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("repository-changed")
                    .param_types([Type::OBJECT])
                    .action()
                    .run_last()
                    .build()]
            });
            SIGNALS.as_ref()
        }
    }

    impl ApplicationImpl for InstallGridApplication {}
    impl GtkApplicationImpl for InstallGridApplication {}
    impl AdwApplicationImpl for InstallGridApplication {}
}

glib::wrapper! {
    pub struct InstallGridApplication(ObjectSubclass<imp::InstallGridApplication>)
        @extends adw::Application, gtk::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl InstallGridApplication {
    pub fn new(application_id: &str, flags: ApplicationFlags) -> Self {
        Object::builder::<Self>()
            .property("application-id", application_id)
            .property("flags", flags)
            .build()
    }
}
