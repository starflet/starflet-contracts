#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, Attribute, BankMsg, Binary, CanonicalAddr, CosmosMsg, Deps,
    DepsMut, Env, MessageInfo, Reply, ReplyOn, Response, StdError, StdResult, SubMsg, Uint128,
    WasmMsg,
};
use moneymarket::querier::query_supply;
use starflet_protocol::planet::{
    Action, CommissionResponse, ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg,
    MigrateMsg, QueryMsg, RateResponse, StakerInfoResponse,
};
use std::ops::{Div, Mul, Sub};
use terra_cosmwasm::TerraMsgWrapper;
use terraswap::asset::{Asset, AssetInfo};
use terraswap::querier::query_token_balance;

use crate::error::ContractError;
use crate::state::{
    add_commission, add_vaults, get_commission, get_config, get_vaults, init, set_config,
    set_vaults, sub_all_commission, sub_commission, sub_vaults, Config,
};

use crate::response::MsgInstantiateContractResponse;
use cosmwasm_bignumber::{Decimal256, Uint256};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};
use protobuf::Message;
use terraswap::token::InstantiateMsg as TokenInstantiateMsg;

use std::convert::From;

pub const MSG_REPLY_ID_TOKEN_INSTANT: u64 = 1;
pub const MSG_REPLY_ID_EXECUTE: u64 = 2;
pub const MSG_REPLY_ID_EXECUTE_SKIP: u64 = 3;
pub const MSG_REPLY_ID_MUST_EXECUTE: u64 = 4;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let state = Config {
        owner: info.sender.clone(),
        commission_rate: msg.commission_rate,
        asset_info: msg.asset_info,
        token_code_id: msg.token_code_id,
        token_address: None,
    };

    set_config(deps.branch(), state).unwrap();

    init(deps).unwrap();

    Ok(Response::new()
        .add_attribute("action", Action::Instantiate.to_string())
        .add_attribute("owner", info.sender)
        .add_attribute("commission_rate", msg.commission_rate.to_string())
        .add_submessage(SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some(env.contract.address.to_string()),
                code_id: msg.token_code_id,
                funds: vec![],
                label: "".to_string(),
                msg: to_binary(&TokenInstantiateMsg {
                    name: format!("{} vaults", msg.symbol),
                    symbol: format!("v{}", msg.symbol),
                    decimals: 6u8,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: env.contract.address.to_string(),
                        cap: None,
                    }),
                })?,
            }),
            MSG_REPLY_ID_TOKEN_INSTANT,
        )))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => receive_cw20(deps, env, info, msg),
        ExecuteMsg::UpdateConfig {
            owner,
            commission_rate,
            code_id,
        } => try_update_config(deps, info, owner, commission_rate, code_id),
        ExecuteMsg::Bond { asset } => try_bond(deps, info, asset),
        ExecuteMsg::Execute { msg, is_distribute } => {
            try_execute(deps.as_ref(), info, msg, is_distribute)
        }
        ExecuteMsg::Claim {} => try_claim(deps, info),
    }
}

pub fn try_update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    commission_rate: Option<Decimal256>,
    code_id: Option<u64>,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    let mut config: Config = get_config(deps.as_ref()).unwrap();
    let mut res: Vec<Attribute> = vec![Attribute::new("action", Action::UpdateConfig.to_string())];

    // permission check
    if info.sender != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        let _ = deps.api.addr_validate(&owner)?;

        let canonical_owner: CanonicalAddr = deps.api.addr_canonicalize(&owner)?;
        config.owner = deps.api.addr_humanize(&canonical_owner)?;
        res.push(Attribute::new("owner", owner));
    }

    if let Some(commission_rate) = commission_rate {
        config.commission_rate = commission_rate;
        res.push(Attribute::new(
            "commission_rate",
            commission_rate.to_string(),
        ));
    }

    if let Some(code_id) = code_id {
        config.token_code_id = code_id;
        res.push(Attribute::new("code_id", code_id.to_string()));
    }

    set_config(deps, config).unwrap();

    Ok(Response::new().add_attributes(res))
}

pub fn compute_share_rate(deps: Deps, vaults_contract: Addr) -> StdResult<Decimal256> {
    let vaults_total_supply = query_supply(deps, vaults_contract).unwrap();
    if vaults_total_supply.is_zero() {
        return Ok(Decimal256::one());
    }

    let balance = get_vaults(deps).unwrap();

    let dec_commission = get_commission(deps).unwrap();

    let revenue = balance.sub(dec_commission);

    Ok(revenue.div(Decimal256::from_uint256(vaults_total_supply)))
}

