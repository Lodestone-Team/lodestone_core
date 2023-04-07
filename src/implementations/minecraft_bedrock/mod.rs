pub mod configurable;
pub mod r#macro;
pub mod player;
pub mod players_manager;
pub mod resource;
pub mod util;
pub mod server;

use crate::event_broadcaster::EventBroadcaster;
use crate::traits::t_configurable::GameType;

use std::collections::HashMap;
use async_trait::async_trait;
use color_eyre::eyre::{eyre, Context, ContextCompat};
use enum_kinds::EnumKind;
use indexmap::IndexMap;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::SystemExt;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};

use tokio::sync::{Mutex, broadcast};

use ::serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;
use tokio::sync::broadcast::Sender;
use tracing::{debug, error, info};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::{self};
use ts_rs::TS;

use crate::error::{Error, ErrorKind};
use crate::events::{CausedBy, Event, EventInner, ProgressionEvent, ProgressionEventInner};
use crate::macro_executor::{MacroExecutor, MacroPID};
use crate::prelude::PATH_TO_BINARIES;
use crate::traits::t_configurable::PathBuf;

use crate::traits::t_configurable::manifest::{
    ConfigurableManifest, ConfigurableValue, ConfigurableValueType, SectionManifest,
    SettingManifest, SetupManifest, SetupValue,
};

use self::util::{get_server_zip_url, get_minecraft_bedrock_version, read_properties_from_path};
use self::configurable::ServerPropertySetting;

use crate::traits::t_macro::TaskEntry;
use crate::traits::t_server::{State, TServer, MonitorReport};
use crate::traits::TInstance;
use crate::types::{DotLodestoneConfig, InstanceUuid, Snowflake};
use crate::util::{
    dont_spawn_terminal, download_file, format_byte, format_byte_download, unzip_file,
};

