extern crate serde_json;

use std::fs::File;
use std::collections::HashMap;
use std::io::ErrorKind;

const STATE_FILE_PATH: &str = "./bin/dependencies.json";

pub struct DependencyManager {
    registered_paths: HashMap<String, String>,
}

impl DependencyManager {
    fn new() -> DependencyManager {
        let mut d = DependencyManager {
            registered_paths: HashMap::new(),
        };
        d.load();
        d
    }

    fn save(&self) {
        let file = File::create(STATE_FILE_PATH);
        match file {
            Ok(file) => {
                serde_json::to_writer(file, &self.registered_paths).unwrap();
            }
            Err(_) => {
                println!("Failed to save dependencies");
            }
        }
    }

    fn load(&mut self) {
        let file = File::open(STATE_FILE_PATH);
        match file {
            Ok(file) => {
                let dependencies: HashMap<String, String> = serde_json::from_reader(file).unwrap();
                self.registered_paths = dependencies;
            }
            Err(error) => {
                match error.kind() {
                    ErrorKind::NotFound => {
                        match File::create(STATE_FILE_PATH) {
                            Ok(_) => {},
                            Err(e) => panic!("Problem creating the file: {:?}", e),
                        }
                    }
                    other_error => {
                        panic!("Problem opening the file: {:?}", other_error);
                    }
                }
            }
        }
    }

    pub fn register(&mut self, name: String, path: String) {
        self.registered_paths.insert(name, path);
        self.save();
    }

    pub fn get(&self, name: String) -> &String {
        match self.registered_paths.get((&name).as_ref()) {
            Some(path) => path,
            None => panic!("Dependency {} not found", name)
        }
    }
}