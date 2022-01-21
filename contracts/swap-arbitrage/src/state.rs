use cosmwasm_std::{Addr, Deps, DepsMut, Order, StdResult};
use cw_storage_plus::{Bound, Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PAIRS: Map<String, Addr> = Map::new("pairs");

pub fn get_pair(deps: Deps, name: String) -> StdResult<Addr> {
    PAIRS.load(deps.storage, name)
}

pub fn set_pair(deps: DepsMut, name: String, pair: Addr) -> StdResult<()> {
    PAIRS.save(deps.storage, name, &pair)
}

pub fn remove_pair(deps: DepsMut, name: String) {
    PAIRS.remove(deps.storage, name)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PairInfo {
    pub name: String,
    pub pair_addr: Addr,
}

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn load_pairs(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<PairInfo>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let start = start_after.map(|s| Bound::exclusive(s.as_bytes().to_vec()));

    PAIRS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(PairInfo {
                name: String::from_utf8(k).unwrap(),
                pair_addr: v,
            })
        })
        .collect::<StdResult<Vec<PairInfo>>>()
}

pub const TMP_SWAP: Item<String> = Item::new("tmp_swap");
pub fn get_tmp_swap(deps: Deps) -> StdResult<String> {
    TMP_SWAP.load(deps.storage)
}

pub fn set_tmp_swap(deps: DepsMut, swap: String) -> StdResult<()> {
    TMP_SWAP.save(deps.storage, &swap)
}

pub fn remove_tmp_swap(deps: DepsMut) {
    TMP_SWAP.remove(deps.storage)
}