pub fn try_bond(
    deps: DepsMut,
    info: MessageInfo,
    asset: Asset,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    asset.assert_sent_native_token_balance(&info).unwrap();

    let config = get_config(deps.as_ref()).unwrap();

    let token_contract = config.token_address.unwrap();
    let share_rate = compute_share_rate(deps.as_ref(), token_contract.clone()).unwrap();
    let mint_amount = Uint256::from(asset.amount) / share_rate;

    add_vaults(deps, Decimal256::from_uint256(Uint256::from(asset.amount))).unwrap();

    Ok(Response::new()
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: token_contract.to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: info.sender.to_string(),
                amount: mint_amount.into(),
            })?,
        }))
        .add_attribute("action", Action::Bond.to_string())
        .add_attribute("bonder", info.sender)
        .add_attribute("asset", asset.to_string())
        .add_attribute("mint_amount", mint_amount))
}

pub fn try_unbond(
    mut deps: DepsMut,
    vaults_contract: Addr,
    asset_info: AssetInfo,
    sender: Addr,
    amount: Uint128,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    let share_rate = compute_share_rate(deps.as_ref(), vaults_contract.clone()).unwrap();
    let unbond_amount = Uint256::from(amount) * share_rate;

    let unbond_asset = Asset {
        amount: unbond_amount.into(),
        info: asset_info,
    };

    sub_vaults(deps.branch(), Decimal256::from_uint256(unbond_amount)).unwrap();

    Ok(Response::new()
        .add_messages(vec![
            CosmosMsg::Bank(BankMsg::Send {
                to_address: sender.to_string(),
                amount: vec![unbond_asset.deduct_tax(&deps.querier)?],
            }),
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: vaults_contract.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Burn { amount })?,
            }),
        ])
        .add_attribute("action", Action::Unbond.to_string())
        .add_attribute("unbonder", sender)
        .add_attribute("asset", unbond_asset.to_string())
        .add_attribute("burn_amount", amount))
}

pub fn try_execute(
    deps: Deps,
    info: MessageInfo,
    msg: Binary,
    is_distribute: bool,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    let config = get_config(deps).unwrap();
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::new().add_submessage(SubMsg {
        id: if is_distribute {
            MSG_REPLY_ID_EXECUTE
        } else {
            MSG_REPLY_ID_EXECUTE_SKIP
        },
        gas_limit: None,
        msg: from_binary(&msg).unwrap(),
        reply_on: ReplyOn::Success,
    }))
}

pub fn try_claim(
    mut deps: DepsMut,
    info: MessageInfo,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    let config = get_config(deps.as_ref()).unwrap();
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let dec_amount = get_commission(deps.as_ref()).unwrap();
    let amount = Uint256::one() * dec_amount;

    let asset = Asset {
        amount: amount.into(),
        info: config.asset_info,
    };

    sub_all_commission(deps.branch()).unwrap();

    Ok(Response::new()
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![asset.deduct_tax(&deps.querier)?],
        }))
        .add_attribute("action", Action::Claim.to_string())
        .add_attribute("claimer", info.sender)
        .add_attribute("asset", asset.to_string()))
}

