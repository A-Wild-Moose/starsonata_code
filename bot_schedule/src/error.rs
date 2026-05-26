use thiserror::Error;


#[derive(Error, Debug)]
pub enum BotError {
    #[error("Missing message information")]
    MissingMessageError(),
    #[error("Message id {0} not found")]
    MessageNotFoundError(String),
}