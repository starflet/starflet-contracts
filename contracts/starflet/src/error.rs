use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
    #[error("Invalid reply ID")]
    InvalidReplyId {},

    #[error("Fail to bond")]
    FailBond {},

    #[error("Fail to unbond")]
    FailUnbond {},

    #[error("Amount does not match after bond and unbond. (expect {0}, result {1})")]
    FailBondAndUnbond(Uint128, Uint128),
}
