pub mod borg;
pub mod df;
pub mod duration;
pub mod ext;
pub mod repo_cache;
pub mod secret_service;

use adw::prelude::*;

use crate::config;
use crate::ui::prelude::*;

use ashpd::desktop::background;

use std::io::Read;

pub struct StatusRow {
    pub title: String,
    pub subtitle: String,
    pub icon_name: String,
    pub level: StatusLevel,
}

#[derive(Clone, Copy)]
pub enum StatusLevel {
    Ok,
    Neutral,
    Warning,
    Error,
}

impl StatusRow {
    pub fn action_row(&self) -> adw::ActionRow {
        let icon = super::widgets::StatusIcon::new(&self.icon_name, self.level);

        let row = adw::ActionRow::builder()
            .title(&self.title)
            .subtitle(&self.subtitle)
            .build();

        row.add_prefix(&icon);

        row
    }
}

/// Checks if a directory is most likely a borg repository. Performed checks are
///
/// - `data/` exists and is a directory
/// - `config` exists and contains the string `[repository]`
pub fn is_backup_repo(path: &std::path::Path) -> bool {
    trace!("Checking path if it is a repo '{}'", &path.display());
    if let Ok(data) = std::fs::File::open(path.join("data")).and_then(|x| x.metadata()) {
        if data.is_dir() {
            if let Ok(mut cfg) = std::fs::File::open(path.join("config")) {
                if let Ok(metadata) = cfg.metadata() {
                    if metadata.len() < 1024 * 1024 {
                        let mut content = String::new();
                        #[allow(unused_must_use)]
                        {
                            cfg.read_to_string(&mut content);
                        }
                        return content.contains("[repository]");
                    }
                }
            }
        }
    };

    false
}

pub fn cache_dir() -> std::path::PathBuf {
    [glib::user_cache_dir(), env!("CARGO_PKG_NAME").into()]
        .iter()
        .collect()
}

pub async fn background_permission() -> std::result::Result<bool, ashpd::Error> {
    let response = background::request(
        &ashpd::WindowIdentifier::default(),
        &gettext("Schedule and continue running backups in background."),
        false,
        None::<&[&str]>,
        true,
    )
    .await;

    match response {
        Err(ashpd::Error::Portal(ashpd::PortalError::Cancelled(_))) => Ok(false),
        x => Ok(x?.run_in_background()),
    }
}

pub trait LookupActiveConfigId<T> {
    #[doc(alias = "get_active")]
    fn active(&self) -> Result<&T>;
    #[doc(alias = "get_active_mut")]
    fn active_mut(&mut self) -> Result<&mut T>;
}

#[allow(clippy::implicit_hasher)]
impl<T> LookupActiveConfigId<T> for std::collections::BTreeMap<ConfigId, T> {
    fn active(&self) -> Result<&T> {
        Ok(self.get_result(&active_config_id_result()?)?)
    }

    fn active_mut(&mut self) -> Result<&mut T> {
        Ok(self.get_result_mut(&active_config_id_result()?)?)
    }
}
impl LookupActiveConfigId<config::Backup> for config::Backups {
    fn active(&self) -> Result<&config::Backup> {
        Ok(self.get_result(&active_config_id_result()?)?)
    }

    fn active_mut(&mut self) -> Result<&mut config::Backup> {
        Ok(self.get_result_mut(&active_config_id_result()?)?)
    }
}

fn active_config_id_result() -> Result<ConfigId> {
    ACTIVE_BACKUP_ID
        .get()
        .ok_or_else(|| Message::short("There is no active backup in the interface.").into())
}

pub async fn spawn_thread<P: core::fmt::Display, F, R>(
    name: P,
    task: F,
) -> std::result::Result<R, futures::channel::oneshot::Canceled>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (send, recv) = futures::channel::oneshot::channel();

    let sender = std::cell::Cell::new(Some(send));

    let task_name = name.to_string();
    let result = std::thread::Builder::new()
        .name(task_name.to_string())
        .spawn(move || {
            if let Some(sender) = sender.replace(None) {
                if sender.send(task()).is_err() {
                    error!(
                        "spawn_thread({}): Error sending to handler: Receiving end hung up",
                        task_name
                    );
                }
            } else {
                error!(
                    "spawn_thread({}): Error sending to handler: Already send",
                    task_name
                );
            }
        });

    if let Err(err) = result {
        error!("Failed to create thread for '{}': {}", name, err);
    }

    recv.await
}

quick_error! {
    #[derive(Debug)]
    pub enum AsyncErr {
        ThreadPanicked { display("{}", gettext("The operation terminated unexpectedly.")) }
    }
}

pub fn folder_chooser<T: IsA<gtk::Window>>(title: &str, parent: &T) -> gtk::FileChooserNative {
    gtk::FileChooserNative::builder()
        .action(gtk::FileChooserAction::SelectFolder)
        .title(title)
        .accept_label(&gettext("Select"))
        .modal(true)
        .transient_for(parent)
        .build()
}

pub async fn folder_chooser_dialog(title: &str) -> Option<gio::File> {
    let dialog = folder_chooser(title, &main_ui().window());

    let result = if dialog.run_future().await == gtk::ResponseType::Accept {
        dialog.file()
    } else {
        None
    };

    result
}

pub async fn folder_chooser_dialog_path(title: &str) -> Option<std::path::PathBuf> {
    folder_chooser_dialog(title).await.and_then(|x| x.path())
}

pub fn dialog_catch_err<X, P: std::fmt::Display, S: std::fmt::Display>(
    res: std::result::Result<X, P>,
    msg: S,
) -> bool {
    match res {
        Err(e) => {
            show_error(msg, e);
            true
        }
        Ok(_) => false,
    }
}

