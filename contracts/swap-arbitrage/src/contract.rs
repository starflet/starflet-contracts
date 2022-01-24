#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Attribute, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Reply, ReplyOn, Response, StdResult, SubMsg, WasmMsg,
};
use cw2::set_contract_version;
use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper};

use crate::{
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, Pair},
    state::{get_pair, get_tmp_swap, remove_pair, set_pair, set_tmp_swap},
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
use std::str::FromStr;
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

    set_pair(
        deps.branch(),
        NATIVESWAP.to_string(),
        Addr::unchecked(NATIVESWAP.to_string()),
    )
    .unwrap();

    for pair in msg.pairs.iter() {
        let addr = deps.api.addr_validate(&pair.pair_addr).unwrap();
        set_pair(deps.branch(), pair.name.to_string(), addr).unwrap();
    }

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
            add_pairs,
            remove_pairs,
        } => try_update_config(
            deps,
            info,
            owner,
            commission_rate,
            code_id,
            add_pairs,
            remove_pairs,
        ),
        ExecuteMsg::Bond { asset } => try_bond(deps, info, asset),
        ExecuteMsg::Swap { path } => try_swap(deps, env, info, path),
        ExecuteMsg::Claim {} => try_claim(deps, info),
    }
}

pub fn try_update_config(
    mut deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    commission_rate: Option<Decimal256>,
    code_id: Option<u64>,
    add_pairs: Option<Vec<Pair>>,
    remove_pairs: Option<Vec<String>>,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    let config: Config = get_config(deps.as_ref()).unwrap();
    let mut attrs: Vec<Attribute> = vec![];

    // permission check
    if info.sender != config.owner {
        return Err(PlanetContractError::Unauthorized {});
    }

    if let Some(add_pairs) = add_pairs {
        attrs.push(Attribute::new("action", "add_pairs"));

        for pair in add_pairs.iter() {
            let addr = deps.api.addr_validate(&pair.pair_addr.clone()).unwrap();
            set_pair(deps.branch(), pair.name.to_string(), addr).unwrap();
            attrs.push(Attribute::new("name", pair.name.to_string()));
            attrs.push(Attribute::new("pair_contract", pair.pair_addr.to_string()));
        }
    }

    if let Some(remove_pairs) = remove_pairs {
        attrs.push(Attribute::new("action", "remove_pairs"));
        for name in remove_pairs.iter() {
            remove_pair(deps.branch(), name.to_string());
            attrs.push(Attribute::new("name", name));
        }
    }

    match try_planet_update_config(deps, info, owner, commission_rate, code_id) {
        Ok(res) => Ok(res.add_attributes(attrs)),
        Err(e) => Err(e),
    }
}

const MSG_REPLY_SWAP: u64 = 11;

const NATIVESWAP: &str = "nativeswap";

pub fn try_swap(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    path: String,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    let dex = path.split("_to_").collect::<Vec<&str>>();

    let pair_contract = get_pair(deps.as_ref(), dex[0].to_string()).unwrap();

    let config: Config = get_config(deps.as_ref()).unwrap();
    let asset_info: AssetInfo = config.asset_info;

    let balance = asset_info
        .query_pool(&deps.querier, deps.api, env.contract.address)
        .unwrap();

    let mut funds: Vec<Coin> = vec![];

    let asset = Asset {
        info: asset_info.clone(),
        amount: balance,
    };

    let coin = match dex[0] {
        NATIVESWAP => Coin {
            denom: format!("{}", asset_info),
            amount: asset.amount,
        },
        _ => asset.deduct_tax(&deps.querier).unwrap(),
    };
    funds.push(coin.clone());

    set_tmp_swap(deps, dex[1].to_string()).unwrap();

    let msg = match dex[0] {
        NATIVESWAP => create_swap_msg(coin, "uluna".to_string()),
        _ => CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_contract.to_string(),
            funds,
            msg: to_binary(&PairExecuteMsg::Swap {
                offer_asset: Asset {
                    amount: coin.amount,
                    info: asset_info,
                },
                belief_price: None,
                max_spread: Some(Decimal::from_str("0.5").unwrap()),
                to: None,
            })?,
        }),
    };

    Ok(Response::new().add_submessage(SubMsg {
        id: MSG_REPLY_SWAP,
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
    let config: PlanetConfigResponse = query_planet_config(deps);

    ConfigResponse {
        owner: config.owner,
        commission_rate: config.commission_rate,
        asset_info: config.asset_info,
        token_code_id: config.token_code_id,
        token_address: config.token_address,
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
        MSG_REPLY_SWAP => {
            let name = get_tmp_swap(deps.as_ref()).unwrap();
            let amount = deps
                .querier
                .query_balance(env.contract.address, "uluna")
                .unwrap()
                .amount;

            let s_name = &*name;
            let msg = match s_name {
                NATIVESWAP => create_swap_msg(
                    Coin {
                        amount,
                        denom: "uluna".to_string(),
                    },
                    "uusd".to_string(),
                ),
                _ => {
                    let contract = get_pair(deps.as_ref(), name).unwrap();
                    CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: contract.to_string(),
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
                            max_spread: Some(Decimal::from_str("0.5").unwrap()),
                            to: None,
                        })?,
                    })
                }
            };

            Ok(Response::new().add_submessage(SubMsg::reply_on_success(
                msg,
                planet::contract::MSG_REPLY_ID_EXECUTE,
            )))
        }
        _ => planet_reply(deps, env, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, env: Env, msg: MigrateMsg) -> StdResult<Response> {
    for pair in msg.pairs.iter() {
        let addr = deps.api.addr_validate(&pair.pair_addr).unwrap();
        set_pair(deps.branch(), pair.name.to_string(), addr).unwrap();
    }

    set_pair(
        deps.branch(),
        NATIVESWAP.to_string(),
        Addr::unchecked(NATIVESWAP.to_string()),
    )
    .unwrap();

    planet_migrate(deps, env, PlanetMigrateMsg {})
}
