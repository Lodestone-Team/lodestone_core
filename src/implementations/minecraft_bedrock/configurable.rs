use std::str::FromStr;
use std::sync::atomic;

use async_trait::async_trait;
use color_eyre::eyre::{eyre, Context, ContextCompat};
use deno_ast::swc::common::errors::Level;
use tempdir::TempDir;

use crate::error::{Error, ErrorKind};
use crate::traits::t_configurable::manifest::{
    ConfigurableManifest, ConfigurableValue, ConfigurableValueType, SettingManifest,
};
use crate::traits::t_configurable::{Game, TConfigurable};
use crate::traits::t_server::State;

use crate::types::InstanceUuid;
use crate::util::download_file;

use super::{MinecraftBedrockInstance};

#[async_trait]
impl TConfigurable for MinecraftBedrockInstance {
    async fn uuid(&self) -> InstanceUuid {
        self.uuid.clone()
    }

    async fn name(&self) -> String {
        self.config.lock().await.name.clone()
    }

    async fn game_type(&self) -> Game {
        Game::MinecraftBedrock{ }
    }

    async fn version(&self) -> String {
        self.config.lock().await.version.clone()
    }

    async fn description(&self) -> String {
        self.config.lock().await.description.clone()
    }

    async fn port(&self) -> u32 {
        self.config.lock().await.port
    }

    async fn creation_time(&self) -> i64 {
        self.creation_time
    }

    async fn path(&self) -> std::path::PathBuf {
        self.path_to_instance.clone()
    }

    async fn auto_start(&self) -> bool {
        self.config.lock().await.auto_start
    }

    async fn restart_on_crash(&self) -> bool {
        self.config.lock().await.restart_on_crash
    }

    async fn set_name(&mut self, name: String) -> Result<(), Error> {
        if name.is_empty() {
            return Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Name cannot be empty"),
            });
        }
        if name.len() > 100 {
            return Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Name cannot be longer than 100 characters"),
            });
        }
        self.config.lock().await.name = name;
        self.write_config_to_file().await?;
        Ok(())
    }

    async fn set_description(&mut self, description: String) -> Result<(), Error> {
        self.config.lock().await.description = description;
        self.write_config_to_file().await?;
        Ok(())
    }

    async fn set_port(&mut self, port: u32) -> Result<(), Error> {
        self.configurable_manifest.lock().await.set_setting(
            ServerPropertySetting::get_section_id(),
            ServerPropertySetting::ServerPort(port as u16).into(),
        )?;
        self.config.lock().await.port = port;

        self.write_config_to_file()
            .await
            .and(self.write_properties_to_file().await)
    }
    async fn set_auto_start(&mut self, auto_start: bool) -> Result<(), Error> {
        self.config.lock().await.auto_start = auto_start;
        self.auto_start.store(auto_start, atomic::Ordering::Relaxed);
        self.write_config_to_file().await
    }

    async fn set_restart_on_crash(&mut self, restart_on_crash: bool) -> Result<(), Error> {
        self.config.lock().await.restart_on_crash = restart_on_crash;
        self.auto_start
            .store(restart_on_crash, atomic::Ordering::Relaxed);
        self.write_config_to_file().await
    }

    async fn change_version(&mut self, version: String) -> Result<(), Error> {
        Err(Error {
            kind: ErrorKind::UnsupportedOperation,
            source: eyre!("This instance does not support changing version"),
        })
    }
    
    async fn configurable_manifest(&self) -> ConfigurableManifest {
        self.configurable_manifest.lock().await.clone()
    }

    async fn update_configurable(
        &mut self,
        section_id: &str,
        setting_id: &str,
        value: ConfigurableValue,
    ) -> Result<(), Error> {
        self.configurable_manifest
            .lock()
            .await
            .update_setting_value(section_id, setting_id, value.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) enum Gamemode {
    #[default]
    Survival,
    Creative,
    Adventure,
}

impl ToString for Gamemode {
    fn to_string(&self) -> String {
        match self {
            Gamemode::Survival => "survival",
            Gamemode::Creative => "creative",
            Gamemode::Adventure => "adventure",
        }
        .to_string()
    }
}

impl FromStr for Gamemode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "survival" => Ok(Gamemode::Survival),
            "creative" => Ok(Gamemode::Creative),
            "adventure" => Ok(Gamemode::Adventure),
            _ => Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Invalid gamemode. The only valid gamemodes are: survival, creative, adventure, spectator"),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) enum Difficulty {
    #[default]
    Peaceful,
    Easy,
    Normal,
    Hard,
}
impl FromStr for Difficulty {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "peaceful" => Ok(Difficulty::Peaceful),
            "easy" => Ok(Difficulty::Easy),
            "normal" => Ok(Difficulty::Normal),
            "hard" => Ok(Difficulty::Hard),
            _ => Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Invalid difficulty. The only valid gamemodes are: peaceful, easy, normal, hard"),
            }),
        }
    }
}


