#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::Reply;
use cosmwasm_std::{
    to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;

use crate::{
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, RoutePath},
    state::{get_router, set_router},
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
        try_claim, try_execute, try_update_config as try_planet_update_config,
    },
    error::ContractError as PlanetContractError,
    state::{get_config, Config},
};
use terraswap::{
    asset::{Asset, AssetInfo},
    router::{ExecuteMsg as TerraswapRouterExecute, SwapOperation},
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

    let router_contract = deps.api.addr_validate(&msg.terraswap_router).unwrap();
    set_router(deps.branch(), router_contract).unwrap();

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
) -> Result<Response, PlanetContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateConfig {
            owner,
            commission_rate,
            code_id,
            terraswap_router,
        } => try_update_config(
            deps,
            info,
            owner,
            commission_rate,
            code_id,
            terraswap_router,
        ),
        ExecuteMsg::Bond { asset } => try_bond(deps, info, asset),
        ExecuteMsg::Swap { route_path } => try_swap(deps.as_ref(), env, info, route_path),
        ExecuteMsg::Claim {} => try_claim(deps, info),
    }
}

pub fn try_update_config(
    mut deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    commission_rate: Option<Decimal256>,
    code_id: Option<u64>,
    terraswap_router: Option<String>,
) -> Result<Response, PlanetContractError> {
    let config: Config = get_config(deps.as_ref()).unwrap();

    // permission check
    if info.sender != config.owner {
        return Err(PlanetContractError::Unauthorized {});
    }

    if let Some(terraswap_router) = terraswap_router {
        let router_contract = deps.api.addr_validate(&terraswap_router).unwrap();
        set_router(deps.branch(), router_contract).unwrap();
    }

    try_planet_update_config(deps, info, owner, commission_rate, code_id)
}

pub fn try_swap(
    deps: Deps,
    env: Env,
    info: MessageInfo,
    route_path: RoutePath,
) -> Result<Response, PlanetContractError> {
    let router_contract = get_router(deps).unwrap();

    let config: Config = get_config(deps).unwrap();
    let asset_info: AssetInfo = config.asset_info;

    let balance = asset_info
        .query_pool(&deps.querier, deps.api, env.contract.address)
        .unwrap();

    let mut funds: Vec<Coin> = vec![];

    if asset_info.is_native_token() {
        let asset = Asset {
            info: asset_info.clone(),
            amount: balance,
        };

        let coin = asset.deduct_tax(&deps.querier);

        if let Ok(coin) = coin {
            funds.push(coin)
        }
    }

    let execute_msg = generate_route_msg(route_path, asset_info, balance);

    let msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: router_contract.to_string(),
        funds,
        msg: to_binary(&execute_msg).unwrap(),
    });

    try_execute(deps, info, msg, true)
}

fn generate_route_msg(
    route_path: RoutePath,
    asset_info: AssetInfo,
    minimum_receive: Uint128,
) -> TerraswapRouterExecute {
    let operations: Vec<SwapOperation> = match route_path {
        RoutePath::NativeSwapToTerraSwap => vec![
            SwapOperation::NativeSwap {
                offer_denom: asset_info.to_string(),
                ask_denom: "uluna".to_string(),
            },
            SwapOperation::TerraSwap {
                offer_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                ask_asset_info: asset_info,
            },
        ],
        RoutePath::TerraSwapToNativeSwap => vec![
            SwapOperation::TerraSwap {
                offer_asset_info: asset_info.clone(),
                ask_asset_info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
            },
            SwapOperation::NativeSwap {
                offer_denom: "uluna".to_string(),
                ask_denom: asset_info.to_string(),
            },
        ],
    };

    TerraswapRouterExecute::ExecuteSwapOperations {
        operations,
        minimum_receive: Some(minimum_receive),
        to: None,
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)),
        _ => planet_query(deps, env, msg),
    }
}

pub fn query_config(deps: Deps) -> ConfigResponse {
    let terraswap_router = get_router(deps).unwrap();

    let config: PlanetConfigResponse = query_planet_config(deps);

    ConfigResponse {
        owner: config.owner,
        commission_rate: config.commission_rate,
        asset_info: config.asset_info,
        token_code_id: config.token_code_id,
        token_address: config.token_address,
        terraswap_router: terraswap_router.to_string(),
    }
}

/// This just stores the result for future query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, PlanetContractError> {
    planet_reply(deps, env, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: PlanetMigrateMsg) -> StdResult<Response> {
    planet_migrate(deps, env, msg)
}
