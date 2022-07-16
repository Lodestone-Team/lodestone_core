use std::collections::HashMap;

use serde::Serialize;

use super::MaybeUnsupported;

pub enum Resource<T>
where
    T: Serialize,
{
    Mod(Vec<T>),
    World(Vec<T>),
}

pub trait TResourceManagement {
    fn list<T>(&self) -> MaybeUnsupported<Resource<T>>
    where
        T: Serialize,
    {
        MaybeUnsupported::Unsupported
    }

    fn load(&mut self, resource: &str) -> MaybeUnsupported<(Result<(), super::Error>)> {
        MaybeUnsupported::Unsupported
    }

    fn unload(&mut self, resource: &str) -> MaybeUnsupported<(Result<(), super::Error>)> {
        MaybeUnsupported::Unsupported
    }

    fn delete(&mut self, resource: &str) -> MaybeUnsupported<(Result<(), super::Error>)> {
        MaybeUnsupported::Unsupported
    }
}
