use async_trait::async_trait;

use crate::{error::Error, traits::t_resource::TResourceManagement};

use super::MinecraftBedrockInstance;

#[async_trait]
impl TResourceManagement for MinecraftBedrockInstance { }