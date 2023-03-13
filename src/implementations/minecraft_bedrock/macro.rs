use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

use async_trait::async_trait;
use color_eyre::eyre::eyre;
use deno_core::{
    anyhow::{self},
    op, OpState,
};

use crate::{
    error::Error,
    events::{CausedBy, EventInner},
    macro_executor::{self, MainWorkerGenerator},
    traits::{t_macro::TMacro, t_server::TServer},
    util::list_dir,
};

use super::MinecraftBedrockInstance;

#[async_trait]
impl TMacro for MinecraftBedrockInstance {
    async fn get_macro_list(&self) -> Vec<String> {
        Vec::new()
    }

    async fn delete_macro(&mut self, name: &str) -> Result<(), Error> {
        Ok(())
    }

    async fn create_macro(&mut self, name: &str, content: &str) -> Result<(), Error> {
        Ok(())
    }

    async fn run_macro(
        &mut self,
        name: &str,
        args: Vec<String>,
        caused_by: CausedBy,
        is_in_game: bool,
    ) -> Result<(), Error> {
        Ok(())
    }
}