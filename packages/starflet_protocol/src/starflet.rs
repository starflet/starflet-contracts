use cosmwasm_std::Addr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, strum_macros::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Action {
    Instantiate,
    UpdateConfig,
    AddPlanet,
    EditPlanet,
    RemovePlanet,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig {
        admin: Option<String>,
    },
    AddPlanet {
        contract_addr: String,
        title: String,
        description: String,
    },
    EditPlanet {
        contract_addr: String,
        title: Option<String>,
        description: Option<String>,
    },
    RemovePlanet {
        contract_addr: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    Config {},
    Planet {
        planet_contract: Addr,
    },
    Planets {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub admin: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PlanetResponse {
    pub contract_addr: String,
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PlanetsResponse {
    pub planets: Vec<PlanetResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}
