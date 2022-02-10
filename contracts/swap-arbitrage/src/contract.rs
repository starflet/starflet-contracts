#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Attribute, Binary, Coin, ContractResult, CosmosMsg, Deps, DepsMut, Env, MessageInfo,
    Reply, ReplyOn, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use terra_cosmwasm::TerraMsgWrapper;

use crate::{
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg},
    state::{get_router, get_tmp_swap, remove_tmp_swap, set_router, set_tmp_swap},
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

    let router_addr = deps.api.addr_validate(&msg.router_addr).unwrap();
    set_router(deps.branch(), router_addr).unwrap();

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
            router_addr,
        } => try_update_config(deps, info, owner, commission_rate, code_id, router_addr),
        ExecuteMsg::Bond { asset } => try_bond(deps, info, asset),
        ExecuteMsg::Swap { path, amount } => try_swap(deps, env, info, path, amount),
        ExecuteMsg::Claim {} => try_claim(deps, info),
    }
}

pub fn try_update_config(
    mut deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    commission_rate: Option<Decimal256>,
    code_id: Option<u64>,
    router_addr: Option<String>,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    let config: Config = get_config(deps.as_ref()).unwrap();
    let mut attrs: Vec<Attribute> = vec![];

    // permission check
    if info.sender != config.owner {
        return Err(PlanetContractError::Unauthorized {});
    }

    if let Some(router_addr) = router_addr {
        attrs.push(Attribute::new("action", "router_addr"));

        let router_addr = deps.api.addr_validate(&router_addr).unwrap();
        set_router(deps.branch(), router_addr.clone()).unwrap();
        attrs.push(Attribute::new("router_addr", router_addr));
    }

    match try_planet_update_config(deps, info, owner, commission_rate, code_id) {
        Ok(res) => Ok(res.add_attributes(attrs)),
        Err(e) => Err(e),
    }
}

const MSG_REPLY_SWAP: u64 = 11;

const NATIVESWAP: &str = "native_swap";
const TERRASWAP: &str = "terra_swap";
const ASTROPORT: &str = "astroport";
const LOOP: &str = "loop";

pub fn try_swap(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    path: String,
    amount: Uint128,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    set_tmp_swap(deps.branch(), path.clone(), amount).unwrap();

    let msg = generate_route_msg(deps, path, amount);

    Ok(Response::new().add_submessage(SubMsg {
        msg,
        id: MSG_REPLY_SWAP,
        gas_limit: None,
        reply_on: ReplyOn::Always,
    }))
}

fn generate_route_msg(
    deps: DepsMut,
    route_path: String,
    amount: Uint128,
) -> CosmosMsg<TerraMsgWrapper> {
    let dex = route_path.split("_to_").collect::<Vec<&str>>();

    let router_contract = get_router(deps.as_ref()).unwrap();

    let config: Config = get_config(deps.as_ref()).unwrap();
    let asset_info: AssetInfo = config.asset_info;

    let mut funds: Vec<Coin> = vec![];

    let asset = Asset {
        info: asset_info.clone(),
        amount,
    };

    let coin = match dex[0] {
        NATIVESWAP => Coin {
            denom: format!("{}", asset_info),
            amount: asset.amount,
        },
        _ => asset.deduct_tax(&deps.querier).unwrap(),
    };
    funds.push(coin);

    let operations: Vec<SwapOperation> = vec![
        get_swap_oprations_path(
            dex[0],
            asset_info.clone(),
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
        )
        .unwrap(),
        get_swap_oprations_path(
            dex[1],
            AssetInfo::NativeToken {
                denom: "uluna".to_string(),
            },
            asset_info,
        )
        .unwrap(),
    ];

    let execute_msg = TerraswapRouterExecute::ExecuteSwapOperations {
        operations,
        minimum_receive: Some(amount),
        to: None,
    };

    CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: router_contract.to_string(),
        funds,
        msg: to_binary(&execute_msg).unwrap(),
    })
}

fn get_swap_oprations_path(
    route_path: &str,
    offer_asset_info: AssetInfo,
    ask_asset_info: AssetInfo,
) -> Result<SwapOperation, PlanetContractError> {
    let operation: SwapOperation = match route_path {
        NATIVESWAP => SwapOperation::NativeSwap {
            offer_denom: offer_asset_info.to_string(),
            ask_denom: ask_asset_info.to_string(),
        },
        TERRASWAP => SwapOperation::TerraSwap {
            offer_asset_info,
            ask_asset_info,
        },
        ASTROPORT => SwapOperation::Astroport {
            offer_asset_info,
            ask_asset_info,
        },
        LOOP => SwapOperation::Loop {
            offer_asset_info,
            ask_asset_info,
        },
        _ => return Err(PlanetContractError::FailedToParse(route_path.to_string())),
    };

    Ok(operation)
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

const LIMIT_MINIMUM: Uint128 = Uint128::new(10_000_000_000u128);
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    mut deps: DepsMut,
    env: Env,
    mut reply: Reply,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    if reply.id == MSG_REPLY_SWAP {
        let tmp_swap = get_tmp_swap(deps.as_ref()).unwrap();

        match reply.result {
            ContractResult::Ok(_) => {
                remove_tmp_swap(deps.branch());
                reply.id = planet::contract::MSG_REPLY_ID_EXECUTE;
            }
            ContractResult::Err(_err) => {
                let minimum_receive = tmp_swap
                    .minimum_receive
                    .checked_div(Uint128::from(2u128))
                    .unwrap();
                if tmp_swap.minimum_receive < LIMIT_MINIMUM {
                    remove_tmp_swap(deps.branch());

                    return Err(PlanetContractError::Std(StdError::generic_err(
                        "limit minimum received",
                    )));
                }
                set_tmp_swap(deps.branch(), tmp_swap.route_path.clone(), minimum_receive).unwrap();
                let msg = generate_route_msg(deps, tmp_swap.route_path, minimum_receive);
                return Ok(Response::new().add_submessage(SubMsg {
                    msg,
                    id: MSG_REPLY_SWAP,
                    gas_limit: None,
                    reply_on: ReplyOn::Always,
                }));
            }
        };
    }

    planet_reply(deps, env, reply)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(mut deps: DepsMut, env: Env, msg: MigrateMsg) -> StdResult<Response> {
    let router_addr = deps.api.addr_validate(&msg.router_addr).unwrap();
    set_router(deps.branch(), router_addr).unwrap();

    planet_migrate(deps, env, PlanetMigrateMsg {})
}
