use async_trait::async_trait;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::traits::t_player::Player;
use crate::traits::t_player::{TPlayer, TPlayerManagement};
use crate::Error;

use super::MinecraftBedrockInstance;

#[async_trait]
impl TPlayerManagement for MinecraftBedrockInstance { }