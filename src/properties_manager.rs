use std::{collections::HashMap, path::Path, fs::File, io::BufReader};
use std::io::{self, prelude::*, LineWriter};
use std::result::Result;
use regex::Regex;
pub struct PropertiesManager {
    properties : HashMap<String, String>,
    path_to_properties : String
}

impl PropertiesManager {
    pub fn new(path : String) -> Result<PropertiesManager, String> {
        if !Path::new(path.as_str()).exists() {
            return Err("server.properties not found".to_string());
        }
        let file = File::open(path.as_str()).unwrap();
        let buf_reader = BufReader::new(file);
        let mut properties = HashMap::new();
        for line in buf_reader.lines() {
            let res: Vec<String> = line.unwrap().split("=").map(|s| s.to_string()).collect();
            properties.insert(res.get(0).unwrap().clone(), res.get(1).unwrap().clone());
        }
        Ok(PropertiesManager {
           properties,
           path_to_properties : path,
        })
    }

    pub fn edit_field(&mut self, field : String, value : String) -> Result<(), String> {
        *self.properties.get_mut(&field).ok_or("property does not exist".to_string()).unwrap() = value;
        Ok(())
    }

    pub fn write_to_file(self) -> Result<(), String> {
        let file = File::create(self.path_to_properties.as_str()).map_err(|e| e.to_string())?;
        let mut line_writer = LineWriter::new(file);
        for entry in self.properties {
            line_writer.write_all(format!("{}={}\n", entry.0, entry.1).as_bytes()).unwrap();
        }
        line_writer.flush().unwrap();
        Ok(())
    }

}