fn ellipsize<S: std::fmt::Display>(x: S) -> String {
    let s = x.to_string();
    let vec = s.chars().collect::<Vec<_>>();

    if vec.len() > 410 {
        format!(
            "{}\n…\n{}",
            vec.iter().take(200).collect::<String>(),
            vec.iter().rev().take(200).rev().collect::<String>()
        )
    } else {
        s
    }
}

pub fn show_error<S: std::fmt::Display, P: std::fmt::Display>(message: S, detail: P) {
    show_error_transient_for(message, detail, &main_ui().window());
}

pub fn show_notice<S: std::fmt::Display>(message: S) {
    warn!("Displaying notice:\n  {}", &message);
    main_ui()
        .internal_message_text()
        .set_text(&message.to_string());
    main_ui().internal_message().set_revealed(true);
}

pub fn show_error_transient_for<S: std::fmt::Display, P: std::fmt::Display, W: IsA<gtk::Window>>(
    message: S,
    detail: P,
    window: &W,
) {
    let primary_text = ellipsize(message);
    let secondary_text = ellipsize(detail);
    warn!(
        "Displaying error:\n  {}\n  {}",
        &primary_text, &secondary_text
    );

    if crate::ui::app_window::is_displayed() {
        let dialog = gtk::MessageDialog::builder()
            .modal(true)
            .transient_for(window)
            .message_type(gtk::MessageType::Error)
            .buttons(gtk::ButtonsType::Close)
            .text(&primary_text)
            .build();

        dialog.set_secondary_text(if secondary_text.is_empty() {
            None
        } else {
            Some(&secondary_text)
        });

        dialog.connect_response(|dialog, _| {
            dialog.close();
            dialog.hide();
        });

        dialog.show();
    } else {
        let notification = gio::Notification::new(&primary_text);
        notification.set_body(if secondary_text.is_empty() {
            None
        } else {
            Some(&secondary_text)
        });
        gtk_app().send_notification(None, &notification);
    }
}

pub async fn confirmation_dialog(
    title: &str,
    message: &str,
    cancel: &str,
    accept: &str,
) -> std::result::Result<(), UserCanceled> {
    ConfirmationDialog::new(title, message, cancel, accept)
        .ask()
        .await
}

pub struct ConfirmationDialog {
    title: String,
    message: String,
    cancel: String,
    accept: String,
    destructive: bool,
}

impl ConfirmationDialog {
    pub fn new(title: &str, message: &str, cancel: &str, accept: &str) -> Self {
        Self {
            title: title.to_string(),
            message: message.to_string(),
            cancel: cancel.to_string(),
            accept: accept.to_string(),
            destructive: false,
        }
    }

    pub fn set_destructive(&mut self, destructive: bool) -> &mut Self {
        self.destructive = destructive;
        self
    }

    pub async fn ask(&self) -> std::result::Result<(), UserCanceled> {
        let dialog = gtk::MessageDialog::builder()
            .transient_for(&main_ui().window())
            .modal(true)
            .message_type(gtk::MessageType::Question)
            .text(&self.title)
            .secondary_text(&self.message)
            .build();

        dialog.add_button(&self.cancel, gtk::ResponseType::Cancel);
        let accept_button = dialog.add_button(&self.accept, gtk::ResponseType::Accept);

        if self.destructive {
            accept_button.add_css_class("destructive-action");
        }

        let result = dialog.run_future().await;
        dialog.close();
        dialog.hide();

        if result == gtk::ResponseType::Accept {
            Ok(())
        } else {
            Err(UserCanceled::new())
        }
    }
}

pub fn clear(listbox: &gtk::ListBox) {
    let mut i = 0;
    while let Some(row) = listbox.row_at_index(i) {
        if !row.widget_name().starts_with('-') {
            listbox.remove(&row);
        } else {
            i += 1;
        }
    }
}

pub fn file_icon(path: &std::path::Path) -> Option<gtk::Image> {
    let file = gio::File::for_path(path);
    let info = file.query_info(
        "*",
        gio::FileQueryInfoFlags::NONE,
        gtk::gio::Cancellable::NONE,
    );

    let mut gicon = None;
    if let Ok(info) = info {
        if let Some(icon) = info.icon() {
            gicon = Some(icon.clone());

            if let Some(theme) =
                gtk::gdk::Display::default().and_then(|d| gtk::IconTheme::for_display(&d))
            {
                if !theme.has_gicon(&icon) {
                    gicon = gio::Icon::for_string("system-run").ok();
                }
            }
        }
    }

    if gicon.is_none() {
        gicon = gio::Icon::for_string("folder-visiting").ok();
    }

    gicon.map(|x| {
        gtk::Image::builder()
            .gicon(&x)
            .css_classes(vec![String::from("row-icon")])
            .build()
    })
}

pub fn file_symbolic_icon(path: &std::path::Path) -> Option<gtk::Image> {
    let file = gio::File::for_path(path);
    let info = file.query_info(
        "*",
        gio::FileQueryInfoFlags::NONE,
        gtk::gio::Cancellable::NONE,
    );
    if let Ok(info) = info {
        let icon = info.symbolic_icon();
        icon.map(|icon| gtk::Image::from_gicon(&icon))
    } else {
        None
    }
}

pub fn new_action_row_with_gicon(icon: Option<&gio::Icon>) -> adw::ActionRow {
    let row = adw::ActionRow::builder().activatable(true).build();

    if let Some(gicon) = icon {
        row.add_prefix(
            &gtk::Image::builder()
                .gicon(gicon)
                .css_classes(vec![String::from("row-icon")])
                .build(),
        );
    }

    row
}