/// This just stores the result for future query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    mut deps: DepsMut,
    env: Env,
    msg: Reply,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    match msg.id {
        MSG_REPLY_ID_TOKEN_INSTANT => {
            // get new token's contract address
            let res: MsgInstantiateContractResponse = Message::parse_from_bytes(
                msg.result.unwrap().data.unwrap().as_slice(),
            )
            .map_err(|_| {
                ContractError::Std(StdError::parse_err(
                    "MsgInstantiateContractResponse",
                    "failed to parse data",
                ))
            })?;

            let mut config = get_config(deps.as_ref()).unwrap();
            config.token_address = Some(Addr::unchecked(res.get_contract_address()));
            set_config(deps, config).unwrap();

            Ok(Response::new()
                .add_attribute("reply", "token_instant")
                .add_attribute("token_address", res.get_contract_address()))
        }
        MSG_REPLY_ID_EXECUTE | MSG_REPLY_ID_MUST_EXECUTE => {
            let mut attrs: Vec<Attribute> = vec![attr("reply", "execute")];

            let post_vaults = get_vaults(deps.as_ref()).unwrap();

            let config = get_config(deps.as_ref()).unwrap();

            let ubalance = match config.asset_info {
                AssetInfo::NativeToken { denom } => {
                    deps.querier
                        .query_balance(env.contract.address, denom)
                        .unwrap()
                        .amount
                }
                AssetInfo::Token { contract_addr } => query_token_balance(
                    &deps.querier,
                    env.contract.address,
                    deps.api.addr_validate(contract_addr.as_str())?,
                )
                .unwrap(),
            };

            let balance = Decimal256::from_uint256(ubalance);

            if balance > post_vaults {
                let revenue = balance - post_vaults;
                let commission = revenue * config.commission_rate;
                add_commission(deps.branch(), commission).unwrap();

                attrs.push(Attribute::new("result", "success"));
                attrs.push(Attribute::new("revenue", revenue.to_string()));
                attrs.push(Attribute::new("add_commission", commission.to_string()));
            } else if msg.id == MSG_REPLY_ID_MUST_EXECUTE {
                let loss = post_vaults - balance;
                sub_commission(deps.branch(), loss).unwrap();

                attrs.push(Attribute::new("result", "fail"));
                attrs.push(Attribute::new("loss", loss.to_string()));
            } else {
                return Err(ContractError::FailedExecute(
                    post_vaults.to_string(),
                    balance.to_string(),
                ));
            }

            set_vaults(deps.branch(), balance).unwrap();

            Ok(Response::new().add_attributes(attrs))
        }
        MSG_REPLY_ID_EXECUTE_SKIP => Ok(Response::new().add_attribute("reply", "execute_skip")),
        _ => Err(ContractError::InvalidReplyId {}),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response<TerraMsgWrapper>, ContractError> {
    let contract_addr = info.sender;
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Unbond {}) => {
            // only asset contract can execute this message
            let config: Config = get_config(deps.as_ref()).unwrap();
            if contract_addr != config.token_address.unwrap() {
                return Err(ContractError::Unauthorized {});
            }

            let cw20_sender_addr = deps.api.addr_validate(&cw20_msg.sender)?;
            try_unbond(
                deps,
                contract_addr,
                config.asset_info,
                cw20_sender_addr,
                cw20_msg.amount,
            )
        }
        _ => Err(ContractError::MissingUnbondHook {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)),
        QueryMsg::StakerInfo { staker_addr } => to_binary(&query_stake_info(deps, staker_addr)),
        QueryMsg::Commission {} => to_binary(&query_commission(deps)),
        QueryMsg::Rate {} => to_binary(&query_rate(deps)),
    }
}

pub fn query_config(deps: Deps) -> ConfigResponse {
    let config = get_config(deps).unwrap();

    ConfigResponse {
        owner: config.owner.to_string(),
        commission_rate: config.commission_rate,
        asset_info: config.asset_info,
        token_code_id: config.token_code_id,
        token_address: match config.token_address {
            Some(token_address) => token_address.to_string(),
            None => String::default(),
        },
    }
}

fn query_stake_info(deps: Deps, staker_addr: String) -> StakerInfoResponse {
    let config = get_config(deps).unwrap();

    let staker = Addr::unchecked(staker_addr);
    let vaults_token_address = config.token_address.unwrap();

    let balance: Uint256 =
        match query_token_balance(&deps.querier, vaults_token_address.clone(), staker) {
            Ok(balance) => balance.into(),
            Err(_) => Uint256::zero(),
        };

    let rate = compute_share_rate(deps, vaults_token_address).unwrap();

    StakerInfoResponse {
        asset: Asset {
            info: config.asset_info,
            amount: rate.mul(balance).into(),
        },
    }
}

fn query_commission(deps: Deps) -> CommissionResponse {
    let config = get_config(deps).unwrap();

    let commission = get_commission(deps).unwrap();

    CommissionResponse {
        asset: Asset {
            info: config.asset_info,
            amount: (Uint256::one() * commission).into(),
        },
    }
}

fn query_rate(deps: Deps) -> RateResponse {
    let config = get_config(deps).unwrap();

    let rate = compute_share_rate(deps, config.token_address.unwrap()).unwrap();

    RateResponse { rate }
}

#[cfg(test)]
mod test_instantiate {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{attr, WasmMsg};
    use starflet_protocol::mock_querier::mock_dependencies;
    use terraswap::asset::AssetInfo::NativeToken;

    use std::str::FromStr;

