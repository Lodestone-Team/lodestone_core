pub mod t_server;
pub mod t_configurable;
pub mod t_player;
pub mod t_resource;

pub enum MaybeUnsupported<T> {
    Supported(T),
    Unsupported,
}