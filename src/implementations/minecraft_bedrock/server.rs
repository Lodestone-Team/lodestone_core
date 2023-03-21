use std::collections::BTreeMap;
use std::env;
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use color_eyre::eyre::{eyre, Context};
use sysinfo::{Pid, PidExt, ProcessExt, SystemExt};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::error::{Error, ErrorKind};
use crate::events::{CausedBy, Event, EventInner, InstanceEvent, InstanceEventInner};
use crate::implementations::minecraft_java::player::MinecraftPlayer;
use crate::implementations::minecraft_java::util::{name_to_uuid, read_properties_from_path};
use crate::prelude::LODESTONE_PATH;
use crate::traits::t_configurable::TConfigurable;
use crate::traits::t_macro::TMacro;
use crate::traits::t_server::{MonitorReport, State, StateAction, TServer};

use crate::types::Snowflake;
use crate::util::dont_spawn_terminal;

use super::MinecraftBedrockInstance;
use tracing::{debug, error, info, warn};

#[async_trait]
impl TServer for MinecraftBedrockInstance {
    async fn start(&mut self, cause_by: CausedBy, block: bool) -> Result<(), Error> {
        let config = self.config.lock().await.clone();
        self.state.lock().await.try_transition(
            StateAction::UserStart,
            Some(&|state| {
                self.event_broadcaster.send(Event {
                    event_inner: EventInner::InstanceEvent(InstanceEvent {
                        instance_name: config.name.clone(),
                        instance_uuid: self.uuid.clone(),
                        instance_event_inner: InstanceEventInner::StateTransition { to: state },
                    }),
                    snowflake: Snowflake::default(),
                    details: "Starting server".to_string(),
                    caused_by: cause_by.clone(),
                });
            }),
        )?;

        if !port_scanner::local_port_available(config.port as u16) {
            return Err(Error {
                kind: ErrorKind::Internal,
                source: eyre!("Port {} is already in use", config.port),
            });
        }

        env::set_current_dir(&self.path_to_instance).context(
            "Failed to set current directory to the instance's path, is the path valid?",
        )?;

        // skip prelaunch part

        // write server_settings to server.properties
        
        let mut server_start_command = Command::new(self
            .path_to_instance
            .join("bedrock_server"));

        match dont_spawn_terminal(&mut server_start_command)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(mut proc) => {
                let stdin = proc.stdin.take().ok_or_else(|| {
                    error!(
                        "[{}] Failed to take stdin during startup",
                        config.name.clone()
                    );
                    eyre!("Failed to take stdin during startup")
                })?;
                self.stdin.lock().await.replace(stdin);
                let stdout = proc.stdout.take().ok_or_else(|| {
                    error!(
                        "[{}] Failed to take stdout during startup",
                        config.name.clone()
                    );
                    eyre!("Failed to take stdout during startup")
                })?;
                let stderr = proc.stderr.take().ok_or_else(|| {
                    error!(
                        "[{}] Failed to take stderr during startup",
                        config.name.clone()
                    );
                    eyre!("Failed to take stderr during startup")
                })?;
                *self.process.lock().await = Some(proc);

            }
            Err(e) => {}
        }

        Ok(())
    }

    async fn stop(&mut self, cause_by: CausedBy, block: bool) -> Result<(), Error> {
        Ok(())
    }

    async fn restart(&mut self, caused_by: CausedBy, block: bool) -> Result<(), Error> {
        Ok(())
    }

    async fn kill(&mut self, _cause_by: CausedBy) -> Result<(), Error> {
        Ok(())
    }

    async fn state(&self) -> State {
        *self.state.lock().await
    }

    async fn send_command(&self, command: &str, cause_by: CausedBy) -> Result<(), Error> {
        Ok(())
    }
    async fn monitor(&self) -> MonitorReport {
        MonitorReport::default()
    }
}
