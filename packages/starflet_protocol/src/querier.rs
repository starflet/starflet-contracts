use cosmwasm_std::{to_binary, Addr, QuerierWrapper, QueryRequest, StdResult, WasmQuery};

use crate::planet::{ConfigResponse, QueryMsg};
use cw20::{Cw20QueryMsg, TokenInfoResponse};

pub fn query_planet_config(
    querier: &QuerierWrapper,
    planet_address: Addr,
) -> StdResult<ConfigResponse> {
    let res: ConfigResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: planet_address.to_string(),
        msg: to_binary(&QueryMsg::Config {})?,
    }))?;

    Ok(res)
}

pub fn query_vaults_info(
    querier: &QuerierWrapper,
    vaults_address: Addr,
) -> StdResult<TokenInfoResponse> {
    let res: TokenInfoResponse = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: vaults_address.to_string(),
        msg: to_binary(&Cw20QueryMsg::TokenInfo {})?,
    }))?;

    Ok(res)
}