    static OWNER: &str = "owner0000";
    static COMMISSION_RATE: &str = "0.1";
    static SYMBOL: &str = "TTN";
    static CODE_ID: u64 = 123u64;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);
        let asset_info = NativeToken {
            denom: "uusd".to_string(),
        };
        let msg = InstantiateMsg {
            commission_rate: Decimal256::from_str(COMMISSION_RATE).unwrap(),
            asset_info: asset_info.clone(),
            token_code_id: CODE_ID,
            symbol: SYMBOL.to_string(),
        };

        let info = mock_info(OWNER, &[]);

        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "instantiate"),
                attr("owner", OWNER.to_string()),
                attr("commission_rate", COMMISSION_RATE.to_string()),
            ]
        );

        assert_eq!(
            res.messages,
            vec![SubMsg::reply_on_success(
                CosmosMsg::Wasm(WasmMsg::Instantiate {
                    admin: Some(MOCK_CONTRACT_ADDR.to_string()),
                    code_id: CODE_ID,
                    funds: vec![],
                    label: "".to_string(),
                    msg: to_binary(&TokenInstantiateMsg {
                        name: format!("{} vaults", SYMBOL.to_string()),
                        symbol: format!("v{}", SYMBOL),
                        decimals: 6u8,
                        initial_balances: vec![],
                        mint: Some(MinterResponse {
                            minter: MOCK_CONTRACT_ADDR.to_string(),
                            cap: None,
                        }),
                    })
                    .unwrap(),
                }),
                MSG_REPLY_ID_TOKEN_INSTANT
            )]
        );

        let res_config = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res_config).unwrap();
        assert_eq!(OWNER.to_string(), config.owner);
        assert_eq!(
            COMMISSION_RATE.to_string(),
            config.commission_rate.to_string()
        );
        assert_eq!(asset_info, config.asset_info);
        assert_eq!(CODE_ID, config.token_code_id);
    }
}

#[cfg(test)]
mod config {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info};
    use starflet_protocol::mock_querier::mock_dependencies;
    use terraswap::asset::AssetInfo::NativeToken;

    use std::str::FromStr;

    static OWNER: &str = "owner0000";
    static COMMISSION_RATE: &str = "0.1";
    static SYMBOL: &str = "TTN";
    static CODE_ID: u64 = 123u64;

    static CHANGE_OWNER: &str = "owner0001";
    static CHANGE_COMMISSION_RATE: &str = "0.5";
    static CHANGE_CODE_ID: u64 = 456u64;

    fn init(mut deps: DepsMut) {
        let asset_info = NativeToken {
            denom: "uusd".to_string(),
        };
        let msg = InstantiateMsg {
            commission_rate: Decimal256::from_str(COMMISSION_RATE).unwrap(),
            asset_info,
            token_code_id: CODE_ID,
            symbol: SYMBOL.to_string(),
        };

        let info = mock_info(OWNER, &[]);

        instantiate(deps.branch(), mock_env(), info, msg).unwrap();
    }

    #[test]
    fn update_config_owner() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        let asset_info = NativeToken {
            denom: "uusd".to_string(),
        };

        let msg = ExecuteMsg::UpdateConfig {
            owner: Some(CHANGE_OWNER.to_string()),
            commission_rate: None,
            code_id: None,
        };

        let info = mock_info(OWNER, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let res_config = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res_config).unwrap();
        assert_eq!(CHANGE_OWNER.to_string(), config.owner);
        assert_eq!(
            COMMISSION_RATE.to_string(),
            config.commission_rate.to_string()
        );
        assert_eq!(asset_info, config.asset_info);
        assert_eq!(CODE_ID, config.token_code_id);
    }

    #[test]
    #[should_panic]
    fn update_config_diffrent_owner_will_panic() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        let msg = ExecuteMsg::UpdateConfig {
            owner: Some(CHANGE_OWNER.to_string()),
            commission_rate: None,
            code_id: None,
        };

        let info = mock_info(CHANGE_OWNER, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    }

    #[test]
    fn update_config_multiple_commission_rate_and_owner() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        let change_asset_info = NativeToken {
            denom: "uusd".to_string(),
        };

        let msg = ExecuteMsg::UpdateConfig {
            owner: Some(CHANGE_OWNER.to_string()),
            commission_rate: Some(Decimal256::from_str(CHANGE_COMMISSION_RATE).unwrap()),
            code_id: None,
        };

        let info = mock_info(OWNER, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let res_config = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res_config).unwrap();
        assert_eq!(CHANGE_OWNER.to_string(), config.owner);
        assert_eq!(
            CHANGE_COMMISSION_RATE.to_string(),
            config.commission_rate.to_string()
        );
        assert_eq!(change_asset_info, config.asset_info);
        assert_eq!(CODE_ID, config.token_code_id);
    }

    #[test]
    fn update_config_code_id() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        let asset_info = NativeToken {
            denom: "uusd".to_string(),
        };

        let msg = ExecuteMsg::UpdateConfig {
            owner: None,
            commission_rate: None,
            code_id: Some(CHANGE_CODE_ID),
        };

        let info = mock_info(OWNER, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        let res_config = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res_config).unwrap();
        assert_eq!(OWNER.to_string(), config.owner);
        assert_eq!(
            COMMISSION_RATE.to_string(),
            config.commission_rate.to_string()
        );
        assert_eq!(asset_info, config.asset_info);
        assert_eq!(CHANGE_CODE_ID, config.token_code_id);
    }
}

