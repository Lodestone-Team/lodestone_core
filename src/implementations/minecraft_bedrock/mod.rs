pub mod configurable;
pub mod r#macro;
pub mod player;
pub mod resource;
pub mod server;

use async_trait::async_trait;
use color_eyre::eyre::{eyre, Context, ContextCompat};
use enum_kinds::EnumKind;
use std::collections::BTreeMap;
use std::process::Stdio;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::SystemExt;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};

use tokio::sync::Mutex;

use ::serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use tokio::sync::broadcast::Sender;
use tracing::{debug, error, info};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::{self};
use ts_rs::TS;

use crate::error::{Error, ErrorKind};
use crate::events::{CausedBy, Event, EventInner, ProgressionEvent, ProgressionEventInner};
use crate::macro_executor::MacroExecutor;
use crate::prelude::PATH_TO_BINARIES;
use crate::traits::t_configurable::{PathBuf, TConfigurable, Game};

use crate::traits::t_configurable::manifest::{
    ConfigurableManifest, ConfigurableValue, ConfigurableValueType, ManifestValue, SectionManifest,
    SectionManifestValue, SettingManifest,
};

use crate::traits::t_macro::TMacro;
use crate::traits::t_player::TPlayerManagement;
use crate::traits::t_resource::TResourceManagement;
use crate::traits::t_server::{State, TServer, MonitorReport};
use crate::traits::TInstance;
use crate::types::{DotLodestoneConfig, InstanceUuid, Snowflake};
use crate::util::{
    dont_spawn_terminal, download_file, format_byte, format_byte_download, unzip_file,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetupConfig {
    pub name: String,
    pub version: String,
    pub port: u32,
    pub server_args: Vec<String>,
    pub description: Option<String>,
    pub auto_start: Option<bool>,
    pub restart_on_crash: Option<bool>,
    pub start_on_connection: Option<bool>,
    pub backup_period: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RestoreConfig {
    pub name: String,
    pub version: String,
    pub description: String,
    pub server_args: Vec<String>,
    pub port: u32,
    pub auto_start: bool,
    pub restart_on_crash: bool,
    pub backup_period: Option<u32>,
    pub has_started: bool,
}

#[derive(Clone)]
pub struct MinecraftBedrockInstance {
    config: RestoreConfig,
    uuid: InstanceUuid,
    creation_time: i64,
    state: Arc<Mutex<State>>,
    event_broadcaster: Sender<Event>,

    // file paths
    path_to_instance: PathBuf,
    path_to_config: PathBuf,
    path_to_properties: PathBuf,

    // directory paths
    path_to_macros: PathBuf,
    path_to_resources: PathBuf,
    path_to_runtimes: PathBuf,

    // variables which can be changed at runtime
    auto_start: Arc<AtomicBool>,
    restart_on_crash: Arc<AtomicBool>,
    backup_period: Option<u32>,
    process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    system: Arc<Mutex<sysinfo::System>>,
    // players_manager: Arc<Mutex<PlayersManager>>,
    pub(super) server_properties_buffer: Arc<Mutex<BTreeMap<String, String>>>,
    configurable_manifest: Arc<Mutex<ConfigurableManifest>>,
    macro_executor: MacroExecutor,
    // backup_sender: UnboundedSender<BackupInstruction>,
    rcon_conn: Arc<Mutex<Option<rcon::Connection<tokio::net::TcpStream>>>>,
}

impl MinecraftBedrockInstance {
    async fn write_config_to_file(&self) -> Result<(), Error> {
        tokio::fs::write(
            &self.path_to_config,
            to_string_pretty(&self.config)
                .context("Failed to serialize config to string, this is a bug, please report it")?,
        )
        .await
        .context(format!(
            "Failed to write config to file at {}",
            &self.path_to_config.display()
        ))?;
        Ok(())
    }
}

impl TInstance for MinecraftBedrockInstance {}