use self::players_manager::PlayersManager;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetupConfig {
    pub name: String,
    pub version: String,
    pub port: u32,
    pub description: Option<String>,
    pub auto_start: Option<bool>,
    pub restart_on_crash: Option<bool>,
    pub backup_period: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RestoreConfig {
    pub name: String,
    pub version: String,
    pub description: String,
    pub port: u32,
    pub auto_start: bool,
    pub restart_on_crash: bool,
    pub backup_period: Option<u32>,
    pub has_started: bool,
}

#[derive(Clone)]
pub struct MinecraftBedrockInstance {
    config: Arc<Mutex<RestoreConfig>>,
    uuid: InstanceUuid,
    creation_time: i64,
    state: Arc<Mutex<State>>,
    event_broadcaster: EventBroadcaster,

    // file paths
    path_to_instance: PathBuf,
    path_to_config: PathBuf,
    path_to_properties: PathBuf,

    // directory paths
    path_to_macros: PathBuf,
    path_to_worlds: PathBuf,

    // variables which can be changed at runtime
    auto_start: Arc<AtomicBool>,
    restart_on_crash: Arc<AtomicBool>,
    backup_period: Option<u32>,
    process: Arc<Mutex<Option<Child>>>,
    stdin: Arc<Mutex<Option<tokio::process::ChildStdin>>>,
    system: Arc<Mutex<sysinfo::System>>,
    players_manager: Arc<Mutex<PlayersManager>>,
    configurable_manifest: Arc<Mutex<ConfigurableManifest>>,
    macro_executor: MacroExecutor,
    backup_sender: UnboundedSender<BackupInstruction>,
    macro_name_to_last_run: Arc<Mutex<HashMap<String, i64>>>,
    pid_to_task_entry: Arc<Mutex<IndexMap<MacroPID, TaskEntry>>>,
}


#[derive(Debug, Clone)]
enum BackupInstruction {
    SetPeriod(Option<u32>),
    BackupNow,
    Pause,
    Resume,
}

impl MinecraftBedrockInstance { 
    pub async fn setup_manifest() -> Result<SetupManifest, Error> {
        let version = get_minecraft_bedrock_version().await?;

        let name_setting = SettingManifest::new_required_value(
            "name".to_string(),
            "Server Name".to_string(),
            "The name of the server instance".to_string(),
            ConfigurableValue::String("Minecraft Server".to_string()),
            None,
            false,
            true,
        );

        let description_setting = SettingManifest::new_optional_value(
            "description".to_string(),
            "Description".to_string(),
            "A description of the server instance".to_string(),
            None,
            ConfigurableValueType::String { regex: None },
            None,
            false,
            true,
        );

        let version_setting = SettingManifest::new_required_value(
            "version".to_string(),
            "Version".to_string(),
            "The version of minecraft to use".to_string(),
            ConfigurableValue::String(version.clone()),
            None,
            false,
            true,
        );

        let port_setting = SettingManifest::new_value_with_type(
            "port".to_string(),
            "Port".to_string(),
            "The port to run the server on".to_string(),
            Some(ConfigurableValue::UnsignedInteger(25565)),
            ConfigurableValueType::UnsignedInteger {
                min: Some(0),
                max: Some(65535),
            },
            Some(ConfigurableValue::UnsignedInteger(25565)),
            false,
            true,
        );

        let min_ram_setting = SettingManifest::new_required_value(
            "min_ram".to_string(),
            "Minimum RAM".to_string(),
            "The minimum amount of RAM to allocate to the server".to_string(),
            ConfigurableValue::UnsignedInteger(1024),
            Some(ConfigurableValue::UnsignedInteger(1024)),
            false,
            true,
        );

        let max_ram_setting = SettingManifest::new_required_value(
            "max_ram".to_string(),
            "Maximum RAM".to_string(),
            "The maximum amount of RAM to allocate to the server".to_string(),
            ConfigurableValue::UnsignedInteger(2048),
            Some(ConfigurableValue::UnsignedInteger(2048)),
            false,
            true,
        );

        let command_line_args_setting = SettingManifest::new_optional_value(
            "cmd_args".to_string(),
            "Command Line Arguments".to_string(),
            "Command line arguments to pass to the server".to_string(),
            None,
            ConfigurableValueType::String { regex: None },
            None,
            false,
            true,
        );

        let mut section_1_map = IndexMap::new();
        section_1_map.insert("name".to_string(), name_setting);
        section_1_map.insert("description".to_string(), description_setting);

        section_1_map.insert("version".to_string(), version_setting);
        section_1_map.insert("port".to_string(), port_setting);

        let mut section_2_map = IndexMap::new();

        section_2_map.insert("min_ram".to_string(), min_ram_setting);

        section_2_map.insert("max_ram".to_string(), max_ram_setting);

        section_2_map.insert("cmd_args".to_string(), command_line_args_setting);

        let section_1 = SectionManifest::new(
            "section_1".to_string(),
            "Basic Settings".to_string(),
            "Basic settings for the server.".to_string(),
            section_1_map,
        );

        let section_2 = SectionManifest::new(
            "section_2".to_string(),
            "Advanced Settings".to_string(),
            "Advanced settings for your minecraft server.".to_string(),
            section_2_map,
        );

        let mut sections = IndexMap::new();

        sections.insert("section_1".to_string(), section_1);
        sections.insert("section_2".to_string(), section_2);

        Ok(SetupManifest {
            setting_sections: sections,
        })
    }

    pub async fn construct_setup_config(
        setup_value: SetupValue,
    ) -> Result<SetupConfig, Error> {
        Self::setup_manifest()
            .await?
            .validate_setup_value(&setup_value)?;

        // ALL of the following unwraps are safe because we just validated the manifest value
        let description = setup_value
            .get_unique_setting("description")
            .unwrap()
            .get_value()
            .map(|v| v.try_as_string().unwrap());

        let name = setup_value
            .get_unique_setting("name")
            .unwrap()
            .get_value()
            .unwrap()
            .try_as_string()
            .unwrap();

        let version = setup_value
            .get_unique_setting("version")
            .unwrap()
            .get_value()
            .unwrap()
            .try_as_enum()
            .unwrap();

        let port = setup_value
            .get_unique_setting("port")
            .unwrap()
            .get_value()
            .unwrap()
            .try_as_unsigned_integer()
            .unwrap();

        Ok(SetupConfig {
            name: name.clone(),
            description: description.cloned(),
            version: version.clone(),
            port,
            auto_start: Some(setup_value.auto_start),
            restart_on_crash: Some(setup_value.restart_on_crash),
            backup_period: None,
        })
    }

    fn init_configurable_manifest(
        restore_config: &RestoreConfig,
    ) -> ConfigurableManifest {
        let server_properties_section_manifest = SectionManifest::new(
            ServerPropertySetting::get_section_id().to_string(),
            "Server Properties Settings".to_string(),
            "All settings in the server.properties file can be configured here".to_string(),
            IndexMap::new(),
        );

        let mut setting_sections = IndexMap::new();

        setting_sections.insert(
            ServerPropertySetting::get_section_id().to_string(),
            server_properties_section_manifest,
        );

        ConfigurableManifest::new(false, false, setting_sections)
    }

    async fn write_config_to_file(&self) -> Result<(), Error> {
        tokio::fs::write(
            &self.path_to_config,
            to_string_pretty(&*self.config.lock().await)
                .context("Failed to serialize config to string, this is a bug, please report it")?,
        )
        .await
        .context(format!(
            "Failed to write config to file at {}",
            &self.path_to_config.display()
        ))?;
        Ok(())
    }

    async fn read_properties(&mut self) -> Result<(), Error> {
        let properties = read_properties_from_path(&self.path_to_properties).await?;
        for (key, value) in properties.iter() {
            self.configurable_manifest.lock().await.set_setting(
                ServerPropertySetting::get_section_id(),
                ServerPropertySetting::from_key_val(key, value)?.into(),
            )?;
        }
        Ok(())
    }

    async fn write_properties_to_file(&self) -> Result<(), Error> {
        // open the file in write-only mode, returns `io::Result<File>`
        let mut file = tokio::fs::File::create(&self.path_to_properties)
            .await
            .context(format!(
                "Failed to open properties file at {}",
                &self.path_to_properties.display()
            ))?;
        let mut setting_str = "".to_string();
        for (key, value) in self
            .configurable_manifest
            .lock()
            .await
            .get_section(ServerPropertySetting::get_section_id())
            .unwrap()
            .all_settings()
            .iter()
        {
            // print the key and value separated by a =
            // println!("{}={}", key, value);
            setting_str.push_str(&format!(
                "{}={}\n",
                key,
                value
                    .get_value()
                    .expect("Programming error, value is not set")
                    .to_string()
            ));
        }
        file.write_all(setting_str.as_bytes())
            .await
            .context(format!(
                "Failed to write properties to file at {}",
                &self.path_to_properties.display()
            ))?;
        Ok(())
    }

    pub async fn new(
        config: SetupConfig,
        dot_lodestone_config: DotLodestoneConfig,
        path_to_instance: PathBuf,
        progression_event_id: Snowflake,
        event_broadcaster: EventBroadcaster,
        macro_executor: MacroExecutor,
    ) -> Result<MinecraftBedrockInstance, Error> {
        // Step 2: Download server zip
        let server_zip_url = get_server_zip_url(&config.version)
            .await
            .ok_or_else({
                || {
                    eyre!(
                        "Could get the server zip url, this is a bug, please report it",
                    )
                }
            })?;

        let server_zip = download_file(
            server_zip_url.as_str(),
            &path_to_instance,
            Some("server.zip"),
            {
                let event_broadcaster = event_broadcaster.clone();
                let progression_event_id = progression_event_id;
                &move |dl| {
                    if let Some(total) = dl.total {
                        let _ = event_broadcaster.send(Event {
                            event_inner: EventInner::ProgressionEvent(ProgressionEvent {
                                event_id: progression_event_id,
                                progression_event_inner: ProgressionEventInner::ProgressionUpdate {
                                    progress: (dl.step as f64 / total as f64) * 3.0,
                                    progress_message: format!(
                                        "1/3: Downloading {}",
                                        format_byte_download(dl.downloaded, total),
                                    ),
                                },
                            }),
                            details: "".to_string(),
                            snowflake: Snowflake::default(),
                            caused_by: CausedBy::Unknown,
                        });
                    } else {
                        let _ = event_broadcaster.send(Event {
                            event_inner: EventInner::ProgressionEvent(ProgressionEvent {
                                event_id: progression_event_id,
                                progression_event_inner: ProgressionEventInner::ProgressionUpdate {
                                    progress: 0.0,
                                    progress_message: format!(
                                        "1/3: Downloading {}",
                                        format_byte(dl.downloaded),
                                    ),
                                },
                            }),
                            details: "".to_string(),
                            snowflake: Snowflake::default(),
                            caused_by: CausedBy::Unknown,
                        });
                    }
                }
            },
            true,
        )
        .await?;

        // Step 2: Unzip server zip
        unzip_file(&server_zip, &path_to_instance, true).await?;

        tokio::fs::remove_file(&server_zip).await.context(format!(
            "Could not remove zip {}",
            server_zip.display()
        ))?;

        let path_to_config = path_to_instance.join(".lodestone_minecraft_config.json");
        let path_to_macros = path_to_instance.join("macros");
        let path_to_properties = path_to_instance.join("server.properties");

        let uuid = dot_lodestone_config.uuid().to_owned();

        // Step 2: Create Directories
        let _ = event_broadcaster.send(Event {
            event_inner: EventInner::ProgressionEvent(ProgressionEvent {
                event_id: progression_event_id,
                progression_event_inner: ProgressionEventInner::ProgressionUpdate {
                    progress: 1.0,
                    progress_message: "2/3: Creating directories".to_string(),
                },
            }),
            details: "".to_string(),
            snowflake: Snowflake::default(),
            caused_by: CausedBy::Unknown,
        });
        
        tokio::fs::create_dir_all(&path_to_instance)
            .await
            .and(tokio::fs::create_dir_all(&path_to_macros).await)
            .and(
                tokio::fs::write(&path_to_properties, format!("server-port={}", config.port)).await,
            )
            .context("Could not create some files or directories for instance")
            .map_err(|e| {
                error!("{e}");
                e
            })?;


        
        // Step 3: Finishing Up
        let _ = event_broadcaster.send(Event {
            event_inner: EventInner::ProgressionEvent(ProgressionEvent {
                event_id: progression_event_id,
                progression_event_inner: ProgressionEventInner::ProgressionUpdate {
                    progress: 1.0,
                    progress_message: "3/3: Finishing up".to_string(),
                },
            }),
            details: "".to_string(),
            snowflake: Snowflake::default(),
            caused_by: CausedBy::Unknown,
        });

        let restore_config = RestoreConfig {
            name: config.name,
            version: config.version,
            description: config.description.unwrap_or_default(),
            port: config.port,
            auto_start: config.auto_start.unwrap_or(false),
            restart_on_crash: config.restart_on_crash.unwrap_or(false),
            backup_period: config.backup_period,
            has_started: false,
        };
        // create config file
        tokio::fs::write(
            &path_to_config,
            to_string_pretty(&restore_config).context(
                "Failed to serialize config to string. This is a bug, please report it.",
            )?,
        )
        .await
        .context(format!(
            "Failed to write config file at {}",
            &path_to_config.display()
        ))?;

        MinecraftBedrockInstance::restore(
            path_to_instance,
            dot_lodestone_config,
            uuid,
            event_broadcaster,
            macro_executor,
        )
        .await
    }

    pub async fn restore(
        path_to_instance: PathBuf,
        dot_lodestone_config: DotLodestoneConfig,
        instance_uuid: InstanceUuid,
        event_broadcaster: EventBroadcaster,
        _macro_executor: MacroExecutor,
    ) -> Result<MinecraftBedrockInstance, Error> {
        let path_to_config = path_to_instance.join(".lodestone_minecraft_config.json");
        let restore_config: RestoreConfig =
            serde_json::from_reader(std::fs::File::open(&path_to_config).context(format!(
                "Failed to open config file at {}",
                &path_to_config.display()
            ))?)
            .context(
                "Failed to deserialize config from string. Was the config file modified manually?",
            )?;
        let path_to_macros = path_to_instance.join("macros");
        let path_to_worlds = path_to_instance.join("worlds");
        let path_to_properties = path_to_instance.join("server.properties");
        // if the properties file doesn't exist, create it
        if !path_to_properties.exists() {
            tokio::fs::write(
                &path_to_properties,
                format!("server-port={}", restore_config.port),
            )
            .await
            .expect("failed to write to server.properties");
        };

        let state = Arc::new(Mutex::new(State::Stopped));
        let (backup_tx, mut backup_rx): (
            UnboundedSender<BackupInstruction>,
            UnboundedReceiver<BackupInstruction>,
        ) = tokio::sync::mpsc::unbounded_channel();
        let _backup_task = tokio::spawn({
            let backup_period = restore_config.backup_period;
            let path_to_worlds = path_to_worlds.clone();
            let path_to_instance = path_to_instance.clone();
            let state = state.clone();
            async move {
                let backup_now = || async {
                    debug!("Backing up instance");
                    let backup_dir = &path_to_worlds.join("backup");
                    tokio::fs::create_dir_all(&backup_dir).await.ok();
                    // get current time in human readable format
                    let time = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
                    let backup_name = format!("backup-{}", time);
                    let backup_path = backup_dir.join(&backup_name);
                    if let Err(e) = tokio::task::spawn_blocking({
                        let path_to_instance = path_to_instance.clone();
                        let backup_path = backup_path.clone();
                        let mut copy_option = fs_extra::dir::CopyOptions::new();
                        copy_option.copy_inside = true;
                        move || {
                            fs_extra::dir::copy(
                                path_to_instance.join("world"),
                                &backup_path,
                                &copy_option,
                            )
                        }
                    })
                    .await
                    {
                        error!("Failed to backup instance: {}", e);
                    }
                };
                let mut backup_period = backup_period;
                let mut counter = 0;
                loop {
                    tokio::select! {
                           instruction = backup_rx.recv() => {
                             if instruction.is_none() {
                                 info!("Backup task exiting");
                                 break;
                             }
                             let instruction = instruction.unwrap();
                             match instruction {
                             BackupInstruction::SetPeriod(new_period) => {
                                 backup_period = new_period;
                             },
                             BackupInstruction::BackupNow => backup_now().await,
                             BackupInstruction::Pause => {
                                     loop {
                                         if let Some(BackupInstruction::Resume) = backup_rx.recv().await {
                                             break;
                                         } else {
                                             continue
                                         }
                                     }

                             },
                             BackupInstruction::Resume => {
                                 continue;
                             },
                             }
                           }
                           _ = tokio::time::sleep(Duration::from_secs(1)) => {
                             if let Some(period) = backup_period {
                                 if *state.lock().await == State::Running {
                                     debug!("counter is {}", counter);
                                     counter += 1;
                                     if counter >= period {
                                         counter = 0;
                                         backup_now().await;
                                     }
                                 }
                             }
                           }
                    }
                }
            }
        });


        let configurable_manifest = Arc::new(Mutex::new(Self::init_configurable_manifest(
            &restore_config,
        )));

        let mut instance = MinecraftBedrockInstance {
            state: Arc::new(Mutex::new(State::Stopped)),
            uuid: instance_uuid.clone(),
            creation_time: dot_lodestone_config.creation_time(),
            auto_start: Arc::new(AtomicBool::new(restore_config.auto_start)),
            restart_on_crash: Arc::new(AtomicBool::new(restore_config.restart_on_crash)),
            backup_period: restore_config.backup_period,
            players_manager: Arc::new(Mutex::new(PlayersManager::new(
                event_broadcaster.clone(),
                instance_uuid,
            ))),
            config: Arc::new(Mutex::new(restore_config)),
            path_to_instance,
            path_to_config,
            path_to_properties,
            path_to_macros,
            path_to_worlds,
            macro_executor: MacroExecutor::new(event_broadcaster.clone()),
            event_broadcaster,
            process: Arc::new(Mutex::new(None)),
            system: Arc::new(Mutex::new(sysinfo::System::new_all())),
            stdin: Arc::new(Mutex::new(None)),
            backup_sender: backup_tx,
            configurable_manifest,
            macro_name_to_last_run: Arc::new(Mutex::new(HashMap::new())),
            pid_to_task_entry: Arc::new(Mutex::new(IndexMap::new())),
        };
        instance
            .read_properties()
            .await
            .context("Failed to read properties")?;
        Ok(instance)
    }
}

#[tokio::test]
async fn test_setup_server() {

    let setup_conf = SetupConfig {
        name: String::from("test"),
        version: String::from("1.19.71.02"),
        port: 25567,
        description: Some(String::from("test")),
        auto_start: Some(false),
        restart_on_crash: Some(true),
        backup_period: Some(0),
    };

    let lodestone_conf = DotLodestoneConfig::new(
        Default::default(), 
        GameType::MinecraftBedrock,
    );

    let (event_broadcaster, _) = EventBroadcaster::new(10);
    let macro_executor = MacroExecutor::new(event_broadcaster.clone());
        // config: SetupConfig,
        // dot_lodestone_config: DotLodestoneConfig,
        // path_to_instance: PathBuf,
        // progression_event_id: Snowflake,
        // event_broadcaster: Sender<Event>,
        // macro_executor: MacroExecutor,
    let mut instance = MinecraftBedrockInstance::new(
        setup_conf,
        lodestone_conf,
        PathBuf::from(r"D:\Programming\gamerinstance"),
        Snowflake::default(),
        event_broadcaster,
        macro_executor,
    ).await.unwrap();

    let caused_by = CausedBy::Unknown;

    instance.start(caused_by, false).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(500)).await;
}

impl TInstance for MinecraftBedrockInstance {}