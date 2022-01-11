use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Deps, DepsMut, StdResult};
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Contracts {
    pub pair: Addr,
}

pub const CONTRACTS: Item<Contracts> = Item::new("contracts");

pub fn get_contracts(deps: Deps) -> StdResult<Contracts> {
    CONTRACTS.load(deps.storage)
}

pub fn set_contracts(deps: DepsMut, contracts: Contracts) -> StdResult<()> {
    CONTRACTS.save(deps.storage, &contracts)
}
