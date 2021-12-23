use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::CosmosMsg;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::{Asset, AssetInfo};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub commission_rate: Decimal256,
    pub asset_info: AssetInfo,
    pub symbol: String,
    pub token_code_id: u64,
}

#[derive(Serialize, strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Action {
    Instantiate,
    UpdateConfig,
    Bond,
    Unbond,
    Execute,
    Claim,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    UpdateConfig {
        owner: Option<String>,
        commission_rate: Option<Decimal256>,
        code_id: Option<u64>,
    },
    Bond {
        asset: Asset,
    },
    Execute {
        msg: CosmosMsg,
        is_distribute: bool,
    },
    Claim {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Return stable coins to a user
    /// according to exchange rate
    Unbond {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    StakerInfo { staker_addr: String },
    Commission {},
    Rate {},
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
pub struct StakerInfoResponse {
    pub asset: Asset,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CommissionResponse {
    pub asset: Asset,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RateResponse {
    pub rate: Decimal256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
