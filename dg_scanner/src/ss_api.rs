use std::io::{BufWriter, Write, BufReader};
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;

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
    id: i64,
    df: f64,
    #[serde(alias="lastUpdate")]
    last_update: i64,
    layer: i64,
    links: Vec<i64>,
    mapable: Option<bool>,
    name: String,
    x: f64,
    y: f64,
}

fn download_sanitize_galaxy_data(path: &str) -> Vec<GalaxyData> {
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

    galaxy_data
}

fn load_galaxy_data(path: &str) -> Vec<GalaxyData> {
    let file = File::open(path).expect("Unable to open galaxy data file.");
    let reader = BufReader::new(file);

    let data: Vec<GalaxyData> = serde_json::from_reader(reader).expect("Unable to read galaxy data file.");
    data
}

pub fn get_galaxy_data(path: &str) -> Vec<GalaxyData> {
    if Path::new(path).exists() {
        load_galaxy_data(path)
    } else {
        download_sanitize_galaxy_data(path)
    }
}