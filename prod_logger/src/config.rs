use serde::Deserialize;


#[derive(Deserialize, Debug)]
pub struct StarSonata_Account {
    username: String,
    password: SecretBox<String>
}

#[derive(Deserialize, Debug)]
pub struct StarSonata_Startup {
    ss_path: String,
    initial_sleep: u64,
    client_load_sleep: u64,
    character_load_sleep: u64,
}


#[derive(Deserialize, Debug)]
pub struct AppConfig {
    pub starsonata_account: StarSonata_Account,
    pub starsonata_startup: StarSonata_Startup,
}

