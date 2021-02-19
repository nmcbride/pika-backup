use gtk::prelude::*;

use crate::borg;
use crate::ui;
use crate::ui::prelude::*;

pub fn init() {
    main_ui().window().connect_delete_event(|_, _| on_delete());

    // decorate headerbar of pre-release versions
    if !option_env!("APPLICATION_ID_SUFFIX")
        .unwrap_or_default()
        .is_empty()
    {
        main_ui().window().get_style_context().add_class("devel");
    }
}

pub fn is_displayed() -> bool {
    main_ui().window().get_id() != 0
}

pub fn show() {
    if !is_displayed() {
        debug!("Displaying ui that was hidden before.");

        gtk_app().add_window(&main_ui().window());
        main_ui().window().show_all();
        main_ui().window().present();

        Handler::run(ui::init_check_borg());
        Handler::run(ui::update_config::run());

        // redo size estimates for backups running in background before
        std::thread::spawn(|| {
            for (config_id, communication) in BACKUP_COMMUNICATION.load().iter() {
                if communication.status.load().estimated_size.is_none() {
                    debug!("A running backup is lacking size estimate");
                    if let Some(config) = SETTINGS.load().backups.get(config_id) {
                        borg::reestimate_size(config, communication.clone());
                    }
                }
            }
        });
    } else {
        debug!("Not displaying ui because it is already visible.")
    }
}

fn on_delete() -> Inhibit {
    debug!("Potential quit: ApplicationWindow delete event");

    Handler::run(super::quit());
    Inhibit(true)
}