impl ToString for Difficulty {
    fn to_string(&self) -> String {
        match self {
            Difficulty::Peaceful => "peaceful",
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
            Difficulty::Hard => "hard",
        }
        .to_string()
    }
}


#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) enum LevelType{
    #[default]
    Flat,
    Legacy,
    Default,
}

impl ToString for LevelType {
    fn to_string(&self) -> String {
        match self {
            LevelType::Flat => "flat",
            LevelType::Legacy => "legacy",
            LevelType::Default => "default",
        }
        .to_string()
    }
}

impl FromStr for LevelType {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "flat" => Ok(LevelType::Flat),
            "legacy" => Ok(LevelType::Legacy),
            "default" => Ok(LevelType::Default),
            _ => Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Invalid gamemode. The only valid gamemodes are: survival, creative, adventure, spectator"),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum DefaultPlayerPermissionLevel{
    #[default]
    Visitor,
    Member,
    Operator,
}

impl FromStr for DefaultPlayerPermissionLevel {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "visitor" => Ok(DefaultPlayerPermissionLevel::Visitor),
            "member" => Ok(DefaultPlayerPermissionLevel::Member),
            "operator" => Ok(DefaultPlayerPermissionLevel::Operator),
            _ => Err(Error {
                kind: ErrorKind::BadRequest,
                source: eyre!("Invalid default permission level. The only valid default permission levels are: visitor, member, operator"),
            }),
        }
    }
}

