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


// fn series_diff(a: &Series) -> Series {
//     let mut x = a
//         .into_iter()
//         .map()
// }

fn main() {
    unsafe{
        std::env::set_var("POLARS_FMT_MAX_ROWS", "20");
        std::env::set_var("POLARS_FMT_MAX_COLS", "15");
    }

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
        ["id", "room"],
        SortMultipleOptions::default()
            .with_order_descending_multi([false, true])
    ).unwrap();

    let galaxy_data = get_galaxy_data("raw/api_galaxy_data.json");

    // lots of instructions here to create the proper offset for sorting rooms neatly with no gaps
    data = data
        .lazy()
        .with_columns([
            // create a clean id for the entire DG
            col("id").str().extract(lit("[0-9]*"), 0).alias("clean_id"),
            // add an A to the ID so that boss propagates all the way as necessary
            when(not(col("id").str().contains(lit("[A-Z]"), true)))
                .then(col("id") + lit("A"))
                .otherwise(col("id")),
            (lit("DG ") + col("name")).alias("proper_name")
        ])
        .join(
            galaxy_data.clone().lazy().select([col("name"), col("links")]),
            [col("proper_name")],
            [col("name")],
            JoinArgs::new(JoinType::Left)
        )
        .with_columns([
            // get the parent galaxy
            // TODO verify that this is always the first in the list
            col("links").list().first().alias("parent_id"),
            // boss to all the rooms
            col("boss").last().over(["galaxy", "id"])
        ])
        .select([all().exclude(["links"])])
        .join(
            galaxy_data.lazy().select([col("id"), col("name"), col("df"), col("layer")]).rename(["name"], ["parent_name"], true),
            [col("parent_id")],
            [col("id")],
            JoinArgs::new(JoinType::Left)
        )
        .with_columns([
            // Get the level of the parent galaxy for dg levels. Need 2 steps here because 
            // of numbers in galaxy names, etc.
            col("parent_name")
                .str()
                .extract(lit("[0-9]{1,2}\\.[0-9]"), 0)
                .str()
                .extract(lit("[0-9]{1,2}"), 0)
                .cast(DataType::Int64)
                .alias("parent_room")
        ])
        .with_columns([
            // fill nulls in the parent room
            when(is_null(col("parent_room")))
                .then(col("room") + lit(1))
                .otherwise(col("parent_room"))
                .alias("parent_room")
        ])
        .with_columns([
            // first offset
            (col("parent_room") - col("room") - lit(1))
                .cum_sum(false)
                .over(["galaxy", "id"])
                .alias("offset")
        ])
        .with_columns([
            (col("room").max().over(["galaxy", "clean_id"]) - col("room") + lit(1) - col("offset")).alias("room_asc")
        ])
        // need to collect here, as we need some of the data to get the offsets offset
        // for splits in galaxies
        .collect()
        .unwrap();
    
    // get a subset of the data we can use to merge in to get offsets of parents 
    // where necessary
    let subset = data
        .clone()
        .lazy()
        .select([
            cols(["name", "proper_name", "parent_name", "offset"])
        ])
        .unique(None, UniqueKeepStrategy::First)
        .rename(["offset"], ["offset2"], true)
        .collect()
        .unwrap();
    
    // back to merging this in with the data
    data = data
        .lazy()
        .join(
            subset.lazy(),
            [col("parent_name")],
            [col("proper_name")],
            JoinArgs::new(JoinType::Left)
        )
        .with_columns([
            col("offset2")
                .fill_null(lit(0))
                .first()
                .over(["galaxy", "id"])
        ])
        .with_columns([
            col("room_asc") - col("offset2"),
            col("name").first().over(["galaxy", "clean_id"]),
            col("df").first().over(["galaxy", "clean_id"])
        ])
        .collect()
        .unwrap();
    
    // pivot to the wider view for saving
    let mut data_wide = pivot_stable(&data, ["room_asc"], Some(["name", "id", "df", "layer", "boss"]), Some(["guard"]), false, None, None).unwrap();
    // join with the layer information
    data_wide = data_wide
        .lazy()
        .join(
            layer_map.lazy(),
            [col("layer")],
            [col("layer")],
            JoinArgs::new(JoinType::Left)
        )
        .with_columns([
            col("df") * lit(100)
        ])
        .select([
            cols(["layer_name", "name", "id", "df", "boss"]),
            all().exclude(["layer_name", "name", "id", "df", "boss"])
        ])
        .sort(
            ["df", "name", "id"],
            SortMultipleOptions::default()
                .with_order_descending_multi([false, false, false])
        )
        .with_columns([
            int_range(lit(0), len(), 1, DataType::Int64).over(["name"]).alias("_range")
        ])
        .with_columns([
            when(col("_range").eq(0))
                .then(col("name"))
                .otherwise(lit(NULL))
                .alias("name")
        ])
        .select([
            all().exclude(["_range"])
        ])
        .collect()
        .unwrap();

    let path = Path::new("raw/dgs.csv");
    create_dir_all(path.parent().unwrap()).unwrap();
    let mut file = File::create(path).expect("Could not create csv file.");

    let _ = CsvWriter::new(&mut file)
        .include_header(true)
        .with_separator(b',')
        .finish(&mut data_wide);
}








































