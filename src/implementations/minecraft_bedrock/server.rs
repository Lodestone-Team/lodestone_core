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