impl ToString for DefaultPlayerPermissionLevel {
    fn to_string(&self) -> String {
        match self {
            DefaultPlayerPermissionLevel::Visitor => "visitor",
            DefaultPlayerPermissionLevel::Member => "member",
            DefaultPlayerPermissionLevel::Operator => "operator",
        }
        .to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum ServerPropertySetting {
    Gamemode(Gamemode),
    ForceGamemode(bool),
    Difficulty(Difficulty),
    LevelType(LevelType),
    ServerName(String),
    MaxPlayers(u32),
    ServerPort(u16),
    ServerPortv6(u16),
    LevelName(String),
    LevelSeed(String),
    OnlineMode(bool),
    AllowList(bool),
    AllowCheats(bool),
    ViewDistance(u32),
    PlayerIdleTimeout(u32),
    MaxThreads(u16),
    TickDistance(u8),
    DefaultPlayerPermissionLevel(DefaultPlayerPermissionLevel),
    TexturePackRequired(bool),
    ContentLogFileEnabled(bool),
    CompressionThreshold(u16),
    ServerAuthoritativeMovement(bool),
    PlayerMovementScoreThreshold(u32),
    PlayerMovementActionDirectionThreshold(f32),
    PlayerMovementDistanceThreshold(f32),
    PlayerMovementDurationThresholdInMs(u32),
    CorrectPlayerMovement(bool),
    DisablePlayerInteraction(bool),
    Unknown(String, String),
}

impl From<ServerPropertySetting> for SettingManifest {
    fn from(value: ServerPropertySetting) -> Self {
        match value {
            ServerPropertySetting::Gamemode(ref inner_val) => Self::new_value_with_type(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                Some(ConfigurableValue::Enum(inner_val.to_string())),
                ConfigurableValueType::Enum {
                    options: vec![
                        "survival".to_string(),
                        "creative".to_string(),
                        "adventure".to_string(),
                        "spectator".to_string(),
                    ],
                },
                None,
                false,
                true,
            ),
            ServerPropertySetting::ForceGamemode(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::Boolean(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::Difficulty(ref inner_val) => Self::new_value_with_type(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                Some(ConfigurableValue::Enum(inner_val.to_string())),
                ConfigurableValueType::Enum {
                    options: vec![
                        "peaceful".to_string(),
                        "easy".to_string(),
                        "normal".to_string(),
                        "hard".to_string(),
                    ],
                },
                None,
                false,
                true,
            ),
            ServerPropertySetting::LevelType(ref inner_val) => Self::new_value_with_type(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                Some(ConfigurableValue::Enum(inner_val.to_string())),
                ConfigurableValueType::Enum {
                    options: vec![
                        "flat".to_string(),
                        "legacy".to_string(),
                        "default".to_string(),
                    ],
                },
                None,
                false,
                true,
            ),
            ServerPropertySetting::ServerName(ref inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::String(inner_val.clone()),
                None,
                false,
                true,
            ),
            ServerPropertySetting::MaxPlayers(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::UnsignedInteger(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::ServerPort(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::UnsignedInteger(inner_val as u32),
                None,
                false,
                true,
            ),
            ServerPropertySetting::ServerPortv6(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::UnsignedInteger(inner_val as u32),
                None,
                false,
                true,
            ),
            ServerPropertySetting::LevelName(ref inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::String(inner_val.clone()),
                None,
                false,
                true,
            ),
            ServerPropertySetting::LevelSeed(ref inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::String(inner_val.clone()),
                None,
                false,
                true,
            ), 
            ServerPropertySetting::OnlineMode(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::Boolean(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::AllowList(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::Boolean(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::AllowCheats(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::Boolean(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::ViewDistance(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::UnsignedInteger(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::PlayerIdleTimeout(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::UnsignedInteger(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::MaxThreads(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::UnsignedInteger(inner_val as u32),
                None,
                false,
                true,
            ),
            ServerPropertySetting::TickDistance(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::UnsignedInteger(inner_val as u32),
                None,
                false,
                true,
            ),
            ServerPropertySetting::DefaultPlayerPermissionLevel(inner_val) => Self::new_value_with_type(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                Some(ConfigurableValue::Enum(inner_val.to_string())),
                ConfigurableValueType::Enum {
                    options: vec![
                        "visitor".to_string(),
                        "member".to_string(),
                        "operator".to_string(),
                    ],
                },
                None,
                false,
                true,
            ),
            ServerPropertySetting::TexturePackRequired(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::Boolean(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::ContentLogFileEnabled(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::Boolean(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::CompressionThreshold(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::UnsignedInteger(inner_val as u32),
                None,
                false,
                true,
            ),
             ServerPropertySetting::ServerAuthoritativeMovement(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::Boolean(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::PlayerMovementScoreThreshold(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::UnsignedInteger(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::PlayerMovementDistanceThreshold(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::Float(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::PlayerMovementActionDirectionThreshold(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::Float(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::PlayerMovementDurationThresholdInMs(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::UnsignedInteger(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::CorrectPlayerMovement(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::Boolean(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::DisablePlayerInteraction(inner_val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::Boolean(inner_val),
                None,
                false,
                true,
            ),
            ServerPropertySetting::Unknown(_, ref val) => Self::new_required_value(
                value.get_identifier(),
                value.get_name(),
                value.get_description(),
                ConfigurableValue::String(val.clone()),
                None,
                false,
                true,
            ),
        }
    }
}

impl ServerPropertySetting {
    pub fn get_section_id() -> &'static str {
        "server_properties_section"
    }

    pub fn get_identifier(&self) -> String {
        match self {
            Self::Gamemode(_) => "gamemode",
            Self::ForceGamemode(_) => "force-gamemode",
            Self::Difficulty(_) => "difficulty",
            Self::LevelType(_) => "level-type",
            Self::ServerName(_) => "server-name",
            Self::MaxPlayers(_) => "max-players",
            Self::ServerPort(_) => "server-port",
            Self::ServerPortv6(_) => "server-portv6",
            Self::LevelName(_) => "level-name",
            Self::LevelSeed(_) => "level-seed",
            Self::OnlineMode(_) => "online-mode",
            Self::AllowList(_) => "allow-list",
            Self::AllowCheats(_) => "allow-cheats",
            Self::ViewDistance(_) => "view-distance",
            Self::PlayerIdleTimeout(_) => "player-idle-timeout",
            Self::MaxThreads(_) => "max-threads",
            Self::TickDistance(_) => "tick-distance",
            Self::DefaultPlayerPermissionLevel(_) => "default-player-permission-level",
            Self::TexturePackRequired(_) => "texturepack-required",
            Self::ContentLogFileEnabled(_) => "content-log-file-enabled",
            Self::CompressionThreshold(_) => "compression-threshold	",
            Self::ServerAuthoritativeMovement(_) => "server-authoritative-movement",
            Self::PlayerMovementScoreThreshold(_) => "player-movement-score-threshold",
            Self::PlayerMovementActionDirectionThreshold(_) => "player-movement-action-direction-threshold",
            Self::PlayerMovementDistanceThreshold(_) => "player-movement-distance-threshold",
            Self::PlayerMovementDurationThresholdInMs(_) => "player-movement-duration-threshold-in-ms",
            Self::CorrectPlayerMovement(_) => "correct-player-movement",
            Self::DisablePlayerInteraction(_) => "disable-player-interaction",
            Self::Unknown(key, _) => key,
        }
        .to_string()
    }

    // name to be displayed in the UI
    fn get_name(&self) -> String {
        if let Self::Unknown(key, _) = self {
            // capitalize the first letter of the key
            let mut chars = key.chars();
            let first = chars.next().unwrap().to_uppercase();
            let rest = chars.as_str();
            return format!("{}{}", first, rest);
        };
        match self {
            Self::Gamemode(_) => "Gamemode",
            Self::ForceGamemode(_) => "Force Gamemode",
            Self::Difficulty(_) => "Difficulty",
            Self::LevelType(_) => "Level Type",
            Self::ServerName(_) => "Server Name",
            Self::MaxPlayers(_) => "Max Players",
            Self::ServerPort(_) => "Server Port (IPv4)",
            Self::ServerPortv6(_) => "Server Port (IPv6)",
            Self::LevelName(_) => "Level Name",
            Self::LevelSeed(_) => "Level Seed",
            Self::OnlineMode(_) => "Online Mode",
            Self::AllowList(_) => "Allow List",
            Self::AllowCheats(_) => "Allow Cheats",
            Self::ViewDistance(_) => "View Distance",
            Self::PlayerIdleTimeout(_) => "Player Idle Timeout",
            Self::MaxThreads(_) => "Max Threads",
            Self::TickDistance(_) => "Tick Distance",
            Self::DefaultPlayerPermissionLevel(_) => "Default Player Permission Level",
            Self::TexturePackRequired(_) => "Texturepack Required",
            Self::ContentLogFileEnabled(_) => "Content Log File Enabled",
            Self::CompressionThreshold(_) => "Compression Threshold	",
            Self::ServerAuthoritativeMovement(_) => "Server Authoritative Movement",
            Self::PlayerMovementScoreThreshold(_) => "Player Movement Score Threshold",
            Self::PlayerMovementActionDirectionThreshold(_) => "Player Movement Action Direction Threshold",
            Self::PlayerMovementDistanceThreshold(_) => "Player Movement Distance Threshold",
            Self::PlayerMovementDurationThresholdInMs(_) => "Player Movement Duration Threshold (in ms)",
            Self::CorrectPlayerMovement(_) => "Correct Player Movement",
            Self::DisablePlayerInteraction(_) => "Disable Player Interaction",
            Self::Unknown(_, _) => unreachable!("Handled above"),
        }
        .to_string()
    }

    // a short description of the property
    fn get_description(&self) -> String {
        if let Self::Unknown(key, val) = self {
            return format!(
                "Unknown property: {key} = {val} Please report this to the developers."
            );
        };

        match self {
            Self::Gamemode(_) => "A variable representing the game mode of the server",
            Self::ForceGamemode(_) => "A variable representing whether the server enforces the game mode",
            Self::Difficulty(_) => "A variable representing the difficulty level of the server",
            Self::LevelType(_) => "A variable representing the type of the server's level",
            Self::ServerName(_) => "A variable representing the name of the server",
            Self::MaxPlayers(_) => "A variable representing the maximum number of players allowed on the server",
            Self::ServerPort(_) => "A variable representing the IPv4 port of the server",
            Self::ServerPortv6(_) => "A variable representing the IPv6 port of the server",
            Self::LevelName(_) => "A variable representing the name of the server's level",
            Self::LevelSeed(_) => "A variable representing the seed for the server's level generation",
            Self::OnlineMode(_) => "A variable representing whether the server is in online mode or not",
            Self::AllowList(_) => "A variable representing the list of players allowed on the server",
            Self::AllowCheats(_) => "A variable representing whether cheats are allowed on the server",
            Self::ViewDistance(_) => "A variable representing the maximum distance players can see",
            Self::PlayerIdleTimeout(_) => "A variable representing the time until idle players are kicked from the server",
            Self::MaxThreads(_) => "A variable representing the maximum number of threads the server can use",
            Self::TickDistance(_) => "A variable representing the distance from a player before their chunks are ticked",
            Self::DefaultPlayerPermissionLevel(_) => "A variable representing the default permission level of players on the server",
            Self::TexturePackRequired(_) => "A variable representing whether a texture pack is required to join the server",
            Self::ContentLogFileEnabled(_) => "A variable representing whether the content log file is enabled",
            Self::CompressionThreshold(_) => "A variable representing the compression threshold for network packets",
            Self::ServerAuthoritativeMovement(_) => "A variable representing whether the server's movement calculations are authoritative",
            Self::PlayerMovementScoreThreshold(_) => "A variable representing the movement score threshold for players",
            Self::PlayerMovementActionDirectionThreshold(_) => "A variable representing the movement action direction threshold for players",
            Self::PlayerMovementDistanceThreshold(_) => "A variable representing the movement distance threshold for players",
            Self::PlayerMovementDurationThresholdInMs(_) => "A variable representing the movement duration threshold for players in milliseconds",
            Self::CorrectPlayerMovement(_) => "A variable representing whether the server corrects player movement",
            Self::DisablePlayerInteraction(_) => "A variable representing whether player interaction is disabled on the server",
            Self::Unknown(_, _) => unreachable!("Handled above"),
       }.to_string()
    }

    pub fn from_key_val(key: &str, value: &str) -> Result<Self, Error> {
        match key {
            "gamemode" => {
                Ok(Self::Gamemode(value.parse::<Gamemode>().with_context(
                    || eyre!("Invalid value: {value} for \"gamemode\", expected Gamemode"),
                )?))
            },
            "force-gamemode" => {
                Ok(Self::ForceGamemode(value.parse::<bool>().with_context(
                    || eyre!("Invalid value: {value} for \"force-gamemode\", expected bool"),
                )?))
            },
            "difficulty" => {
                Ok(Self::Difficulty(value.parse::<Difficulty>().with_context(
                    || eyre!("Invalid value: {value} for \"difficulty\", expected Difficulty."),
                )?))
            },
            "level-type" => {
                Ok(Self::LevelType(value.parse::<LevelType>().with_context(
                    || eyre!("Invalid value: {value} for \"level-type\", expected Gamemode"),
                )?))
            },
            "server-name" => {
                Ok(Self::ServerName(value.to_string()))
            },
            "max-players" => {
                Ok(Self::MaxPlayers(value.parse::<u32>().with_context(
                    || eyre!("Invalid value: {value} for \"max-players\", expected u32"),
                )?))
            },
            "server-port" => {
                Ok(Self::ServerPort(value.parse::<u16>().with_context(
                    || eyre!("Invalid value: {value} for \"server-port\", expected u16"),
                )?))
            },
            "server-portv6" => {
                Ok(Self::ServerPortv6(value.parse::<u16>().with_context(
                    || eyre!("Invalid value: {value} for \"server-portv6\", expected u16"),
                )?))
            },
            "level-name" => {
                Ok(Self::LevelName(value.to_string()))
            },
            "level-seed" => {
                Ok(Self::LevelSeed(value.to_string()))
            },
            "online-mode" => {
                Ok(Self::OnlineMode(value.parse::<bool>().with_context(
                    || eyre!("Invalid value: {value} for \"online-mode\", expected bool"),
                )?))
            },
            "allow-list" => {
                Ok(Self::AllowList(value.parse::<bool>().with_context(
                    || eyre!("Invalid value: {value} for \"allow-list\", expected bool"),
                )?))
            },
            "allow-cheats" => {
                Ok(Self::AllowCheats(value.parse::<bool>().with_context(
                    || eyre!("Invalid value: {value} for \"allow-cheats\", expected bool"),
                )?))
            },
            "view-distance" => {
                Ok(Self::ViewDistance(value.parse::<u32>().with_context(
                    || eyre!("Invalid value: {value} for \"view-distance\", expected u8"),
                )?))
            },
            "player-idle-timeout" => {
                Ok(Self::PlayerIdleTimeout(value.parse::<u32>().with_context(
                    || eyre!("Invalid value: {value} for \"player-idle-timeout\", expected u32"),
                )?))
            },
            "max-threads" => {
                Ok(Self::MaxThreads(value.parse::<u16>().with_context(
                    || eyre!("Invalid value: {value} for \"max-threads\", expected u8"),
                )?))
            },
            "tick-distance" => {
                Ok(Self::TickDistance(value.parse::<u8>().with_context(
                    || eyre!("Invalid value: {value} for \"tick-distance\", expected u8"),
                )?))
            },
            "default-player-permission-level" => {
                Ok(Self::DefaultPlayerPermissionLevel(value.parse::<DefaultPlayerPermissionLevel>().with_context(
                    || eyre!("Invalid value: {value} for \"default-player-permission-level\", expected DefaultPlayerPermissionLevel"),
                )?))
            },
            "texturepack-required" => {
                Ok(Self::TexturePackRequired(value.parse::<bool>().with_context(
                    || eyre!("Invalid value: {value} for \"texturepack-required\", expected bool"),
                )?))
            },
            "content-log-file-enabled" => {
                Ok(Self::ContentLogFileEnabled(value.parse::<bool>().with_context(
                    || eyre!("Invalid value: {value} for \"content-log-file-enabled\", expected bool"),
                )?))
            },
            "compression-threshold" => {
                Ok(Self::CompressionThreshold(value.parse::<u16>().with_context(
                    || eyre!("Invalid value: {value} for \"compression-threshold\", expected u16"),
                )?))
            },
            "server-authoritative-movement" => {
                Ok(Self::ServerAuthoritativeMovement(value.parse::<bool>().with_context(
                    || eyre!("Invalid value: {value} for \"server-authoritative-movement\", expected bool"),
                )?))
            },
            "player-movement-score-threshold" => {
                Ok(Self::PlayerMovementScoreThreshold(value.parse::<u32>().with_context(
                    || eyre!("Invalid value: {value} for \"player-movement-score-threshold\", expected u32"),
                )?))
            },
            "player-movement-action-direction-threshold" => {
                Ok(Self::PlayerMovementActionDirectionThreshold(value.parse::<f32>().with_context(
                    || eyre!("Invalid value: {value} for \"player-movement-action-direction-threshold\", expected f32"),
                )?))
            },
            "player-movement-distance-threshold" => {
                Ok(Self::PlayerMovementDistanceThreshold(value.parse::<f32>().with_context(
                    || eyre!("Invalid value: {value} for \"player-movement-distance-threshold\", expected f32"),
                )?))
            },
            "player-movement-duration-threshold-in-ms" => {
                Ok(Self::PlayerMovementDurationThresholdInMs(value.parse::<u32>().with_context(
                    || eyre!("Invalid value: {value} for \"player-movement-duration-threshold-in-ms\", expected u32"),
                )?))
            },
            "correct-player-movement" => {
                Ok(Self::CorrectPlayerMovement(value.parse::<bool>().with_context(
                    || eyre!("Invalid value: {value} for \"correct-player-movement\", expected bool"),
                )?))
            },
            "disable-player-interaction" => {
                Ok(Self::DisablePlayerInteraction(value.parse::<bool>().with_context(
                    || eyre!("Invalid value: {value} for \"disable-player-interaction\", expected bool"),
                )?))
            },
            _ => Ok(Self::Unknown(key.to_string(), value.to_string())),
        }
    }

    pub fn to_line(&self) -> String {
        match self {
            Self::Gamemode(v) => format!("{}={}", self.get_identifier(), v.to_string()),
            Self::ForceGamemode(v) => format!("{}={}", self.get_identifier(), v),
            Self::Difficulty(v) => format!("{}={}", self.get_identifier(), v.to_string()),
            Self::LevelType(v) => format!("{}={}", self.get_identifier(), v.to_string()),
            Self::ServerName(v) => format!("{}={}", self.get_identifier(), v),
            Self::MaxPlayers(v) => format!("{}={}", self.get_identifier(), v),
            Self::ServerPort(v) => format!("{}={}", self.get_identifier(), v),
            Self::ServerPortv6(v) => format!("{}={}", self.get_identifier(), v),
            Self::LevelName(v) => format!("{}={}", self.get_identifier(), v),
            Self::LevelSeed(v) => format!("{}={}", self.get_identifier(), v),
            Self::OnlineMode(v) => format!("{}={}", self.get_identifier(), v),
            Self::AllowList(v) => format!("{}={}", self.get_identifier(), v),
            Self::AllowCheats(v) => format!("{}={}", self.get_identifier(), v),
            Self::ViewDistance(v) => format!("{}={}", self.get_identifier(), v),
            Self::PlayerIdleTimeout(v) => format!("{}={}", self.get_identifier(), v),
            Self::MaxThreads(v) => format!("{}={}", self.get_identifier(), v),
            Self::TickDistance(v) => format!("{}={}", self.get_identifier(), v),
            Self::DefaultPlayerPermissionLevel(v) => format!("{}={}", self.get_identifier(), v.to_string()),
            Self::TexturePackRequired(v) => format!("{}={}", self.get_identifier(), v),
            Self::ContentLogFileEnabled(v) => format!("{}={}", self.get_identifier(), v),
            Self::CompressionThreshold(v) => format!("{}={}", self.get_identifier(), v),
            Self::ServerAuthoritativeMovement(v) => format!("{}={}", self.get_identifier(), v),
            Self::PlayerMovementScoreThreshold(v) => format!("{}={}", self.get_identifier(), v),
            Self::PlayerMovementActionDirectionThreshold(v) => format!("{}={}", self.get_identifier(), v),
            Self::PlayerMovementDistanceThreshold(v) => format!("{}={}", self.get_identifier(), v),
            Self::PlayerMovementDurationThresholdInMs(v) => format!("{}={}", self.get_identifier(), v),
            Self::CorrectPlayerMovement(v) => format!("{}={}", self.get_identifier(), v),
            Self::DisablePlayerInteraction(v) => format!("{}={}", self.get_identifier(), v),
            Self::Unknown(_k, v) => format!("{}={}", self.get_identifier(), v),
        }
    }
}

impl FromStr for ServerPropertySetting {
    type Err = Error;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let mut split = line.split('=');
        let key = split
            .next()
            .with_context(|| eyre!("Invalid line, no key: {}", line))?;
        let value = split
            .next()
            .with_context(|| eyre!("Invalid line, no value: {}", line))?;

        Self::from_key_val(key, value)
    }
}

// #[cfg(test)]
// mod test {
//     use std::io::BufRead;

//     use crate::traits::t_configurable::manifest::SectionManifest;

//     use super::*;

//     #[test]
//     fn test_parse_server_properties() {
//         let properties =
//             "enable-jmx-monitoring=false\nrcon.port=25575\nlevel-seed=\ndifficulty=easy";

//         let mut res: Vec<ServerPropertySetting> = Vec::new();
//         for (line_num, line) in properties.lines().enumerate() {
//             if let Ok(entry) = ServerPropertySetting::from_str(line) {
//                 res.push(entry);
//             } else {
//                 panic!("Failed to parse line: {} at {line_num}", line);
//             }
//         }


//         assert_eq!(res[2], ServerPropertySetting::LevelSeed("".to_string()));

//         assert_eq!(res[3], ServerPropertySetting::Difficulty(Difficulty::Easy));
//     }

//     #[test]
//     fn test_exhausiveness() {
//         let properties_file = std::io::BufReader::new(
//             std::fs::File::open("src/testdata/sample_server.properties")
//                 .expect("Failed to open server.properties"),
//         );
//         let mut config_section = SectionManifest::new(
//             String::from("server_properties"),
//             String::from("Server Properties Test"),
//             Default::default(),
//             Default::default(),
//         );

//         for line in properties_file.lines() {
//             let line = line.expect("Failed to read line");
//             match ServerPropertySetting::from_str(&line) {
//                 Ok(v) => {
//                     if let ServerPropertySetting::Unknown(_, _) = v {
//                         panic!("Unknown property: {}", line);
//                     }

//                     config_section.add_setting(v.into()).unwrap();
//                 }
//                 Err(e) => panic!("Failed to parse line: {} with error: {}", line, e),
//             }
//         }

//         assert!(!config_section
//             .get_setting("enable-jmx-monitoring")
//             .unwrap()
//             .get_value()
//             .unwrap()
//             .try_as_boolean()
//             .unwrap());

//         let property: ServerPropertySetting = config_section
//             .get_setting("enable-jmx-monitoring")
//             .unwrap()
//             .clone()
//             .try_into()
//             .unwrap();
//         assert_eq!(property, ServerPropertySetting::EnableJmxMonitoring(false));
//         assert_eq!(
//             property.to_line(),
//             "enable-jmx-monitoring=false".to_string()
//         );

//         assert_eq!(
//             config_section
//                 .get_setting("rcon.port")
//                 .unwrap()
//                 .get_value()
//                 .unwrap()
//                 .try_as_unsigned_integer()
//                 .unwrap(),
//             25575
//         );

//         let property: ServerPropertySetting = config_section
//             .get_setting("rcon.port")
//             .unwrap()
//             .clone()
//             .try_into()
//             .unwrap();

//         assert_eq!(property, ServerPropertySetting::RconPort(25575));
//         assert_eq!(property.to_line(), "rcon.port=25575".to_string());

//         assert!(config_section
//             .get_setting("resource-pack")
//             .unwrap()
//             .get_value()
//             .unwrap()
//             .try_as_string()
//             .unwrap()
//             .is_empty());

//         let property: ServerPropertySetting = config_section
//             .get_setting("resource-pack")
//             .unwrap()
//             .clone()
//             .try_into()
//             .unwrap();

//         assert_eq!(
//             property,
//             ServerPropertySetting::ResourcePack("".to_string())
//         );

//         assert_eq!(property.to_line(), "resource-pack=".to_string());
//     }
// }