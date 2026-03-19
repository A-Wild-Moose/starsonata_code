use serde::Deserialize;
use secrecy::SecretBox;
use poise::serenity_prelude::ChannelId;

#[derive(Deserialize, Debug)]
pub struct DiscordConfig {
    pub bot_token: String,
    pub prod_log_channel_id: ChannelId,
}
#[derive(Deserialize, Debug)]
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
    pub discord: DiscordConfig,
    pub starsonata_account: StarSonataAccount,
    pub starsonata_startup: StarSonataStartup,
}

