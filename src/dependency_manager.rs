use serde_json;

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{ErrorKind};

const STATE_FILE_PATH: &str = "./bin/dependencies.json";

pub enum DependencyManagerError {
    IoError(io::Error),
    SerdeError(serde_json::Error),
    NotFoundError,
}

pub struct DependencyManager {
    registered_paths: Option<HashMap<String, String>>,
}

impl DependencyManager {
    fn new() -> DependencyManager {
        DependencyManager {
            registered_paths: None,
        }
    }

    fn save(&self) -> Result<(), DependencyManagerError> {
        let file = File::create(STATE_FILE_PATH);
        return match file {
            Ok(file) => match serde_json::to_writer(file, &self.registered_paths) {
                Ok(_) => Ok(()),
                Err(e) => Err(SaveError::SerdeError(e))
            },
            Err(e) => Err(SaveError::IoError(e))
        }
    }

    fn load(&mut self) -> Result<(), DependencyManagerError> {
        if let Some(_) = self.registered_paths {
            return Ok(())
        }

        let file = File::open(STATE_FILE_PATH);
        match file {
            Ok(file) => {
                let dependencies: HashMap<String, String> = serde_json::from_reader(file).unwrap();
                self.registered_paths = Option::from(dependencies);
                Ok(())
            }
            Err(error) => return match error.kind() {
                ErrorKind::NotFound => match File::create(STATE_FILE_PATH) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(DependencyManagerError(e)),
                },
                other_error => {
                    Err(DependencyManagerError(io::Error::from(other_error)))
                }
            }
        }
    }

    pub fn register(&mut self, name: String, path: String) -> Result<(), DependencyManagerError> {
        self.load()?;

        match &self.registered_paths {
            Some(mut hashMap) => hashMap.insert(name, path),
            None => ()
        }
        self.save()
    }

    pub fn get(&mut self, name: String) -> Result<&String, E> {
        self.load()?;

        match self.registered_paths.get((&name).as_ref()) {
            Some(path) => Ok(path),
            None => Err(DependencyManagerError::NotFoundError),
        }
    }
}
