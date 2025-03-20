use dg_scanner::ss_api::get_galaxy_data;

fn main() {
    let gdata = get_galaxy_data("raw/api_galaxy_data.json");

    println!("{:?}", &gdata[..5]);
}