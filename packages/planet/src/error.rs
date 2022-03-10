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

    #[error("Invalid request: \"bond\", \"unbond\" message not included in request")]
    InvalidHookMsg {},

    #[error("Already instantiate contract {0}")]
    AlreadyInstantiate(String),

    #[error("Fail to parse {0} response")]
    FailedToParse(String),

    #[error("Fail to execute. before {0} after {1}")]
    FailedExecute(String, String),
}