#[cfg(test)]
mod reply {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{attr, coins, ContractResult, SubMsgExecutionResponse};
    use starflet_protocol::mock_querier::mock_dependencies;
    use terraswap::asset::AssetInfo::NativeToken;

    use crate::response::MsgExecuteContractResponse;

    use std::str::FromStr;

    static OWNER: &str = "owner0000";
    static COMMISSION_RATE: &str = "0.1";
    static SYMBOL: &str = "TTN";
    static CODE_ID: u64 = 123u64;

    fn init(mut deps: DepsMut) {
        let asset_info = NativeToken {
            denom: "uusd".to_string(),
        };
        let msg = InstantiateMsg {
            commission_rate: Decimal256::from_str(COMMISSION_RATE).unwrap(),
            asset_info,
            token_code_id: CODE_ID,
            symbol: SYMBOL.to_string(),
        };

        let info = mock_info(OWNER, &[]);

        instantiate(deps.branch(), mock_env(), info, msg).unwrap();
    }

    #[test]
    #[should_panic]
    fn unknown_reply_id_will_panic() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        let mut res = MsgInstantiateContractResponse::new();
        res.set_contract_address(MOCK_CONTRACT_ADDR.to_string());

        let reply_msg = Reply {
            id: 123u64,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                data: Some(res.write_to_bytes().unwrap().into()),
            }),
        };

        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();
    }

    #[test]
    fn token_instant_reply() {
        let mut deps = mock_dependencies(&coins(100u128, "uusd"));
        init(deps.as_mut());

        let mut res = MsgInstantiateContractResponse::new();
        res.set_contract_address(MOCK_CONTRACT_ADDR.to_string());

        let reply_msg = Reply {
            id: MSG_REPLY_ID_TOKEN_INSTANT,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                data: Some(res.write_to_bytes().unwrap().into()),
            }),
        };

        let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("reply", "token_instant"),
                attr("token_address", MOCK_CONTRACT_ADDR),
            ]
        );

        let config = get_config(deps.as_ref()).unwrap();
        assert_eq!(
            config.token_address.unwrap().to_string(),
            MOCK_CONTRACT_ADDR
        )
    }

    #[test]
    fn revenue_execute_reply() {
        let mut deps = mock_dependencies(&coins(100u128, "uusd"));
        init(deps.as_mut());

        let mut res = MsgInstantiateContractResponse::new();
        res.set_contract_address(MOCK_CONTRACT_ADDR.to_string());

        let reply_msg = Reply {
            id: MSG_REPLY_ID_TOKEN_INSTANT,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                data: Some(res.write_to_bytes().unwrap().into()),
            }),
        };
        reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        let res = MsgExecuteContractResponse::new();

        let reply_msg = Reply {
            id: MSG_REPLY_ID_EXECUTE,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                data: Some(res.write_to_bytes().unwrap().into()),
            }),
        };

        assert_eq!(Decimal256::zero(), get_vaults(deps.as_ref()).unwrap());

        let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("reply", "execute"),
                attr("result", "success"),
                attr(
                    "revenue",
                    Decimal256::from_uint256(Uint256::from(100u128)).to_string()
                ),
                attr(
                    "add_commission",
                    Decimal256::from_uint256(Uint256::from(10u128)).to_string()
                ),
            ]
        );

        assert_eq!(
            Decimal256::from_uint256(Uint256::from(100u128)),
            get_vaults(deps.as_ref()).unwrap()
        );

        assert_eq!(
            Decimal256::from_uint256(Uint256::from(10u128)),
            get_commission(deps.as_ref()).unwrap()
        );
    }

    #[test]
    fn loss_execute_reply() {
        let mut deps = mock_dependencies(&coins(100u128, "uusd"));
        init(deps.as_mut());

        let mut res = MsgInstantiateContractResponse::new();
        res.set_contract_address(MOCK_CONTRACT_ADDR.to_string());

        let res = MsgExecuteContractResponse::new();
        let reply_msg = Reply {
            id: MSG_REPLY_ID_MUST_EXECUTE,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                data: Some(res.write_to_bytes().unwrap().into()),
            }),
        };

        set_vaults(
            deps.as_mut(),
            Decimal256::from_uint256(Uint256::from(110u128)),
        )
        .unwrap();
        add_commission(
            deps.as_mut(),
            Decimal256::from_uint256(Uint256::from(10u128)),
        )
        .unwrap();

        let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("reply", "execute"),
                attr("result", "fail"),
                attr(
                    "loss",
                    Decimal256::from_uint256(Uint256::from(10u128)).to_string()
                ),
            ]
        );

        assert_eq!(
            Decimal256::from_uint256(Uint256::from(100u128)),
            get_vaults(deps.as_ref()).unwrap()
        );

        assert_eq!(
            Decimal256::from_uint256(Uint256::from(0u128)),
            get_commission(deps.as_ref()).unwrap()
        );
    }

    #[test]
    fn skip_execute_reply() {
        let mut deps = mock_dependencies(&coins(100u128, "uusd"));
        init(deps.as_mut());

        let mut res = MsgInstantiateContractResponse::new();
        res.set_contract_address(MOCK_CONTRACT_ADDR.to_string());

        let res = MsgExecuteContractResponse::new();

        let reply_msg = Reply {
            id: MSG_REPLY_ID_EXECUTE_SKIP,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                data: Some(res.write_to_bytes().unwrap().into()),
            }),
        };

        let res = reply(deps.as_mut(), mock_env(), reply_msg).unwrap();

        assert_eq!(res.attributes, vec![attr("reply", "execute_skip"),]);

        assert_eq!(
            Decimal256::from_uint256(Uint256::from(0u128)),
            get_vaults(deps.as_ref()).unwrap()
        );

        assert_eq!(
            Decimal256::from_uint256(Uint256::from(0u128)),
            get_commission(deps.as_ref()).unwrap()
        );
    }
}

