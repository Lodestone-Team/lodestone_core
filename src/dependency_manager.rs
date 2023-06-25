use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{ErrorKind};

pub enum DependencyManagerError {
    Io(io::Error),
    Serde(serde_json::Error),
    NotFound,
}

pub struct DependencyManager {
    registered_paths: Option<HashMap<String, String>>,
    file_path: String,
}

impl DependencyManager {
    fn new(file_path: &str) -> DependencyManager {
        DependencyManager {
            registered_paths: None,
            file_path: String::from(file_path),
        }
    }

    fn save(&self) -> Result<(), DependencyManagerError> {
        let file = File::create(&self.file_path);
        match file {
            Ok(file) => match serde_json::to_writer(file, &self.registered_paths) {
                Ok(_) => Ok(()),
                Err(e) => Err(DependencyManagerError::Serde(e))
            },
            Err(e) => Err(DependencyManagerError::Io(e))
        }
    }

    fn load(&mut self) -> Result<(), DependencyManagerError> {
        if self.registered_paths.is_some() {
            return Ok(())
        }

        let file = File::open(&self.file_path);
        match file {
            Ok(file) => {
                let dependencies: HashMap<String, String> = serde_json::from_reader(file).unwrap();
                self.registered_paths = Option::from(dependencies);
                Ok(())
            }
            Err(error) => match error.kind() {
                ErrorKind::NotFound => match File::create(&self.file_path) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(DependencyManagerError::Io(e)),
                },
                other_error => {
                    Err(DependencyManagerError::Io(io::Error::from(other_error)))
                }
            }
        }
    }

    pub fn register(&mut self, name: String, path: String) -> Result<(), DependencyManagerError> {
        self.load()?;

        match self.registered_paths {
            Some(ref mut hash_map) => hash_map.insert(name, path),
            None => return Ok(())
        };
        self.save()
    }

    pub fn get(&mut self, name: &String) -> Result<&str, DependencyManagerError> {
        self.load()?;

        match &self.registered_paths {
            Some(hash_map) => match hash_map.get::<String>(name) {
                Some(path) => Ok(path),
                None => Err(DependencyManagerError::NotFound),
            },
            None => panic!("No registered paths")
        }
    }
}