fn main2() {
    unsafe{
        std::env::set_var("POLARS_FMT_MAX_ROWS", "20");
        std::env::set_var("POLARS_FMT_MAX_COLS", "15");
    }

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
        ["id", "room"],
        SortMultipleOptions::default()
            .with_order_descending_multi([false, true])
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
    let mut file = std::fs::File::open("raw/api_galaxy_data.json").unwrap(); 
    let galaxy_data = JsonReader::new(&mut file).finish().unwrap();

    let mut subset = data
        .clone()
        .lazy()
        .with_columns([
            col("id").str().extract(lit("[0-9]*"), 0).alias("clean_id"),
            when(not(col("id").str().contains(lit("[A-Z]"), true)))
            .then(col("id") + lit("A"))
            .otherwise(col("id"))
        ])
        .filter(
            col("galaxy").eq(lit("Xanarkand"))
            // col("galaxy").eq(lit("Ankanja"))
        )
        .filter(
            col("id").str().contains(lit("983"), true)
            // col("id").str().contains(lit("989"), true)
            // col("id").str().contains(lit("1064"), true)
        )
        .with_columns([
            (lit("DG ") + col("name")).alias("proper_name")
        ])
        .join(
            galaxy_data.clone().lazy().select([col("name"), col("links")]),
            [col("proper_name")],
            [col("name")],
            JoinArgs::new(JoinType::Left)
        )
        .collect()
        .unwrap();
    
    subset = subset
        .lazy()
        .with_columns([
            col("links").list().first().alias("parent_id"),
        ])
        .select([col(PlSmallStr::from_static("*")).exclude(["links"])])
        .collect().
        unwrap();
    
    subset = subset
        .lazy()
        .join(
            galaxy_data.lazy().select([col("id"), col("name")]).rename(["name"], ["parent_name"], true),
            [col("parent_id")],
            [col("id")],
            JoinArgs::new(JoinType::Left)
        )
        .with_columns([
            col("parent_name").str().extract(lit("[0-9]{1,2}\\.[0-9]{1,4}"), 0).alias("parent_level")
        ])
        .with_columns([
            col("parent_level").str().extract(lit("[0-9]{1,2}"), 0).cast(DataType::Int64).alias("parent_room")
        ])
        .with_columns([
            when(is_null(col("parent_room")))
            .then(col("room") + lit(1))
            .otherwise(col("parent_room")).alias("parent_room")
        ])
        .with_columns([
            (col("parent_room") - col("room") - lit(1)).alias("offset")
        ])
        .with_columns([
            col("offset").cum_sum(false).over(["galaxy", "id"]).alias("offset2")
        ])
        .with_columns([
            (col("room").max().over(["galaxy", "clean_id"]) - col("room") + lit(1) - col("offset2")).alias("room_asc")
        ])
        .collect()
        .unwrap();

    let mut subset2 = subset
        .clone()    
        .lazy()
        .select([cols(["name", "proper_name", "parent_name", "offset2"])])
        .unique(None, UniqueKeepStrategy::First)
        .rename(["offset2"], ["offset3"], true)
        .collect()
        .unwrap();
    
    println!("{:?}", subset2);

    subset = subset
        .lazy()
        .join(
            subset2.lazy(),
            [col("parent_name")],
            [col("proper_name")],
            JoinArgs::new(JoinType::Left)
        )
        .with_columns([
            col("offset3").fill_null(lit(0))
        ])
        .with_columns([
            col("offset3").first().over(["galaxy", "id"])
        ])
        .with_columns([
            col("room_asc") - col("offset3")
        ])
        .collect()
        .unwrap();
    
    println!("{:?}", subset);
}