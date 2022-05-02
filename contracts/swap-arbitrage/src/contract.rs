#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, Attribute, BankMsg, Binary, Coin, ContractResult, CosmosMsg,
    Deps, DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError, StdResult, SubMsg,
    Uint128, WasmMsg,
};
use cw2::set_contract_version;
use terra_cosmwasm::TerraMsgWrapper;

use crate::{
    msg::{
        ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, MSG_REPLY_BOND, MSG_REPLY_CLAIM,
        MSG_REPLY_MIGRATE, MSG_REPLY_PREPARE_SWAP, MSG_REPLY_SWAP, MSG_REPLY_UNBOND,
    },
    querier::query_epoch_state,
    state::{
        get_anchor_info, get_deposit_asset_info, get_router, get_tmp_bonder, get_tmp_swap,
        remove_tmp_swap, set_deposit_asset_info, set_router, set_tmp_bonder, set_tmp_swap,
    },
};
use starflet_protocol::planet::{
    CommissionResponse, ConfigResponse as PlanetConfigResponse, Cw20HookMsg,
    InstantiateMsg as PlanetInstantiateMsg, QueryMsg, StakerInfoResponse,
};

use cosmwasm_bignumber::{Decimal256, Uint256};
use planet::{
    contract::{
        compute_share_rate, instantiate as planet_instantiate, query as planet_query,
        query_config as query_planet_config, query_stake_info as planet_query_stake_info,
        reply as planet_reply, try_bond as planet_bond,
        try_update_config as try_planet_update_config,
    },
    error::ContractError as PlanetContractError,
    state::{get_commission, get_config, set_vaults, sub_all_commission, sub_vaults, Config},
};
use terraswap::{
    asset::{Asset, AssetInfo},
    querier::{query_balance, query_token_balance},
    router::{ExecuteMsg as TerraswapRouterExecute, SwapOperation},
};

use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg};
use moneymarket::market::{
    Cw20HookMsg as MoneyMarketCw20HookMsg, ExecuteMsg as MoneyMarketExecuteMsg,
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

    set_deposit_asset_info(deps.branch(), msg.deposit_asset_info).unwrap();

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
        ExecuteMsg::Bond { asset } => try_bond(deps, env, info, asset),
        ExecuteMsg::Swap { path, amount } => try_swap(deps, env, path, amount),
        ExecuteMsg::Claim {} => try_claim(deps, info),
    }
}

pub fn receive_cw20(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    let contract_addr = info.sender;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Unbond {}) => {
            // only asset contract can execute this message
            let config: Config = get_config(deps.as_ref()).unwrap();
            if contract_addr != config.clone().token_address.unwrap() {
                return Err(PlanetContractError::Unauthorized {});
            }

            let share_rate =
                compute_share_rate(deps.as_ref(), config.token_address.unwrap()).unwrap();
            let unbond_amount = Uint256::from(cw20_msg.amount) * share_rate;

            let anchor_info = get_anchor_info(deps.as_ref()).unwrap();

            sub_vaults(deps.branch(), Decimal256::from_uint256(unbond_amount)).unwrap();

            let deposit_asset_info = get_deposit_asset_info(deps.as_ref()).unwrap();
            let balance = query_balance(
                &deps.querier,
                env.contract.address,
                deposit_asset_info.to_string(),
            )
            .unwrap();
            set_tmp_bonder(
                deps.branch(),
                Addr::unchecked(cw20_msg.sender.to_string()),
                balance,
            )
            .unwrap();

            Ok(Response::new()
                .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: contract_addr.to_string(),
                    funds: vec![],
                    msg: to_binary(&Cw20ExecuteMsg::Burn {
                        amount: cw20_msg.amount,
                    })?,
                }))
                .add_submessage(SubMsg::reply_on_success(
                    CosmosMsg::Wasm(WasmMsg::Execute {
                        contract_addr: anchor_info.aust.to_string(),
                        funds: vec![],
                        msg: to_binary(&Cw20ExecuteMsg::Send {
                            amount: unbond_amount.into(),
                            contract: anchor_info.market_money.to_string(),
                            msg: to_binary(&MoneyMarketCw20HookMsg::RedeemStable {}).unwrap(),
                        })
                        .unwrap(),
                    }),
                    MSG_REPLY_UNBOND,
                )))
        }
        _ => Err(PlanetContractError::InvalidHookMsg {}),
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

pub fn try_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    asset: Asset,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    let anchor_info = get_anchor_info(deps.as_ref()).unwrap();
    let coin = asset.deduct_tax(&deps.querier).unwrap();

    let aust_addr = Addr::unchecked(anchor_info.aust.to_string());

    let current_amount =
        query_token_balance(&deps.querier, aust_addr, env.contract.address).unwrap();

    set_tmp_bonder(deps, info.sender, current_amount).unwrap();

    Ok(Response::new().add_submessage(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: anchor_info.market_money.to_string(),
            funds: vec![coin],
            msg: to_binary(&MoneyMarketExecuteMsg::DepositStable {}).unwrap(),
        }),
        MSG_REPLY_BOND,
    )))
}

