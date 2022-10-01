use serde_json::json;
use tokio::task;

use crate::traits::t_configurable::TConfigurable;
use crate::traits::t_player::TPlayerManagement;
use crate::traits::Supported;

use super::Instance;

impl TPlayerManagement for Instance {
    fn get_player_count(&self) -> crate::traits::MaybeUnsupported<u32> {
        task::block_in_place(|| Supported(self.players.blocking_lock().get_ref().len() as u32))
    }

    fn get_max_player_count(&self) -> crate::traits::MaybeUnsupported<u32> {
        Supported(
            self.get_field("max-players")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .expect("Failed to parse max-players"),
        )
    }

    fn get_player_list(&self) -> crate::traits::MaybeUnsupported<Vec<serde_json::Value>> {
        task::block_in_place(|| {
            Supported(
                self.players
                    .blocking_lock()
                    .get_ref()
                    .iter()
                    .map(|name| json!({ "name": name }))
                    .collect(),
            )
        })
    }

    fn set_max_player_count(
        &mut self,
        _max_player_count: u32,
    ) -> crate::traits::MaybeUnsupported<()> {
        todo!()
    }
}
