use adw::prelude::*;

use super::event;
use super::frequency::{self, FrequencyObject};
use super::prune_preset::PrunePresetObject;
use super::weekday::{self, WeekdayObject};
use crate::ui::prelude::*;

use once_cell::sync::Lazy;

pub(super) static SCHEDULE_ACTIVE_SIGNAL_HANDLER: Lazy<glib::SignalHandlerId> =
    Lazy::new(init_schedule_active);

pub fn init() {
    // frequency model

    let model = gio::ListStore::new(FrequencyObject::static_type());

    for frequency in frequency::list() {
        model.append(&FrequencyObject::new(frequency));
    }

    main_ui().schedule_frequency().set_model(Some(&model));

    // weekday model

    let model = gio::ListStore::new(WeekdayObject::static_type());

    for weekday in &weekday::LIST {
        model.append(&WeekdayObject::new(*weekday));
    }

    main_ui().preferred_weekday_row().set_model(Some(&model));

    // events

    main_ui()
        .leaflet()
        .connect_visible_child_notify(|_| Handler::run(event::show_page()));

    main_ui()
        .detail_stack()
        .connect_visible_child_notify(|_| Handler::run(event::show_page()));

    Lazy::force(&SCHEDULE_ACTIVE_SIGNAL_HANDLER);

    main_ui()
        .schedule_frequency()
        .connect_selected_item_notify(|_| Handler::run(event::frequency_change()));

    main_ui()
        .schedule_preferred_hour()
        .connect_output(event::preferred_time_change);

    main_ui()
        .schedule_preferred_minute()
        .connect_output(event::preferred_time_change);

    main_ui()
        .schedule_preferred_time_popover()
        .connect_closed(|_| Handler::run(event::preferred_time_close()));

    main_ui()
        .preferred_weekday_row()
        .connect_selected_item_notify(|_| Handler::run(event::preferred_weekday_change()));

    main_ui()
        .schedule_preferred_day_calendar()
        .connect_day_selected(|_| Handler::run(event::preferred_day_change()));

    // prune

    main_ui()
        .prune_preset()
        .set_model(Some(&PrunePresetObject::list_store()));

    main_ui()
        .prune_enabled()
        .connect_active_notify(|_| Handler::run(event::prune_enabled()));

    main_ui()
        .schedule_keep_hourly()
        .connect_value_changed(|_| Handler::run(event::keep_change()));

    main_ui()
        .schedule_keep_daily()
        .connect_value_changed(|_| Handler::run(event::keep_change()));

    main_ui()
        .schedule_keep_weekly()
        .connect_value_changed(|_| Handler::run(event::keep_change()));

    main_ui()
        .schedule_keep_monthly()
        .connect_value_changed(|_| Handler::run(event::keep_change()));

    main_ui()
        .schedule_keep_yearly()
        .connect_value_changed(|_| Handler::run(event::keep_change()));

    // Network

    gio::NetworkMonitor::default()
        .connect_network_metered_notify(|_| Handler::run(event::network_changed()));

    gio::NetworkMonitor::default()
        .connect_network_available_notify(|_| Handler::run(event::network_changed()));

    // Daemon status

    Handler::run(async {
        crate::utils::listen_remote_app_running(&crate::daemon_app_id(), |running| {
            DAEMON_RUNNING.store(Arc::new(running));
            Handler::run(event::network_changed());
        })
        .await
        .err_to_msg(gettext("Unable to determine background process status"))
    });
}

fn init_schedule_active() -> glib::SignalHandlerId {
    main_ui()
        .schedule_active()
        .connect_enable_expansion_notify(|_| Handler::run(event::active_change()))
}
