use std::str::FromStr;
use std::sync::atomic;

use async_trait::async_trait;
use color_eyre::eyre::{eyre, Context, ContextCompat};
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
        self.config.name.clone()
    }

    async fn game_type(&self) -> Game {
        Game::MinecraftBedrock { }
    }
    async fn flavour(&self) -> String {
        String::from("Vanilla")
    }

    async fn description(&self) -> String {
        self.config.description.clone()
    }

    async fn port(&self) -> u32 {
        self.config.port
    }

    async fn creation_time(&self) -> i64 {
        self.creation_time
    }

    async fn path(&self) -> std::path::PathBuf {
        self.path_to_instance.clone()
    }

    async fn auto_start(&self) -> bool {
        self.config.auto_start
    }

    async fn restart_on_crash(&self) -> bool {
        self.config.restart_on_crash
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
        self.config.name = name;
        self.write_config_to_file().await?;
        Ok(())
    }

    async fn set_description(&mut self, description: String) -> Result<(), Error> {
        self.config.description = description;
        self.write_config_to_file().await?;
        Ok(())
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