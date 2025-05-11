use std::io::{BufWriter, Write, BufReader};
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;
use polars::prelude::*;

use serde::{Serialize, Deserialize};

const API_GALAXY_URL: &str = "https://www.starsonata.com/webapi/galaxies/v1";

#[derive(Serialize, Deserialize, Debug)]
struct Payload {
    api: ApiData,
    galaxies: UniverseData,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiData {
    cache_expire: i64,
    cache_hit: bool,
    format: String,
    runtime: f64,
    version: String

}

#[derive(Serialize, Deserialize, Debug)]
struct UniverseData {
    #[serde(flatten)]
    galaxies: HashMap<String, GalaxyData>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GalaxyData {
    #[serde(alias="ID")]
    pub id: i64,
    pub df: f64,
    #[serde(alias="lastUpdate")]
    pub last_update: i64,
    pub layer: i64,
    pub links: Vec<i64>,
    pub mapable: Option<bool>,
    pub name: String,
    pub x: f64,
    pub y: f64,
}

// NOTE: the two below functions first convert to json and then convert to a DF
// for now this is intentional as the rest of the galaxy API data may be useful in the future
fn download_galaxy_data(path: &str) {
    let url = reqwest::Url::parse(&*API_GALAXY_URL).unwrap();
    let res = reqwest::blocking::get(url).unwrap().text().unwrap();

    let data: Payload = serde_json::from_str(&res).unwrap();
    // get just the data we are interested in - just the galaxy data
    // since this is a hashmap, get just the values, clone so that we own the data, and collect into a vector
    let galaxy_data: Vec<GalaxyData> = data.galaxies.galaxies.values().cloned().collect();

    let file = File::create(path).expect("Could not create galaxy data file.");
    let mut writer = BufWriter::new(file);

    let _ = serde_json::to_writer_pretty(&mut writer, &galaxy_data);
    writer.flush().expect("Unable to write galaxy data to file");
}

fn load_galaxy_data(path: &str) -> DataFrame {
    let mut file = std::fs::File::open("raw/api_galaxy_data.json").unwrap(); 
    JsonReader::new(&mut file).finish().unwrap()
}

pub fn get_galaxy_data(path: &str) -> DataFrame {
    if !Path::new(path).exists() {
        download_galaxy_data(path);
    }
    load_galaxy_data(path)
}
