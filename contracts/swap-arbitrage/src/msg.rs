use cosmwasm_bignumber::Decimal256;
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
    pub terraswap_router: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    UpdateConfig {
        owner: Option<String>,
        commission_rate: Option<Decimal256>,
        code_id: Option<u64>,
        terraswap_router: Option<String>,
    },
    Bond {
        asset: Asset,
    },
    Swap {
        route_path: RoutePath,
    },
    Claim {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoutePath {
    NativeSwapToTerraSwap,
    TerraSwapToNativeSwap,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub commission_rate: Decimal256,
    pub asset_info: AssetInfo,
    pub token_code_id: u64,
    pub token_address: String,
    pub terraswap_router: String,
}
