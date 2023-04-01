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
use crate::prelude::LODESTONE_PATH;
use crate::traits::t_configurable::TConfigurable;
use crate::traits::t_macro::TMacro;
use crate::traits::t_server::{MonitorReport, State, StateAction, TServer};

use crate::types::Snowflake;
use crate::util::dont_spawn_terminal;

use super::MinecraftBedrockInstance;
use super::player::MinecraftBedrockPlayer;
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
                tokio::task::spawn({
                    use fancy_regex::Regex;
                    use lazy_static::lazy_static;

                    let event_broadcaster = self.event_broadcaster.clone();
                    let uuid = self.uuid.clone();
                    let name = config.name.clone();
                    let players_manager = self.players_manager.clone();
                    // let macro_executor = self.macro_executor.clone();
                    let mut __self = self.clone();
                    async move {
                        fn parse_system_msg(msg: &str) -> Option<String> {
                            lazy_static! {
                                static ref RE: Regex = Regex::new(r"\[(.*)\]\s(.*)").unwrap();
                            }
                            if RE.is_match(msg).ok()? {
                                RE.captures(msg)
                                    .ok()?
                                    .map(|caps| caps.get(2).unwrap().as_str().to_string())
                            } else {
                                None
                            }
                        }
                        fn parse_player_joined(system_msg: &str) -> Option<(String, String)> {
                            lazy_static! {
                                static ref RE: Regex = Regex::new(r"Player connected:\s*(\w+),\s*xuid:\s*(\d+)").unwrap();
                            }
                            if RE.is_match(system_msg).unwrap() {
                                if let Some(cap) = RE.captures(system_msg).ok()? {
                                    Some((
                                        cap.get(1)?.as_str().to_string(),
                                        cap.get(2)?.as_str().to_string(),
                                    ))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }

                        fn parse_player_left(system_msg: &str) -> Option<String> {
                            lazy_static! {
                                static ref RE: Regex = Regex::new(r"(?<=Player disconnected: )\w+").unwrap();
                            }
                            if RE.is_match(system_msg).unwrap() {
                                if let Some(cap) = RE.captures(system_msg).ok()? {
                                    Some(cap.get(1)?.as_str().to_string())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }

                        fn parse_server_started(system_msg: &str) -> bool {
                            lazy_static! {
                                static ref RE: Regex = Regex::new(r"Server started.").unwrap();
                            }
                            RE.is_match(system_msg).unwrap()
                        }

                        let mut did_start = false;

                        let mut stdout_lines = BufReader::new(stdout).lines();
                        let mut stderr_lines = BufReader::new(stderr).lines();

                        while let (Ok(Some(line)), is_stdout) = tokio::select!(
                            line = stdout_lines.next_line() => {
                                (line, true)
                            }
                            line = stderr_lines.next_line() => {
                                (line, false)
                            }
                        ) {
                            if is_stdout {
                                // info!("[{}] {}", name, line);
                            } else {
                                warn!("[{}] {}", name, line);
                            }
                            let _ = event_broadcaster.send(Event {
                                event_inner: EventInner::InstanceEvent(InstanceEvent {
                                    instance_uuid: uuid.clone(),
                                    instance_event_inner: InstanceEventInner::InstanceOutput {
                                        message: line.clone(),
                                    },
                                    instance_name: name.clone(),
                                }),
                                details: "".to_string(),
                                snowflake: Snowflake::default(),
                                caused_by: CausedBy::System,
                            });

                            if parse_server_started(&line) && !did_start {
                                did_start = true;
                                self.state
                                    .lock()
                                    .await
                                    .try_transition(
                                        StateAction::InstanceStart,
                                        Some(&|state| {
                                            self.event_broadcaster.send(Event {
                                                event_inner: EventInner::InstanceEvent(
                                                    InstanceEvent {
                                                        instance_name: config.name.clone(),
                                                        instance_uuid: self.uuid.clone(),
                                                        instance_event_inner:
                                                            InstanceEventInner::StateTransition {
                                                                to: state,
                                                            },
                                                    },
                                                ),
                                                snowflake: Snowflake::default(),
                                                details: "Starting server".to_string(),
                                                caused_by: cause_by.clone(),
                                            });
                                        }),
                                    )
                                    .unwrap();

                                let _ = self.read_properties().await.map_err(|e| {
                                    error!("Failed to read properties: {}", e);
                                    e
                                });
                            }
                            if let Some(system_msg) = parse_system_msg(&line) {
                                let _ = event_broadcaster.send(Event {
                                    event_inner: EventInner::InstanceEvent(InstanceEvent {
                                        instance_uuid: uuid.clone(),
                                        instance_event_inner: InstanceEventInner::SystemMessage {
                                            message: line,
                                        },
                                        instance_name: name.clone(),
                                    }),
                                    details: "".to_string(),
                                    snowflake: Snowflake::default(),
                                    caused_by: CausedBy::System,
                                });
                                if let Some((player_name, xuid)) = parse_player_joined(&system_msg) {
                                    players_manager.lock().await.add_player(
                                        MinecraftBedrockPlayer {
                                            name: player_name.clone(),
                                            uuid: Some(xuid.clone()),
                                        },
                                        self.name().await,
                                    );
                                } else if let Some(player_name) = parse_player_left(&system_msg) {
                                    players_manager
                                        .lock()
                                        .await
                                        .remove_by_name(&player_name, self.name().await);
                                }
                            }
                        }
                        info!("Instance {} process shutdown", name);
                        self.state
                            .lock()
                            .await
                            .try_transition(
                                StateAction::InstanceStop,
                                Some(&|state| {
                                    self.event_broadcaster.send(Event {
                                        event_inner: EventInner::InstanceEvent(InstanceEvent {
                                            instance_name: config.name.clone(),
                                            instance_uuid: self.uuid.clone(),
                                            instance_event_inner:
                                                InstanceEventInner::StateTransition { to: state },
                                        }),
                                        snowflake: Snowflake::default(),
                                        details: "Instance stopping as server process exited"
                                            .to_string(),
                                        caused_by: cause_by.clone(),
                                    });
                                }),
                            )
                            .unwrap();
                        self.players_manager.lock().await.clear(name);
                    }
                });

                self.config.lock().await.has_started = true;
                self.write_config_to_file().await?;
                let instance_uuid = self.uuid.clone();
                let mut rx = self.event_broadcaster.subscribe();

                if block {
                    while let Ok(event) = rx.recv().await {
                        if let EventInner::InstanceEvent(InstanceEvent {
                            instance_uuid: event_instance_uuid,
                            instance_event_inner: InstanceEventInner::StateTransition { to },
                            ..
                        }) = event.event_inner
                        {
                            if instance_uuid == event_instance_uuid {
                                if to == State::Running {
                                    return Ok(()); // Instance started successfully
                                } else if to == State::Stopped {
                                    return Err(eyre!(
                                        "Instance exited unexpectedly before starting"
                                    )
                                    .into());
                                }
                            }
                        }
                    }
                    Err(eyre!("Sender shutdown").into())
                } else {
                    Ok(())
                }
            }
            Err(e) => {
                error!("Failed to start server, {}", e);
                self.state
                    .lock()
                    .await
                    .try_transition(
                        StateAction::InstanceStop,
                        Some(&|state| {
                            self.event_broadcaster.send(Event {
                                event_inner: EventInner::InstanceEvent(InstanceEvent {
                                    instance_name: config.name.clone(),
                                    instance_uuid: self.uuid.clone(),
                                    instance_event_inner: InstanceEventInner::StateTransition {
                                        to: state,
                                    },
                                }),
                                snowflake: Snowflake::default(),
                                details: "Starting server".to_string(),
                                caused_by: cause_by.clone(),
                            });
                        }),
                    )
                    .unwrap();
                Err(e).context("Failed to start server")?;
                unreachable!();
            }
        }
    }

    async fn stop(&mut self, cause_by: CausedBy, block: bool) -> Result<(), Error> {
        let config = self.config.lock().await.clone();

        self.state.lock().await.try_transition(
            StateAction::UserStop,
            Some(&|state| {
                self.event_broadcaster.send(Event {
                    event_inner: EventInner::InstanceEvent(InstanceEvent {
                        instance_name: config.name.clone(),
                        instance_uuid: self.uuid.clone(),
                        instance_event_inner: InstanceEventInner::StateTransition { to: state },
                    }),
                    snowflake: Snowflake::default(),
                    details: "Stopping server".to_string(),
                    caused_by: cause_by.clone(),
                });
            }),
        )?;
        let name = config.name.clone();
        let _uuid = self.uuid.clone();
        self.stdin
            .lock()
            .await
            .as_mut()
            .ok_or_else(|| {
                error!("[{}] Failed to stop instance: stdin not available", name);
                eyre!("Failed to stop instance: stdin not available")
            })?
            .write_all(b"stop\n")
            .await
            .context("Failed to write to stdin")
            .map_err(|e| {
                error!("[{}] Failed to stop instance: {}", name, e);
                e
            })?;
        let mut rx = self.event_broadcaster.subscribe();
        let instance_uuid = self.uuid.clone();

        if block {
            while let Ok(event) = rx.recv().await {
                if let EventInner::InstanceEvent(InstanceEvent {
                    instance_uuid: event_instance_uuid,
                    instance_event_inner: InstanceEventInner::StateTransition { to },
                    ..
                }) = event.event_inner
                {
                    if instance_uuid == event_instance_uuid && to == State::Stopped {
                        return Ok(());
                    }
                }
            }
            Err(eyre!("Sender shutdown").into())
        } else {
            Ok(())
        }
    }

    async fn restart(&mut self, caused_by: CausedBy, block: bool) -> Result<(), Error> {
        if block {
            self.stop(caused_by.clone(), block).await?;
            self.start(caused_by, block).await
        } else {
            self.state
                .lock()
                .await
                .try_new_state(StateAction::UserStop, None)?;

            let mut __self = self.clone();
            tokio::task::spawn(async move {
                self.stop(caused_by.clone(), true).await.unwrap();
                self.start(caused_by, block).await.unwrap()
            });
            Ok(())
        }
    }

    async fn kill(&mut self, _cause_by: CausedBy) -> Result<(), Error> {
        let config = self.config.lock().await.clone();

        if self.state().await == State::Stopped {
            warn!("[{}] Instance is already stopped", config.name.clone());
            return Err(eyre!("Instance is already stopped").into());
        }
        self.process
            .lock()
            .await
            .as_mut()
            .ok_or_else(|| {
                error!(
                    "[{}] Failed to kill instance: process not available",
                    config.name.clone()
                );
                eyre!("Failed to kill instance: process not available")
            })?
            .kill()
            .await
            .context("Failed to kill process")
            .map_err(|e| {
                error!("[{}] Failed to kill instance: {}", config.name.clone(), e);
                e
            })?;
        Ok(())
    }

    async fn state(&self) -> State {
        *self.state.lock().await
    }

    async fn send_command(&self, command: &str, cause_by: CausedBy) -> Result<(), Error> {
        let config = self.config.lock().await.clone();
        if self.state().await == State::Stopped {
            Err(eyre!("Instance is stopped").into())
        } else {
            match self.stdin.lock().await.as_mut() {
                Some(stdin) => match {
                    if command == "stop" {
                        self.state.lock().await.try_new_state(
                            StateAction::UserStop,
                            Some(&|state| {
                                self.event_broadcaster.send(Event {
                                    event_inner: EventInner::InstanceEvent(InstanceEvent {
                                        instance_name: config.name.clone(),
                                        instance_uuid: self.uuid.clone(),
                                        instance_event_inner: InstanceEventInner::StateTransition {
                                            to: state,
                                        },
                                    }),
                                    snowflake: Snowflake::default(),
                                    details: "Starting server".to_string(),
                                    caused_by: cause_by.clone(),
                                });
                            }),
                        )?;
                    }
                    stdin.write_all(format!("{}\n", command).as_bytes()).await
                } {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        warn!(
                            "[{}] Failed to send command to instance: {}",
                            config.name.clone(),
                            e
                        );
                        Err(e).context("Failed to send command to instance")?;
                        unreachable!()
                    }
                },
                None => {
                    let err_msg =
                        "Failed to write to stdin because stdin is None. Please report this bug.";
                    error!("[{}] {}", config.name.clone(), err_msg);
                    Err(eyre!(err_msg).into())
                }
            }
        }
    }
    async fn monitor(&self) -> MonitorReport {
        let mut sys = self.system.lock().await;
        sys.refresh_memory();
        if let Some(pid) = self.process.lock().await.as_ref().and_then(|p| p.id()) {
            sys.refresh_process(Pid::from_u32(pid));
            let proc = (*sys).process(Pid::from_u32(pid));
            if let Some(proc) = proc {
                let cpu_usage =
                    sys.process(Pid::from_u32(pid)).unwrap().cpu_usage() / sys.cpus().len() as f32;

                let memory_usage = proc.memory();
                let disk_usage = proc.disk_usage();
                let start_time = proc.start_time();
                MonitorReport {
                    memory_usage: Some(memory_usage),
                    disk_usage: Some(disk_usage.into()),
                    cpu_usage: Some(cpu_usage),
                    start_time: Some(start_time),
                }
            } else {
                MonitorReport::default()
            }
        } else {
            MonitorReport::default()
        }
    }

}
