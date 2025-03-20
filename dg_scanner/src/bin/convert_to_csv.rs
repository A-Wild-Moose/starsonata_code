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

fn main() {
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

    let path = Path::new("raw/dgs.csv");
    create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = File::create(path).expect("Could not create csv file.");

    let _ = CsvWriter::new(&mut file)
        .include_header(true)
        .with_separator(b',')
        .finish(&mut data_wide);
}