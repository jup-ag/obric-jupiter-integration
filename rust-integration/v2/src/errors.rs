use thiserror::Error;

#[derive(Error, Debug)]
pub enum AmmError {
    #[error("Account not found")]
    AccountNotFound,
}