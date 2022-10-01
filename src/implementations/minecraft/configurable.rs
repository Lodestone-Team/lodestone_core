use std::{collections::HashMap, sync::atomic};

use serde_json::json;
use tokio::task;

use crate::traits::{self, t_configurable::TConfigurable, ErrorInner, MaybeUnsupported, Supported};

use crate::traits::Error;

use super::Instance;

impl TConfigurable for Instance {
    fn uuid(&self) -> String {
        self.config.uuid.clone()
    }

    fn name(&self) -> String {
        self.config.name.clone()
    }

    fn game_type(&self) -> String {
        self.config.game_type.clone()
    }

    fn flavour(&self) -> String {
        self.config.flavour.to_string()
    }

    fn cmd_args(&self) -> Vec<String> {
        self.config.cmd_args.clone()
    }

    fn description(&self) -> String {
        self.config.description.clone()
    }

    fn port(&self) -> u32 {
        self.config.port
    }

    fn min_ram(&self) -> MaybeUnsupported<u32> {
        Supported(self.config.min_ram)
    }

    fn max_ram(&self) -> MaybeUnsupported<u32> {
        Supported(self.config.max_ram)
    }

    fn creation_time(&self) -> i64 {
        self.config.creation_time
    }

    fn path(&self) -> std::path::PathBuf {
        self.config.path.clone()
    }

    fn auto_start(&self) -> bool {
        self.config.auto_start
    }

    fn restart_on_crash(&self) -> MaybeUnsupported<bool> {
        Supported(self.config.restart_on_crash)
    }

    fn timeout_last_left(&self) -> MaybeUnsupported<Option<u32>> {
        Supported(self.config.timeout_last_left)
    }

    fn timeout_no_activity(&self) -> MaybeUnsupported<Option<u32>> {
        Supported(self.config.timeout_no_activity)
    }

    fn start_on_connection(&self) -> MaybeUnsupported<bool> {
        Supported(self.config.start_on_connection)
    }

    fn backup_period(&self) -> MaybeUnsupported<Option<u32>> {
        Supported(self.config.backup_period)
    }

    fn get_info(&self) -> serde_json::Value {
        json!(self.config)
    }

    fn set_name(&mut self, name: String) -> Result<(), traits::Error> {
        self.config.name = name;
        self.write_config_to_file()?;
        Ok(())
    }

    fn set_description(&mut self, description: String) -> Result<(), traits::Error> {
        self.config.description = description;
        self.write_config_to_file()?;
        Ok(())
    }

    fn set_port(&mut self, port: u32) -> MaybeUnsupported<Result<(), traits::Error>> {
        Supported({
            self.config.port = port;
            self.write_config_to_file()
        })
    }

    fn set_cmd_argss(
        &mut self,
        cmd_args: Vec<String>,
    ) -> MaybeUnsupported<Result<(), traits::Error>> {
        self.config.cmd_args = cmd_args;
        self.write_config_to_file()
            .map_or_else(|e| Supported(Err(e)), |_| Supported(Ok(())))
    }

    fn set_min_ram(&mut self, min_ram: u32) -> MaybeUnsupported<Result<(), traits::Error>> {
        self.config.min_ram = min_ram;
        self.write_config_to_file()
            .map_or_else(|e| Supported(Err(e)), |_| Supported(Ok(())))
    }

    fn set_max_ram(&mut self, max_ram: u32) -> MaybeUnsupported<Result<(), traits::Error>> {
        self.config.min_ram = max_ram;
        self.write_config_to_file()
            .map_or_else(|e| Supported(Err(e)), |_| Supported(Ok(())))
    }

    fn set_auto_start(&mut self, auto_start: bool) -> MaybeUnsupported<Result<(), traits::Error>> {
        self.config.auto_start = auto_start;
        self.auto_start.store(auto_start, atomic::Ordering::Relaxed);
        self.write_config_to_file()
            .map_or_else(|e| Supported(Err(e)), |_| Supported(Ok(())))
    }

    fn set_restart_on_crash(
        &mut self,
        restart_on_crash: bool,
    ) -> MaybeUnsupported<Result<(), traits::Error>> {
        self.config.restart_on_crash = restart_on_crash;
        self.auto_start
            .store(restart_on_crash, atomic::Ordering::Relaxed);
        self.write_config_to_file()
            .map_or_else(|e| Supported(Err(e)), |_| Supported(Ok(())))
    }

    fn set_timeout_last_left(
        &mut self,
        timeout_last_left: Option<u32>,
    ) -> MaybeUnsupported<Result<(), traits::Error>> {
        task::block_in_place(|| {
            *self.timeout_last_left.blocking_lock() = timeout_last_left;
        });
        self.config.timeout_last_left = timeout_last_left;
        self.write_config_to_file()
            .map_or_else(|e| Supported(Err(e)), |_| Supported(Ok(())))
    }

    fn set_timeout_no_activity(
        &mut self,
        timeout_no_activity: Option<u32>,
    ) -> MaybeUnsupported<Result<(), traits::Error>> {
        task::block_in_place(|| {
            *self.timeout_no_activity.blocking_lock() = timeout_no_activity;
        });
        self.config.timeout_no_activity = timeout_no_activity;
        self.write_config_to_file()
            .map_or_else(|e| Supported(Err(e)), |_| Supported(Ok(())))
    }

    fn set_start_on_connection(
        &mut self,
        start_on_connection: bool,
    ) -> MaybeUnsupported<Result<(), traits::Error>> {
        self.config.start_on_connection = start_on_connection;
        self.auto_start
            .store(start_on_connection, atomic::Ordering::Relaxed);
        self.write_config_to_file()
            .map_or_else(|e| Supported(Err(e)), |_| Supported(Ok(())))
    }

    fn set_backup_period(
        &mut self,
        backup_period: Option<u32>,
    ) -> MaybeUnsupported<Result<(), traits::Error>> {
        task::block_in_place(|| {
        *self.backup_period.blocking_lock() = backup_period;
        });
        self.config.timeout_no_activity = backup_period;
        self.write_config_to_file()
            .map_or_else(|e| Supported(Err(e)), |_| Supported(Ok(())))
    }

    fn set_field(&mut self, field: &str, value: String) -> Result<(), Error> {
        task::block_in_place(|| {
        self.settings
            .blocking_lock()
            .insert(field.to_string(), value);
        });
        self.write_properties_to_file()
    }

    fn get_field(&self, field: &str) -> Result<String, Error> {
        task::block_in_place(|| {
        Ok(self
            .settings
            .blocking_lock()
            .get(field)
            .ok_or(Error {
                inner: ErrorInner::FieldNotFound,
                detail: format!("Field {} not found", field),
            })?
            .to_string())
        })
    }

    fn settings(&self) -> Result<HashMap<String, String>, Error> {
        Ok(self.settings.blocking_lock().clone())
    }
}
