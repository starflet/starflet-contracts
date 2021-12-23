use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid reply ID")]
    InvalidReplyId {},

    #[error("Invalid request: \"unbond\" message not included in request")]
    MissingUnbondHook {},

    #[error("Already instantiate contract {0}")]
    AlreadyInstantiate(String),
}
