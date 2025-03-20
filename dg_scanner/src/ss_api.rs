use std::io::{BufWriter, Write, BufReader};
use std::fs::File;
use std::path::Path;
use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use polars::prelude::*;

const API_GALAXY_URL: &str = "https://www.starsonata.com/webapi/galaxies/v1";

// https://stackoverflow.com/questions/73167416/creating-polars-dataframe-from-vecstruct
macro_rules! struct_to_dataframe {
    ($input:expr, [$($field:ident),+]) => {
        {
            let len = $input.len().to_owned();

            // Extract the field values into separate vectors
            $(let mut $field = Vec::with_capacity(len);)*

            for e in $input.into_iter() {
                $($field.push(e.$field);)*
            }
            df! {
                $(stringify!($field) => $field,)*
            }
        }
    };
}

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
fn download_sanitize_galaxy_data(path: &str) -> DataFrame {
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

    struct_to_dataframe!(galaxy_data, [name, layer, df]).expect("Unable to convert struct data to dataframe.")
}

fn load_galaxy_data(path: &str) -> DataFrame {
    let file = File::open(path).expect("Unable to open galaxy data file.");
    let reader = BufReader::new(file);

    let data: Vec<GalaxyData> = serde_json::from_reader(reader).expect("Unable to read galaxy data file.");
    struct_to_dataframe!(data, [name, layer, df]).expect("Unable to convert struct data to dataframe.")
}

pub fn get_galaxy_data(path: &str) -> DataFrame {
    if Path::new(path).exists() {
        load_galaxy_data(path)
    } else {
        download_sanitize_galaxy_data(path)
    }
}