const NATIVESWAP: &str = "native_swap";
const TERRASWAP: &str = "terra_swap";
const ASTROPORT: &str = "astroport";
const LOOP: &str = "loop";

pub fn try_swap(
    mut deps: DepsMut,
    env: Env,
    path: String,
    amount: Uint128,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    set_tmp_swap(deps.branch(), path, amount).unwrap();

    let anchor_info = get_anchor_info(deps.as_ref()).unwrap();
    let epoch_state = query_epoch_state(
        deps.as_ref(),
        anchor_info.market_money.clone(),
        env.block.height,
        None,
    )
    .unwrap();

    let aust = Uint256::from(amount) / epoch_state.exchange_rate;

    Ok(Response::new().add_submessage(SubMsg {
        msg: CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: anchor_info.aust.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                amount: aust.into(),
                contract: anchor_info.market_money.to_string(),
                msg: to_binary(&MoneyMarketCw20HookMsg::RedeemStable {}).unwrap(),
            })
            .unwrap(),
        }),
        id: MSG_REPLY_PREPARE_SWAP,
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

    let asset_info: AssetInfo = get_deposit_asset_info(deps.as_ref()).unwrap();

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

pub fn try_claim(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    let config = get_config(deps.as_ref()).unwrap();
    if config.owner != info.sender {
        return Err(PlanetContractError::Unauthorized {});
    }

    let anchor_info = get_anchor_info(deps.as_ref()).unwrap();

    let dec_amount = sub_all_commission(deps).unwrap();
    let amount = Uint256::one() * dec_amount;

    Ok(Response::new().add_submessage(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: anchor_info.aust.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Send {
                amount: amount.into(),
                contract: anchor_info.market_money.to_string(),
                msg: to_binary(&MoneyMarketCw20HookMsg::RedeemStable {}).unwrap(),
            })
            .unwrap(),
        }),
        MSG_REPLY_CLAIM,
    )))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)),
        QueryMsg::StakerInfo { staker_addr } => {
            to_binary(&query_stake_info(deps, env, staker_addr))
        }
        QueryMsg::Commission {} => to_binary(&query_commission(deps, env)),
        _ => planet_query(deps, env, msg),
    }
}

pub fn query_config(deps: Deps) -> ConfigResponse {
    let config: PlanetConfigResponse = query_planet_config(deps);

    let router = get_router(deps).unwrap();
    let anchor_info = get_anchor_info(deps).unwrap();
    let deposit_asset_info = get_deposit_asset_info(deps).unwrap();

    ConfigResponse {
        owner: config.owner,
        commission_rate: config.commission_rate,
        asset_info: config.asset_info,
        token_code_id: config.token_code_id,
        token_address: config.token_address,
        router_addr: router.to_string(),
        deposit_asset_info,
        money_market_addr: anchor_info.market_money.to_string(),
    }
}

pub fn query_stake_info(deps: Deps, env: Env, staker_addr: String) -> StakerInfoResponse {
    let res = planet_query_stake_info(deps, staker_addr);
    let deposit_asset_info = get_deposit_asset_info(deps).unwrap();

    let anchor_info = get_anchor_info(deps).unwrap();

    let epoch_state =
        query_epoch_state(deps, anchor_info.market_money, env.block.height, None).unwrap();

    let balance = Uint256::from(res.asset.amount) * epoch_state.exchange_rate;

    StakerInfoResponse {
        asset: Asset {
            info: deposit_asset_info,
            amount: balance.into(),
        },
    }
}

fn query_commission(deps: Deps, env: Env) -> CommissionResponse {
    let deposit_asset_info = get_deposit_asset_info(deps).unwrap();

    let anchor_info = get_anchor_info(deps).unwrap();

    let epoch_state =
        query_epoch_state(deps, anchor_info.market_money, env.block.height, None).unwrap();
    let commission = get_commission(deps).unwrap();

    let balance = commission * epoch_state.exchange_rate;

    CommissionResponse {
        asset: Asset {
            info: deposit_asset_info,
            amount: (Uint256::one() * balance).into(),
        },
    }
}

