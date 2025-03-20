use std::fs::{File, create_dir_all};
use std::path::Path;

// use connectorx::prelude::*;
use connectorx::destinations::arrow::ArrowDestination;
use connectorx::sources::sqlite::SQLiteSource;
use connectorx::transports::SQLiteArrowTransport;
use connectorx::prelude::Dispatcher;
// use polars::prelude::{SerWriter, CsvWriter, pivot::pivot_stable};
use polars::prelude::*;
use polars::prelude::pivot::pivot_stable;
use polars::chunked_array::ops::SortMultipleOptions;

use dg_scanner::ss_api::get_galaxy_data;

fn main() {
    let layer_map: DataFrame = df!(
        "layer" => [0i64, 3i64, 4i64, 6i64],
        "layer_name" => ["EF", "WS", "Perilous", "KD"]
    ).unwrap();

    let mut dest = ArrowDestination::new();
    let source = SQLiteSource::new("raw/dgs.db3", 10).expect("cannont create source");
    let queries = &["SELECT * FROM DgData"];
    let dispatcher = Dispatcher::<SQLiteSource, ArrowDestination, SQLiteArrowTransport>::new(source, &mut dest, queries, None);
    dispatcher.run().expect("run failed");

    let mut data = dest.polars().unwrap();
    data = data.sort(
        ["room"],
        SortMultipleOptions::default()
            .with_order_descending(true)
    ).unwrap();

    // fill boss back in for previous levels
    data = data
        .lazy()
        .with_columns([
            col("boss").last().over(["galaxy", "id"]),
        ])
        .collect()
        .unwrap();

    let mut data_wide = pivot_stable(&data, ["room"], Some(["galaxy", "id", "boss"]), Some(["guard"]), false, None, None).unwrap();

    // now sort the galaxy/id
    data_wide = data_wide.sort(
        ["galaxy", "id"],
        SortMultipleOptions::default()
            .with_order_descending_multi([false, false])
    ).unwrap();

    // get the galaxy information from the SS API
    let galaxy_data = get_galaxy_data("raw/api_galaxy_data.json");

    // get the layer/df information
    data_wide = data_wide
        .lazy()
        .join(
            galaxy_data.lazy(),
            [col("galaxy")],
            [col("name")],
            JoinArgs::new(JoinType::Left)
        )
        .join(
            layer_map.lazy(),
            [col("layer")],
            [col("layer")],
            JoinArgs::new(JoinType::Left)
        )
        .with_columns([
            col("df") * lit(100)
        ])
        .select(
            [
                cols(["layer_name", "galaxy", "id", "df", "boss"]),
                all().exclude(["layer_name", "galaxy", "id", "df", "boss", "layer"])
            ]
        )
        .collect()
        .expect("Unable to join DG data with Galaxy data.");

    let path = Path::new("raw/dgs.csv");
    create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = File::create(path).expect("Could not create csv file.");

    let _ = CsvWriter::new(&mut file)
        .include_header(true)
        .with_separator(b',')
        .finish(&mut data_wide);
}