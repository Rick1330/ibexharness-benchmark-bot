use thiserror::Error;

#[derive(Debug, Error)]
pub enum BotError {
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, BotError>;

pub fn bot_err(message: impl Into<String>) -> BotError {
    BotError::Message(message.into())
}
