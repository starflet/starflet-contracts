use cosmwasm_std::{Addr, Deps, DepsMut, StdResult, Uint128};
use cw_storage_plus::Item;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::AssetInfo;

pub const ROUTER: Item<Addr> = Item::new("router");

pub fn get_router(deps: Deps) -> StdResult<Addr> {
    ROUTER.load(deps.storage)
}

pub fn set_router(deps: DepsMut, router: Addr) -> StdResult<()> {
    ROUTER.save(deps.storage, &router)
}

pub const DEPOSIT_ASSET_INFO: Item<AssetInfo> = Item::new("bond_asset");

pub fn get_deposit_asset_info(deps: Deps) -> StdResult<AssetInfo> {
    DEPOSIT_ASSET_INFO.load(deps.storage)
}

pub fn set_deposit_asset_info(deps: DepsMut, asset_info: AssetInfo) -> StdResult<()> {
    DEPOSIT_ASSET_INFO.save(deps.storage, &asset_info)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AnchorInfo {
    pub market_money: Addr,
    pub aust: AssetInfo,
}

pub const ANCHOR_INFO: Item<AnchorInfo> = Item::new("anchor_info");

pub fn get_anchor_info(deps: Deps) -> StdResult<AnchorInfo> {
    ANCHOR_INFO.load(deps.storage)
}

pub fn set_anchor_info(deps: DepsMut, market_money: Addr, aust: AssetInfo) -> StdResult<()> {
    ANCHOR_INFO.save(deps.storage, &AnchorInfo { market_money, aust })
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TmpSwap {
    pub route_path: String,
    pub minimum_receive: Uint128,
}

pub const TMP_SWAP: Item<TmpSwap> = Item::new("tmp_swap");
pub fn get_tmp_swap(deps: Deps) -> StdResult<TmpSwap> {
    TMP_SWAP.load(deps.storage)
}

pub fn set_tmp_swap(deps: DepsMut, route_path: String, minimum_receive: Uint128) -> StdResult<()> {
    TMP_SWAP.save(
        deps.storage,
        &TmpSwap {
            route_path,
            minimum_receive,
        },
    )
}

pub fn remove_tmp_swap(deps: DepsMut) {
    TMP_SWAP.remove(deps.storage)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TmpBonder {
    pub bonder: Addr,
    pub prev_amount: Uint128,
}

pub const TMP_BONDER: Item<TmpBonder> = Item::new("tmp_bonder");

pub fn get_tmp_bonder(deps: Deps) -> StdResult<TmpBonder> {
    TMP_BONDER.load(deps.storage)
}

pub fn set_tmp_bonder(deps: DepsMut, bonder: Addr, prev_amount: Uint128) -> StdResult<()> {
    TMP_BONDER.save(
        deps.storage,
        &TmpBonder {
            bonder,
            prev_amount,
        },
    )
}
