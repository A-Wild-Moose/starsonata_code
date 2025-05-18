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


fn get_subset_galaxy_data(path: &str, galaxies: Series) -> (DataFrame, DataFrame) {
    let galaxy_data = get_galaxy_data(path);

    let data_subset = galaxy_data
        .clone()
        .lazy()
        .filter(
            col("name").str().starts_with(lit("DG "))
        )
        .with_columns([
            col("name")
                .str()
                .strip_prefix(lit("DG "))
                .str()
                .replace(lit(" [0-9]{1,2}\\.[0-9]{1,4}[A-D]?"), lit(""), false)
                .alias("galaxy")
        ])
        .filter(
            col("galaxy").is_in(lit(galaxies))
        )
        .collect()
        .unwrap();

        (galaxy_data, data_subset)
}


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
    // data = data.sort(
    //     ["id", "room"],
    //     SortMultipleOptions::default()
    //         .with_order_descending_multi([false, true])
    // ).unwrap();

    // get the galaxies that are mapped. This may include some DGs that haven't been mapped but
    // are in a galaxy with multiple dgs. Accepting this for now
    let galaxy_series = data
        .column("galaxy")
        .unwrap()
        .as_materialized_series()
        .clone()
        .unique()
        .unwrap();

    let (full_galaxy_data, dg_galaxy_data) = get_subset_galaxy_data("raw/api_galaxy_data.json", galaxy_series);

    // lots of instructions here to create the proper offset for sorting rooms neatly with no gaps
    data = data
        .lazy()
        .with_columns([(lit("DG ") + col("name")).alias("proper_name")])
        .join(
            dg_galaxy_data.clone().lazy().select([cols(["name", "galaxy", "links", "layer"])]),
            [col("proper_name"), col("galaxy")],
            [col("name"), col("galaxy")],
            JoinArgs::new(JoinType::Full)
                .with_coalesce(JoinCoalesce::CoalesceColumns)  // combines columns in the left/right join by
        )
        // re-create some values when we are filling in missing DG levels
        .with_columns([
            when(is_null(col("name")))
                .then(col("proper_name").str().strip_prefix(lit("DG ")))
                .otherwise(col("name"))
                .alias("name"),
            when(is_null(col("level")))
                .then(col("proper_name").str().extract(lit("[0-9]{1,2}\\.[0-9]{1,4}[A-D]?"), 0))
                .otherwise(col("level"))
                .alias("level")
        ])
        .with_columns([
            when(is_null(col("id")))
                .then(col("level").str().split(lit(".")).list().last())
                .otherwise(col("id"))
                .alias("id"),
            when(is_null(col("room")))
                .then(col("level").str().extract(lit("[0-9]{1,2}"), 0))
                .otherwise(col("room"))
                .cast(DataType::Int64)
                .alias("room")
        ])
        .sort(
            ["id", "room"],
            SortMultipleOptions::default()
                .with_order_descending_multi([false, true])
        )
        // now back to handling rooms etc
        .with_columns([
            // create a clean id for the entire DG
            col("id").str().extract(lit("[0-9]*"), 0).alias("clean_id"),
            // add an A to the ID so that boss propagates all the way as necessary
            when(not(col("id").str().contains(lit("[A-Z]"), true)))
                .then(col("id") + lit("A"))
                .otherwise(col("id")),
        ])
        .with_columns([
            // get the parent galaxy
            // TODO verify that this is always the first in the list
            col("links").list().first().alias("parent_id"),
            // boss to all the rooms
            col("boss").last().over(["galaxy", "id"])
        ])
        .select([all().exclude(["links"])])
        .join(
            full_galaxy_data.lazy().select([col("id"), col("name"), col("df")]).rename(["name"], ["parent_name"], true),
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
                .alias("parent_room"),
            // make the DF calculation update so that it represents what players see
            col("df") * lit(10)
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
            col("df") * lit(10)  // *100 / 10 for first level of DG being 10x higher than the connecting galaxy
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