const LIMIT_MINIMUM: Uint128 = Uint128::new(10_000_000_000u128);
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    mut deps: DepsMut,
    env: Env,
    reply: Reply,
) -> Result<Response<TerraMsgWrapper>, PlanetContractError> {
    match reply.id {
        MSG_REPLY_PREPARE_SWAP => {
            let tmp_swap = get_tmp_swap(deps.as_ref()).unwrap();

            let deposit_asset_info = get_deposit_asset_info(deps.as_ref()).unwrap();
            let amount = deposit_asset_info
                .query_pool(&deps.querier, deps.api, env.contract.address)
                .unwrap();

            // discarding
            if amount + Uint128::from(10u64) < tmp_swap.minimum_receive {
                return Err(PlanetContractError::Std(StdError::generic_err(format!(
                    "swap amount is diffrent {} / {}",
                    amount, tmp_swap.minimum_receive
                ))));
            }

            set_tmp_swap(deps.branch(), tmp_swap.route_path.clone(), amount).unwrap();

            let msg = generate_route_msg(deps, tmp_swap.route_path, amount);

            Ok(Response::new().add_submessage(SubMsg {
                msg,
                id: MSG_REPLY_SWAP,
                gas_limit: None,
                reply_on: ReplyOn::Always,
            }))
        }
        MSG_REPLY_SWAP => {
            let tmp_swap = get_tmp_swap(deps.as_ref()).unwrap();

            match reply.result {
                ContractResult::Ok(_) => {
                    remove_tmp_swap(deps.branch());

                    let deposit_asset_info = get_deposit_asset_info(deps.as_ref()).unwrap();
                    let coin = deps
                        .querier
                        .query_balance(env.contract.address, deposit_asset_info.to_string())
                        .unwrap();

                    let asset = Asset {
                        info: deposit_asset_info,
                        amount: coin.amount,
                    };
                    let coin = asset.deduct_tax(&deps.querier).unwrap();

                    let anchor_info = get_anchor_info(deps.as_ref()).unwrap();

                    Ok(Response::new().add_submessage(SubMsg::reply_on_success(
                        CosmosMsg::Wasm(WasmMsg::Execute {
                            contract_addr: anchor_info.market_money.to_string(),
                            funds: vec![coin],
                            msg: to_binary(&MoneyMarketExecuteMsg::DepositStable {}).unwrap(),
                        }),
                        planet::contract::MSG_REPLY_ID_EXECUTE,
                    )))
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
                    set_tmp_swap(deps.branch(), tmp_swap.route_path.clone(), minimum_receive)
                        .unwrap();
                    let msg = generate_route_msg(deps, tmp_swap.route_path, minimum_receive);
                    Ok(Response::new().add_submessage(SubMsg {
                        msg,
                        id: MSG_REPLY_SWAP,
                        gas_limit: None,
                        reply_on: ReplyOn::Always,
                    }))
                }
            }
        }
        MSG_REPLY_BOND => {
            let tmp_bonder = get_tmp_bonder(deps.as_ref()).unwrap();
            let anchor_info = get_anchor_info(deps.as_ref()).unwrap();
            let config = get_config(deps.as_ref()).unwrap();

            let current_balance = config
                .asset_info
                .query_pool(&deps.querier, deps.api, env.contract.address)
                .unwrap();

            planet_bond(
                deps,
                tmp_bonder.bonder,
                Asset {
                    info: anchor_info.aust,
                    amount: current_balance - tmp_bonder.prev_amount,
                },
            )
        }
        MSG_REPLY_MIGRATE => {
            let anchor_info = get_anchor_info(deps.as_ref()).unwrap();

            let aust_addr = Addr::unchecked(anchor_info.aust.to_string());
            let balance =
                query_token_balance(&deps.querier, aust_addr, env.contract.address).unwrap();
            set_vaults(
                deps.branch(),
                Decimal256::from_uint256(Uint256::from(balance)),
            )
            .unwrap();
            Ok(Response::new().add_attribute("migrate", "success"))
        }
        MSG_REPLY_UNBOND => {
            let tmp_bonder = get_tmp_bonder(deps.as_ref()).unwrap();

            let deposit_asset_info = get_deposit_asset_info(deps.as_ref()).unwrap();
            let mut balance = query_balance(
                &deps.querier,
                env.contract.address,
                deposit_asset_info.to_string(),
            )
            .unwrap();

            balance = balance.checked_sub(tmp_bonder.prev_amount).unwrap();
            let asset = Asset {
                amount: balance,
                info: deposit_asset_info,
            };

            Ok(Response::new().add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: tmp_bonder.bonder.to_string(),
                amount: vec![asset.deduct_tax(&deps.querier)?],
            })))
        }
        MSG_REPLY_CLAIM => {
            let config = get_config(deps.as_ref()).unwrap();

            let deposit_asset_info = get_deposit_asset_info(deps.as_ref()).unwrap();
            let balance = query_balance(
                &deps.querier,
                env.contract.address,
                deposit_asset_info.to_string(),
            )
            .unwrap();

            let asset = Asset {
                info: deposit_asset_info,
                amount: balance,
            };

            Ok(Response::new().add_message(CosmosMsg::Bank(BankMsg::Send {
                to_address: config.owner.to_string(),
                amount: vec![asset.deduct_tax(&deps.querier)?],
            })))
        }
        _ => planet_reply(deps, env, reply),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let config = get_config(deps.as_ref()).unwrap();
    let vaults_address = config.token_address.unwrap();
    let amount =
        query_token_balance(&deps.querier, vaults_address.clone(), env.contract.address).unwrap();

    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: vaults_address.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
        })),
    )
}
