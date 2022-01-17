#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Attribute, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn,
    Response, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper};

use crate::{
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, Path},
    state::{get_contracts, set_contracts, Contracts},
};
use starflet_protocol::planet::{
    ConfigResponse as PlanetConfigResponse, InstantiateMsg as PlanetInstantiateMsg,
    MigrateMsg as PlanetMigrateMsg, QueryMsg,
};

use cosmwasm_bignumber::Decimal256;
use planet::{
    contract::{
        instantiate as planet_instantiate, migrate as planet_migrate, query as planet_query,
        query_config as query_planet_config, receive_cw20, reply as planet_reply, try_bond,
        try_claim, try_update_config as try_planet_update_config,
    },
    error::ContractError as PlanetContractError,
    state::{get_config, Config},
};
use terraswap::{
    asset::{Asset, AssetInfo},
    pair::ExecuteMsg as PairExecuteMsg,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:swap-arbitrage";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, PlanetContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let contracts = Contracts {
        pair: deps.api.addr_validate(&msg.pair_contract).unwrap(),
    };

    set_contracts(deps.branch(), contracts).unwrap();

    let planet_msg = PlanetInstantiateMsg {
        commission_rate: msg.commission_rate,
        asset_info: msg.asset_info,
        symbol: msg.symbol,
        token_code_id: msg.token_code_id,
    };

    planet_instantiate(deps, env, info, planet_msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateConfig {
            owner,
            commission_rate,
            code_id,
            pair_contract,
        } => try_update_config(deps, info, owner, commission_rate, code_id, pair_contract),
        ExecuteMsg::Bond { asset } => try_bond(deps, info, asset),
        ExecuteMsg::Swap { path } => try_swap(deps.as_ref(), env, info, path),
        ExecuteMsg::Claim {} => try_claim(deps, info),
    }
}

pub fn try_update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    commission_rate: Option<Decimal256>,
    code_id: Option<u64>,
    pair_contract: Option<String>,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    let config: Config = get_config(deps.as_ref()).unwrap();
    let mut attrs: Vec<Attribute> = vec![];

    // permission check
    if info.sender != config.owner {
        return Err(PlanetContractError::Unauthorized {});
    }

    let mut contracts = get_contracts(deps.as_ref()).unwrap();

    if let Some(pair_contract) = pair_contract {
        let pair = deps.api.addr_validate(&pair_contract).unwrap();
        contracts.pair = pair.clone();
        attrs.push(Attribute::new("pair_contract", pair.to_string()));
    }

    match try_planet_update_config(deps, info, owner, commission_rate, code_id) {
        Ok(res) => Ok(res.add_attributes(attrs)),
        Err(e) => Err(e),
    }
}

const MSG_REPLY_NATIVE_SWAP_TO_TERRA_SWAP: u64 = 11;
const MSG_REPLY_TERRA_SWAP_TO_NATIVE_SWAP: u64 = 12;

pub fn try_swap(
    deps: Deps,
    env: Env,
    _info: MessageInfo,
    path: Path,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    let contracts = get_contracts(deps).unwrap();

    let config: Config = get_config(deps).unwrap();
    let asset_info: AssetInfo = config.asset_info;

    let balance = asset_info
        .query_pool(&deps.querier, deps.api, env.contract.address)
        .unwrap();

    let mut funds: Vec<Coin> = vec![];

    let asset = Asset {
        info: asset_info.clone(),
        amount: balance,
    };

    let coin = match path {
        Path::TerraSwapToNativeSwap => asset.deduct_tax(&deps.querier).unwrap(),
        Path::NativeSwapToTerraSwap => Coin {
            denom: format!("{}", asset_info),
            amount: asset.amount,
        },
    };
    funds.push(coin.clone());

    let (msg, sub_msg_id) = match path {
        Path::NativeSwapToTerraSwap => (
            create_swap_msg(coin, "uluna".to_string()),
            MSG_REPLY_NATIVE_SWAP_TO_TERRA_SWAP,
        ),
        Path::TerraSwapToNativeSwap => (
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contracts.pair.to_string(),
                funds,
                msg: to_binary(&PairExecuteMsg::Swap {
                    offer_asset: Asset {
                        amount: coin.amount,
                        info: asset_info,
                    },
                    belief_price: None,
                    max_spread: None,
                    to: None,
                })?,
            }),
            MSG_REPLY_TERRA_SWAP_TO_NATIVE_SWAP,
        ),
    };

    Ok(Response::new().add_submessage(SubMsg {
        id: sub_msg_id,
        gas_limit: None,
        msg,
        reply_on: ReplyOn::Success,
    }))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)),
        _ => planet_query(deps, env, msg),
    }
}

pub fn query_config(deps: Deps) -> ConfigResponse {
    let contracts = get_contracts(deps).unwrap();

    let config: PlanetConfigResponse = query_planet_config(deps);

    ConfigResponse {
        owner: config.owner,
        commission_rate: config.commission_rate,
        asset_info: config.asset_info,
        token_code_id: config.token_code_id,
        token_address: config.token_address,
        pair_contract: contracts.pair.to_string(),
    }
}

/// This just stores the result for future query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut,
    env: Env,
    msg: Reply,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    match msg.id {
        MSG_REPLY_NATIVE_SWAP_TO_TERRA_SWAP => {
            let contracts = get_contracts(deps.as_ref()).unwrap();
            let amount = deps
                .querier
                .query_balance(env.contract.address, "uluna")
                .unwrap()
                .amount;

            Ok(Response::new().add_submessage(SubMsg {
                id: planet::contract::MSG_REPLY_ID_EXECUTE,
                gas_limit: None,
                msg: CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contracts.pair.to_string(),
                    funds: vec![Coin {
                        amount,
                        denom: "uluna".to_string(),
                    }],
                    msg: to_binary(&PairExecuteMsg::Swap {
                        offer_asset: Asset {
                            amount,
                            info: AssetInfo::NativeToken {
                                denom: "uluna".to_string(),
                            },
                        },
                        belief_price: None,
                        max_spread: None,
                        to: None,
                    })?,
                }),
                reply_on: ReplyOn::Success,
            }))
        }
        MSG_REPLY_TERRA_SWAP_TO_NATIVE_SWAP => {
            let amount = deps
                .querier
                .query_balance(env.contract.address, "uluna")
                .unwrap()
                .amount;

            Ok(Response::new().add_submessage(SubMsg {
                id: planet::contract::MSG_REPLY_ID_EXECUTE,
                gas_limit: None,
                msg: create_swap_msg(
                    Coin {
                        amount,
                        denom: "uluna".to_string(),
                    },
                    "uusd".to_string(),
                ),
                reply_on: ReplyOn::Success,
            }))
        }
        _ => planet_reply(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, env: Env, msg: MigrateMsg) -> StdResult<Response> {
    let contracts = Contracts {
        pair: deps.api.addr_validate(&msg.pair_contract).unwrap(),
    };
    set_contracts(deps.branch(), contracts).unwrap();
    planet_migrate(deps, env, PlanetMigrateMsg {})
}