#[cfg(test)]
mod bond {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{attr, coins, ContractResult, SubMsgExecutionResponse};
    use starflet_protocol::mock_querier::mock_dependencies;
    use terraswap::asset::AssetInfo::NativeToken;

    use crate::response::MsgInstantiateContractResponse;

    use std::str::FromStr;

    static OWNER: &str = "owner0000";
    static COMMISSION_RATE: &str = "0.1";
    static SYMBOL: &str = "TTN";
    static CODE_ID: u64 = 123u64;

    static BONDER1: &str = "bonder0000";
    static BONDER1_AMOUNT: u128 = 100u128;

    fn init(mut deps: DepsMut) {
        let asset_info = NativeToken {
            denom: "uusd".to_string(),
        };
        let msg = InstantiateMsg {
            commission_rate: Decimal256::from_str(COMMISSION_RATE).unwrap(),
            asset_info,
            token_code_id: CODE_ID,
            symbol: SYMBOL.to_string(),
        };

        let info = mock_info(OWNER, &[]);

        instantiate(deps.branch(), mock_env(), info, msg).unwrap();

        let mut res = MsgInstantiateContractResponse::new();
        res.set_contract_address(MOCK_CONTRACT_ADDR.to_string());

        let reply_msg = Reply {
            id: MSG_REPLY_ID_TOKEN_INSTANT,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                data: Some(res.write_to_bytes().unwrap().into()),
            }),
        };

        reply(deps.branch(), mock_env(), reply_msg).unwrap();
    }

    #[test]
    fn normal_bond() {
        let mut deps = mock_dependencies(&[]);
        deps.querier.with_token_balances(&[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &[(&BONDER1.to_string(), &Uint128::from(0u128))],
        )]);

        init(deps.as_mut());

        let bond_asset = Asset {
            amount: Uint128::from(BONDER1_AMOUNT),
            info: NativeToken {
                denom: "uusd".to_string(),
            },
        };

        let bond_msg = ExecuteMsg::Bond {
            asset: bond_asset.clone(),
        };

        let info = mock_info(BONDER1, &coins(100, "uusd"));

        let res = execute(deps.as_mut(), mock_env(), info, bond_msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "bond"),
                attr("bonder", BONDER1.to_string()),
                attr("asset", bond_asset.to_string()),
                attr("mint_amount", bond_asset.amount.to_string()),
            ]
        );

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: BONDER1.to_string(),
                    amount: bond_asset.amount,
                })
                .unwrap(),
            }))]
        );

        assert_eq!(
            Decimal256::from_uint256(Uint256::from(100u128)),
            get_vaults(deps.as_ref()).unwrap()
        );
    }

    #[test]
    #[should_panic]
    fn insufficient_bond_will_panic() {
        let mut deps = mock_dependencies(&[]);
        deps.querier.with_token_balances(&[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &[(&BONDER1.to_string(), &Uint128::from(0u128))],
        )]);

        let bond_asset = Asset {
            amount: Uint128::from(BONDER1_AMOUNT),
            info: NativeToken {
                denom: "uusd".to_string(),
            },
        };

        let bond_msg = ExecuteMsg::Bond { asset: bond_asset };

        let info = mock_info(BONDER1, &coins(99, "uusd"));

        execute(deps.as_mut(), mock_env(), info, bond_msg).unwrap();
    }
}

