use crate::traits::{
    t_configurable::TConfigurable,
    t_manifest::{Manifest, Operation, TManifest},
};

use super::Instance;

impl TManifest for Instance {
    fn get_manifest(&self) -> Manifest {
        Manifest {
            supported_operations: Operation::all(),
            settings: self.settings().unwrap().keys().cloned().collect(),
        }
    }
}
