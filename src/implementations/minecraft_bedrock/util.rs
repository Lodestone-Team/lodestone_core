use color_eyre::eyre::{eyre, Context, ContextCompat};
use indexmap::IndexMap;
use serde_json::{self, Value};
use std::{collections::BTreeMap, path::Path, str::FromStr};
use tokio::io::AsyncBufReadExt;
use reqwest;
use scraper::{Html, Selector};

use crate::error::Error;


pub(super) async fn read_properties_from_path(
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

pub(super) async fn get_latest_zip_url() -> Result<String, Error> {
    let html_doc = reqwest::get("https://www.minecraft.net/en-us/download/server/bedrock/")
        .await
        .map_err(|_| eyre!("Failed to fetch the bedrock server html"))?
        .text()
        .await
        .unwrap();

    let html = Html::parse_document(&html_doc);

    let link_selector = Selector::parse("a.downloadlink[data-platform=serverBedrockWindows]").unwrap();
    let href_attr = "href";
    let link = html.select(&link_selector).next().unwrap();

    let href = link.value().attr(href_attr).unwrap();

    let url = reqwest::Url::parse(href).unwrap();

    Ok(url.to_string())
}


#[test]
fn test_get_latest() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let url = rt.block_on(get_latest_zip_url()).unwrap();
    println!("{}", url);
}