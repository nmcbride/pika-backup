use adw::prelude::*;

use crate::config;
use crate::ui;
use crate::ui::builder::DialogExcludePattern;
use crate::ui::prelude::*;

pub fn show() {
    let ui = DialogExcludePattern::new();
    let dialog = ui.dialog();

    dialog.set_transient_for(Some(&main_ui().window()));
    ui.add()
        .connect_clicked(clone!(@weak ui => move |_| Handler::run(clicked(ui))));

    // ensure lifetime until window closes
    let mutex = std::sync::Mutex::new(Some(ui.clone()));
    ui.dialog().connect_close_request(move |_| {
        *mutex.lock().unwrap() = None;
        gtk::Inhibit(false)
    });

    dialog.show();
}

async fn clicked(ui: DialogExcludePattern) -> Result<()> {
    let selected = ui.pattern_type().selected();
    let pattern = ui.pattern().text();

    let exclude = config::Exclude::from_pattern(match selected {
        0 => Ok(config::Pattern::Fnmatch(pattern.to_string().into())),
        1 => config::Pattern::from_regular_expression(pattern)
            .err_to_msg(gettext("Invalid Regular Expression")),
        // Not translated because this should not happen
        _ => Err(Message::short("No valid pattern type selected").into()),
    }?);

    BACKUP_CONFIG.update_result(move |config| {
        let active = config.active_mut()?;

        active.exclude.insert(exclude.clone());

        Ok(())
    })?;

    ui.dialog().destroy();
    ui::page_backup::refresh()?;

    Ok(())
}