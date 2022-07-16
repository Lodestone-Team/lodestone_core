

use std::iter::Iterator;
pub enum ServerOperationError {
    AlreadyStarting,
    AlreadyRunning,
    AlreadyStopping,
    NotRunning,
}

pub enum State {
    Starting,
    Running,
    Stopping,
    Stopped,
}

pub enum StdinOperationError {
    NotOpen,
    FailedToWrite,
    FailedToAquireLock,
}

pub trait TServer {
    fn start(&mut self) -> Result<(), ServerOperationError>;
    fn stop(&mut self) -> Result<(), ServerOperationError>;
    fn state(&self) -> State;
    fn send_stdin(&self, command: &str) -> Result<(), StdinOperationError>;
    fn get_stdout(&self) -> Box<dyn Iterator<Item = String>>;
}