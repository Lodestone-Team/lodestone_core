pub use std::path::PathBuf;

use rocket::serde::json::serde_json;
pub use serde::{Deserialize, Serialize};

use super::MaybeUnsupported;

pub enum PropertiesError {
    NotFound,
    InvalidValue,
}

#[derive(Debug, Clone, Copy)]
pub enum Flavour {
    Vanilla,
    Fabric,
    Paper,
    Spigot,
}

impl<'de> Deserialize<'de> for Flavour {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "vanilla" => Ok(Flavour::Vanilla),
            "fabric" => Ok(Flavour::Fabric),
            "paper" => Ok(Flavour::Paper),
            "spigot" => Ok(Flavour::Spigot),
            _ => Err(serde::de::Error::custom(format!("Unknown flavour: {}", s))),
        }
    }
}
impl Serialize for Flavour {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Flavour::Vanilla => serializer.serialize_str("vanilla"),
            Flavour::Fabric => serializer.serialize_str("fabric"),
            Flavour::Paper => serializer.serialize_str("paper"),
            Flavour::Spigot => serializer.serialize_str("spigot"),
        }
    }
}

impl ToString for Flavour {
    fn to_string(&self) -> String {
        match self {
            Flavour::Vanilla => "vanilla".to_string(),
            Flavour::Fabric => "fabric".to_string(),
            Flavour::Paper => "paper".to_string(),
            Flavour::Spigot => "spigot".to_string(),
        }
    }
}

pub trait TConfiurable {
    // getters
    fn uuid(&self) -> String;
    fn name(&self) -> String;
    fn flavour(&self) -> MaybeUnsupported<String> {
        MaybeUnsupported::Unsupported
    }
    fn jvm_args(&self) -> MaybeUnsupported<Vec<String>> {
        MaybeUnsupported::Unsupported
    }
    fn description(&self) -> String;
    fn port(&self) -> u32;
    fn min_ram(&self) -> MaybeUnsupported<u32>;
    fn max_ram(&self) -> MaybeUnsupported<u32>;
    fn creation_time(&self) -> u32;
    fn path(&self) -> PathBuf;
    /// does start when lodestone starts
    fn auto_start(&self) -> bool;
    fn restart_on_crash(&self) -> MaybeUnsupported<bool> {
        MaybeUnsupported::Unsupported
    }
    fn timeout_last_left(&self) -> MaybeUnsupported<Option<i32>> {
        MaybeUnsupported::Unsupported
    }
    fn timeout_no_activity(&self) -> MaybeUnsupported<Option<i32>> {
        MaybeUnsupported::Unsupported
    }
    fn start_on_connection(&self) -> MaybeUnsupported<bool> {
        MaybeUnsupported::Unsupported
    }
    fn backup_period(&self) -> MaybeUnsupported<Option<i32>> {
        MaybeUnsupported::Unsupported
    }

    // setters
    fn set_name(&mut self, name: String);
    fn set_description(&mut self, description: String);
    fn set_jvm_args(&mut self, jvm_args: Vec<String>) -> MaybeUnsupported<()> {
        MaybeUnsupported::Unsupported
    }
    fn set_min_ram(&mut self, min_ram: u32) -> MaybeUnsupported<()> {
        MaybeUnsupported::Unsupported
    }
    fn set_max_ram(&mut self, max_ram: u32) -> MaybeUnsupported<()> {
        MaybeUnsupported::Unsupported
    }
    fn set_auto_start(&mut self, auto_start: bool);
    fn set_restart_on_crash(&mut self, restart_on_crash: bool) -> MaybeUnsupported<()> {
        MaybeUnsupported::Unsupported
    }
    fn set_timeout_last_left(&mut self, timeout_last_left: Option<i32>) -> MaybeUnsupported<()> {
        MaybeUnsupported::Unsupported
    }
    fn set_timeout_no_activity(&mut self, timeout_no_activity: Option<i32>) -> MaybeUnsupported<()> {
        MaybeUnsupported::Unsupported
    }
    fn set_start_on_connection(&mut self, start_on_connection: bool) -> MaybeUnsupported<()> {
        MaybeUnsupported::Unsupported
    }
    fn set_backup_period(&mut self, backup_period: Option<i32>) -> MaybeUnsupported<()> {
        MaybeUnsupported::Unsupported
    }

    // server config files (server.properties)
    fn set_field(&mut self, field: &str, value: String) -> Result<(), PropertiesError>;
    fn get_field(&self, field: &str) -> Result<String, PropertiesError>;

    fn setup_params(&self) -> serde_json::Value;
}