#[cfg(test)]
mod unbond {
    use super::*;
    use crate::response::MsgInstantiateContractResponse;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{attr, coins, ContractResult, SubMsgExecutionResponse};
    use starflet_protocol::mock_querier::mock_dependencies;
    use terraswap::asset::AssetInfo::NativeToken;

    use std::str::FromStr;

    static OWNER: &str = "owner0000";
    static COMMISSION_RATE: &str = "0.1";
    static SYMBOL: &str = "TTN";
    static CODE_ID: u64 = 123u64;

    static BONDER1: &str = "bonder0000";
    static BONDER1_AMOUNT: u128 = 100u128;
    static UNBONDER1_AMOUNT: u128 = 10u128;

    fn init(mut deps: DepsMut) {
        let asset_info = NativeToken {
            denom: "uusd".to_string(),
        };
        let msg = InstantiateMsg {
            commission_rate: Decimal256::from_str(COMMISSION_RATE).unwrap(),
            asset_info,
            token_code_id: CODE_ID,
            symbol: SYMBOL.to_string(),
        };

        let info = mock_info(OWNER, &[]);

        instantiate(deps.branch(), mock_env(), info, msg).unwrap();

        let mut res = MsgInstantiateContractResponse::new();
        res.set_contract_address(MOCK_CONTRACT_ADDR.to_string());

        let reply_msg = Reply {
            id: MSG_REPLY_ID_TOKEN_INSTANT,
            result: ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![],
                data: Some(res.write_to_bytes().unwrap().into()),
            }),
        };

        reply(deps.branch(), mock_env(), reply_msg).unwrap();

        let bond_asset = Asset {
            amount: Uint128::from(BONDER1_AMOUNT),
            info: NativeToken {
                denom: "uusd".to_string(),
            },
        };

        let bond_msg = ExecuteMsg::Bond { asset: bond_asset };

        let info = mock_info(BONDER1, &coins(100, "uusd"));

        execute(deps, mock_env(), info, bond_msg).unwrap();
    }

    #[test]
    fn normal_unbond() {
        let mut deps = mock_dependencies(&[]);
        deps.querier.with_token_balances(&[(
            &MOCK_CONTRACT_ADDR.to_string(),
            &[(&BONDER1.to_string(), &Uint128::from(0u128))],
        )]);

        init(deps.as_mut());

        let unbond_msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender: BONDER1.to_string(),
            amount: Uint128::from(UNBONDER1_AMOUNT),
            msg: to_binary(&Cw20HookMsg::Unbond {}).unwrap(),
        });

        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);

        let res = execute(deps.as_mut(), mock_env(), info, unbond_msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "unbond"),
                attr("unbonder", BONDER1.to_string()),
                attr("asset", UNBONDER1_AMOUNT.to_string() + &"uusd".to_string()),
                attr("burn_amount", UNBONDER1_AMOUNT.to_string()),
            ]
        );

        assert_eq!(
            Decimal256::from_uint256(Uint256::from(90u128)),
            get_vaults(deps.as_ref()).unwrap()
        );
    }
}

#[cfg(test)]
mod claim {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{attr, coins, BankMsg};
    use starflet_protocol::mock_querier::mock_dependencies;
    use terraswap::asset::AssetInfo::NativeToken;

    use crate::response::MsgInstantiateContractResponse;

    use std::str::FromStr;

    static OWNER: &str = "owner0000";
    static COMMISSION_RATE: &str = "0.1";
    static SYMBOL: &str = "TTN";
    static CODE_ID: u64 = 123u64;

    static BONDER1: &str = "bonder0000";
    static CLAIM_BALANCE: Decimal256 = Decimal256::one();

    fn init(mut deps: DepsMut) {
        let asset_info = NativeToken {
            denom: "uusd".to_string(),
        };
        let msg = InstantiateMsg {
            commission_rate: Decimal256::from_str(COMMISSION_RATE).unwrap(),
            asset_info,
            token_code_id: CODE_ID,
            symbol: SYMBOL.to_string(),
        };

        let info = mock_info(OWNER, &[]);

        instantiate(deps.branch(), mock_env(), info, msg).unwrap();

        let mut res = MsgInstantiateContractResponse::new();
        res.set_contract_address(MOCK_CONTRACT_ADDR.to_string());

        add_commission(deps.branch(), CLAIM_BALANCE).unwrap();
    }

