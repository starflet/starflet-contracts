use cosmwasm_std::{Addr, Deps, DepsMut, StdResult};
use cw_storage_plus::Item;

pub const ROUTER: Item<Addr> = Item::new("router");

pub fn get_router(deps: Deps) -> StdResult<Addr> {
    ROUTER.load(deps.storage)
}

pub fn set_router(deps: DepsMut, router: Addr) -> StdResult<()> {
    ROUTER.save(deps.storage, &router)
}
