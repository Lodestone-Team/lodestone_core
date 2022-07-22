use crate::traits::t_resource::{self, TResourceManagement};

use super::Instance;

impl TResourceManagement for Instance {
}

impl Instance {
    fn list<T>(&self) -> T where T:serde::Serialize{
        String::new()
    }

}