    #[test]
    fn normal_claim() {
        let mut deps = mock_dependencies(&[]);

        init(deps.as_mut());

        let res_commission = query_commission(deps.as_ref());

        let msg = ExecuteMsg::Claim {};

        let info = mock_info(OWNER, &[]);

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("claimer", OWNER.to_string()),
                attr("asset", res_commission.asset.to_string()),
            ]
        );

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: OWNER.to_string(),
                amount: coins(1, "uusd"),
            }))]
        );
    }

    #[test]
    fn unknown_owner_claim_will_err() {
        let mut deps = mock_dependencies(&[]);

        init(deps.as_mut());

        let msg = ExecuteMsg::Claim {};

        let info = mock_info(BONDER1, &[]);

        execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    }
}

#[cfg(test)]
mod rate {
    use super::*;
    use cosmwasm_std::testing::{mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coin, coins};
    use starflet_protocol::mock_querier::mock_dependencies;
    use terraswap::asset::AssetInfo::NativeToken;

    use crate::response::MsgInstantiateContractResponse;

    use std::str::FromStr;

    static OWNER: &str = "owner0000";
    static COMMISSION_RATE: &str = "0.1";
    static SYMBOL: &str = "TTN";
    static CODE_ID: u64 = 123u64;

    static BONDER0: &str = "bonder0000";
    static BONDER1: &str = "bonder0001";

    static VAULTS_TOKEN_CONTRACT: &str = "vaults0000";

    fn init(mut deps: DepsMut) {
        let asset_info = NativeToken {
            denom: "uusd".to_string(),
        };
        let msg = InstantiateMsg {
            commission_rate: Decimal256::from_str(COMMISSION_RATE).unwrap(),
            asset_info,
            token_code_id: CODE_ID,
            symbol: SYMBOL.to_string(),
        };

        let info = mock_info(OWNER, &[]);

        instantiate(deps.branch(), mock_env(), info, msg).unwrap();

        let mut res = MsgInstantiateContractResponse::new();
        res.set_contract_address(MOCK_CONTRACT_ADDR.to_string());

        let mut config = get_config(deps.as_ref()).unwrap();
        config.token_address = Some(Addr::unchecked(VAULTS_TOKEN_CONTRACT));
        set_config(deps, config).unwrap();
    }

    #[test]
    fn more_than_share_rate() {
        let mut deps = mock_dependencies(&coins(110u128, "uusd"));
        deps.querier
            .with_balance(&[(&MOCK_CONTRACT_ADDR.to_string(), vec![coin(110u128, "uusd")])]);
        deps.querier.with_token_balances(&[(
            &VAULTS_TOKEN_CONTRACT.to_string(),
            &[
                (&BONDER0.to_string(), &Uint128::from(1u128)),
                (&BONDER1.to_string(), &Uint128::from(9u128)),
            ],
        )]);

        init(deps.as_mut());

        set_vaults(
            deps.as_mut(),
            Decimal256::from_uint256(Uint256::from(110u128)),
        )
        .unwrap();
        add_commission(
            deps.as_mut(),
            Decimal256::from_uint256(Uint256::from(10u128)),
        )
        .unwrap();

        let bonder0_balance = query_stake_info(deps.as_ref(), BONDER0.to_string());
        assert_eq!("10uusd", bonder0_balance.asset.to_string(),);

        let bonder1_balance = query_stake_info(deps.as_ref(), BONDER1.to_string());
        assert_eq!("90uusd", bonder1_balance.asset.to_string());
    }

    #[test]
    fn less_than_share_rate() {
        let mut deps = mock_dependencies(&coins(110u128, "uusd"));
        deps.querier
            .with_balance(&[(&MOCK_CONTRACT_ADDR.to_string(), vec![coin(110u128, "uusd")])]);

        deps.querier.with_token_balances(&[(
            &VAULTS_TOKEN_CONTRACT.to_string(),
            &[
                (&BONDER0.to_string(), &Uint128::from(10000u128)),
                (&BONDER1.to_string(), &Uint128::from(90000u128)),
            ],
        )]);

        init(deps.as_mut());

        set_vaults(
            deps.as_mut(),
            Decimal256::from_uint256(Uint256::from(110u128)),
        )
        .unwrap();
        add_commission(
            deps.as_mut(),
            Decimal256::from_uint256(Uint256::from(10u128)),
        )
        .unwrap();

        let bonder0_balance = query_stake_info(deps.as_ref(), BONDER0.to_string());
        assert_eq!("10uusd", bonder0_balance.asset.to_string(),);

        let bonder1_balance = query_stake_info(deps.as_ref(), BONDER1.to_string());
        assert_eq!("90uusd", bonder1_balance.asset.to_string());
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
