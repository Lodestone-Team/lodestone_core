use color_eyre::eyre::{eyre, Context, ContextCompat};
use indexmap::IndexMap;
use serde_json::{self, Value};
use std::{collections::BTreeMap, path::Path, str::FromStr};
use tokio::io::AsyncBufReadExt;

use crate::error::Error;


pub async fn read_properties_from_path(
    path_to_properties: &Path,
) -> Result<IndexMap<String, String>, Error> {
    let properties_file = tokio::fs::File::open(path_to_properties)
        .await
        .context(format!(
            "Failed to open properties file at {}",
            path_to_properties.display()
        ))?;
    let buf_reader = tokio::io::BufReader::new(properties_file);
    let mut stream = buf_reader.lines();
    let mut ret = IndexMap::new();

    while let Some(line) = stream
        .next_line()
        .await
        .context("Failed to read line from properties file")?
    {
        // if a line starts with '#', it is a comment, skip it
        if line.starts_with('#') {
            continue;
        }
        // split the line into key and value
        let mut split = line.split('=');
        let key = split
            .next()
            .ok_or_else(|| eyre!("Failed to read key from properties file"))?
            .trim();
        let value = split
            .next()
            .ok_or_else(|| eyre!("Failed to read value from properties file for key {}", key))?
            .trim();

        ret.insert(key.to_string(), value.to_string());
    }
    Ok(ret)
}

// Returns the jar url and the updated flavour with version information
pub async fn get_server_zip_url(version: &str) -> Option<String> {
    Some(format!("https://minecraft.azureedge.net/bin-win/bedrock-server-{version}.zip"))
}

pub async fn get_minecraft_bedrock_version() -> Result<String, Error> {
    Ok(String::from("1.19.71.02"))
}