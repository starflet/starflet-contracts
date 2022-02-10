use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::{Asset, AssetInfo};

pub const MSG_REPLY_PREPARE_SWAP: u64 = 11;
pub const MSG_REPLY_SWAP: u64 = 12;
pub const MSG_REPLY_BOND: u64 = 21;
pub const MSG_REPLY_UNBOND: u64 = 31;
pub const MSG_REPLY_CLAIM: u64 = 41;
pub const MSG_REPLY_MIGRATE: u64 = 51;
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub commission_rate: Decimal256,
    pub deposit_asset_info: AssetInfo,
    pub asset_info: AssetInfo,
    pub symbol: String,
    pub token_code_id: u64,
    pub router_addr: String,
    pub money_market_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    UpdateConfig {
        owner: Option<String>,
        commission_rate: Option<Decimal256>,
        code_id: Option<u64>,
        router_addr: Option<String>,
    },
    Bond {
        asset: Asset,
    },
    Swap {
        path: String,
        amount: Uint128,
    },
    Claim {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Pair {
    pub name: String,
    pub pair_addr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Pairs {
    pub pairs: Vec<Pair>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub commission_rate: Decimal256,
    pub asset_info: AssetInfo,
    pub token_code_id: u64,
    pub token_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct MigrateMsg {
    pub money_market_addr: String,
    pub asset_info: AssetInfo,
    pub deposit_asset_info: AssetInfo,
}
