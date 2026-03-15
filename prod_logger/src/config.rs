use serde::Deserialize;
use secrecy::SecretBox;


#[derive(serde::Deserialize, Debug)]
pub struct StarSonataAccount {
    pub username: String,
    pub password: SecretBox<String>
}

#[derive(Deserialize, Debug)]
pub struct StarSonataStartup {
    pub ss_path: String,
    pub initial_sleep: u64,
    pub client_load_sleep: u64,
    pub character_load_sleep: u64,
}


#[derive(Deserialize, Debug)]
pub struct AppConfig {
    pub starsonata_account: StarSonataAccount,
    pub starsonata_startup: StarSonataStartup,
}

