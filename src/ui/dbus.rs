use crate::ui::prelude::*;
use async_std::prelude::*;

use crate::{schedule, ui};
use async_std::channel::Sender;

struct PikaBackup {
    command: Sender<Command>,
}

#[derive(Debug)]
enum Command {
    StartBackup(ConfigId, Option<schedule::DueCause>),
    ShowOverview,
    ShowSchedule(ConfigId),
}

#[zbus::dbus_interface(name = "org.gnome.World.PikaBackup1")]
impl PikaBackup {
    async fn start_scheduled_backup(&self, config_id: ConfigId, due_cause: schedule::DueCause) {
        info!(
            "Request to start scheduled backup {:?} {:?}",
            config_id, due_cause
        );
        if let Err(err) = self
            .command
            .send(Command::StartBackup(config_id, Some(due_cause)))
            .await
        {
            error!("{}", err);
        } else {
            debug!("Command to start scheduled backup sent.")
        }
    }

    async fn start_backup(&self, config_id: ConfigId) {
        info!("Request to start backup {:?}", config_id);
        if let Err(err) = self
            .command
            .send(Command::StartBackup(config_id, None))
            .await
        {
            error!("{}", err);
        }
    }

    async fn show_overview(&self) {
        info!("Request to show overview");
        if let Err(err) = self.command.send(Command::ShowOverview).await {
            error!("{}", err);
        }
    }

    async fn show_schedule(&self, config_id: ConfigId) {
        info!("Request to show schedule {:?}", config_id);
        if let Err(err) = self.command.send(Command::ShowSchedule(config_id)).await {
            error!("{}", err);
        }
    }
}

pub async fn init() {
    Handler::handle(
        session_connection()
            .await
            .map(|_| ())
            .err_to_msg(gettext("Failed to spawn interface for scheduled backups.")),
    );
}

async fn spawn_command_listener() -> Sender<Command> {
    let (sender, mut receiver) = async_std::channel::unbounded();

    Handler::run(async move {
        debug!("Internally awaiting D-Bus API commands");
        while let Some(command) = receiver.next().await {
            debug!("Received D-Bus API command {command:?}");
            match command {
                Command::StartBackup(config_id, due_cause) => {
                    // Prevent app from closing
                    let guard = QuitGuard::default();
                    // Start backup
                    ui::page_backup::start_backup(config_id, due_cause, guard);
                }
                Command::ShowOverview => ui::page_overview::dbus_show(),
                Command::ShowSchedule(backup_id) => ui::page_schedule::dbus_show(backup_id),
            }
        }

        Ok(())
    });

    sender
}

/// Session Bus
pub async fn session_connection() -> zbus::Result<zbus::Connection> {
    static CONNECTION: async_lock::Mutex<Option<zbus::Connection>> = async_lock::Mutex::new(None);

    let mut connection = CONNECTION.lock().await;

    if let Some(connection) = &*connection {
        Ok(connection.clone())
    } else {
        let command = spawn_command_listener().await;
        let new_connection = zbus::ConnectionBuilder::session()?
            .name(crate::DBUS_API_NAME)?
            .serve_at(crate::DBUS_API_PATH, PikaBackup { command })?
            .build()
            .await?;
        debug!("D-Bus listening on {}", crate::DBUS_API_NAME);

        *connection = Some(new_connection.clone());
        Ok(new_connection)
    }
}
