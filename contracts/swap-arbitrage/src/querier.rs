use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{to_binary, Addr, Deps, QueryRequest, StdResult, WasmQuery};
use moneymarket::market::{EpochStateResponse, QueryMsg};

pub fn query_epoch_state(
    deps: Deps,
    market_addr: Addr,
    block_height: u64,
    distributed_interest: Option<Uint256>,
) -> StdResult<EpochStateResponse> {
    let epoch_state: EpochStateResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: market_addr.to_string(),
            msg: to_binary(&QueryMsg::EpochState {
                block_height: Some(block_height),
                distributed_interest,
            })?,
        }))?;

    Ok(epoch_state)
}
