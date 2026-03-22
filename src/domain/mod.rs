pub mod agent;
pub mod auth;
pub mod game;
pub mod lobby;
pub mod settlement;

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Invalid action: {0}")]
    InvalidAction(String),
    #[error("Invalid amount: {0}")]
    InvalidAmount(String),
    #[error("Not player's turn")]
    NotYourTurn,
    #[error("Game not in progress")]
    GameNotInProgress,
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("State parse error: {0}")]
    StateParseError(String),
}
