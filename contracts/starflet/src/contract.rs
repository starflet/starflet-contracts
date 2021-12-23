#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Attribute, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::error::ContractError;
use crate::state::{
    load_planet, load_planets, remove_planet, store_planet, Config, PlanetInfo, CONFIG,
};
use starflet_protocol::starflet::{
    Action, ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, PlanetResponse,
    PlanetsResponse, QueryMsg,
};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        admin: info.sender.clone(),
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", Action::Instantiate.to_string())
        .add_attribute("admin", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { admin } => try_update_config(deps.branch(), info, admin),
        ExecuteMsg::AddPlanet {
            contract_addr,
            title,
            description,
        } => try_add_planet(deps.branch(), info, contract_addr, title, description),
        ExecuteMsg::EditPlanet {
            contract_addr,
            title,
            description,
        } => try_edit_planet(deps.branch(), info, contract_addr, title, description),
        ExecuteMsg::RemovePlanet { contract_addr } => {
            try_remove_planet(deps.branch(), info, contract_addr)
        }
    }
}

pub fn try_update_config(
    deps: DepsMut,
    info: MessageInfo,
    admin: Option<String>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let mut res: Vec<Attribute> = vec![Attribute::new("action", Action::UpdateConfig.to_string())];

    if let Some(admin) = admin {
        config.admin = deps.api.addr_validate(&admin).unwrap();
        res.push(Attribute::new("admin", config.admin.clone()));
    }

    CONFIG.save(deps.storage, &config).unwrap();

    Ok(Response::new().add_attributes(res))
}

pub fn try_add_planet(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: String,
    title: String,
    description: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let contract = deps.api.addr_validate(&contract_addr).unwrap();

    let planet_info = PlanetInfo {
        contract_addr: contract.clone(),
        title: title.clone(),
        description: description.clone(),
    };

    store_planet(deps, planet_info).unwrap();

    Ok(Response::new()
        .add_attribute("action", Action::AddPlanet.to_string())
        .add_attribute("contract_addr", contract)
        .add_attribute("title", title)
        .add_attribute("description", description))
}

pub fn try_edit_planet(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: String,
    title: Option<String>,
    description: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let contract = deps.api.addr_validate(&contract_addr).unwrap();

    let mut planet_info = load_planet(deps.as_ref(), contract.clone()).unwrap();
    let mut res: Vec<Attribute> = vec![
        Attribute::new("action", Action::EditPlanet.to_string()),
        Attribute::new("contract_addr", contract),
    ];

    if let Some(title) = title {
        planet_info.title = title.to_string();
        res.push(Attribute::new("title", title));
    }

    if let Some(description) = description {
        planet_info.description = description.to_string();
        res.push(Attribute::new("description", description));
    }

    store_planet(deps, planet_info).unwrap();

    Ok(Response::new().add_attributes(res))
}

pub fn try_remove_planet(
    mut deps: DepsMut,
    info: MessageInfo,
    contract_addr: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // permission check
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }

    let contract = deps.api.addr_validate(&contract_addr).unwrap();

    load_planet(deps.as_ref(), contract.clone()).unwrap();

    remove_planet(deps.branch(), contract.clone());

    Ok(Response::new()
        .add_attribute("action", Action::RemovePlanet.to_string())
        .add_attribute("contract_addr", contract))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Planet { planet_contract } => to_binary(&query_planet(deps, planet_contract)?),
        QueryMsg::Planets { start_after, limit } => {
            to_binary(&query_planets(deps, start_after, limit))
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = CONFIG.load(deps.storage)?;
    let res = ConfigResponse {
        admin: state.admin.to_string(),
    };

    Ok(res)
}

fn query_planet(deps: Deps, planet_contract: Addr) -> StdResult<PlanetResponse> {
    let planet_info = load_planet(deps, planet_contract).unwrap();

    planet_info.to_normal()
}

fn query_planets(deps: Deps, start_after: Option<Addr>, limit: Option<u32>) -> PlanetsResponse {
    let planets = load_planets(deps, start_after, limit).unwrap();

    PlanetsResponse { planets }
}

#[cfg(test)]
mod config {
    use super::*;
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    static OWNER: &str = "owner0000";

    static CHANGE_OWNER: &str = "owner0001";

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info(OWNER, &[]);

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(2, res.attributes.len());
        assert_eq!("action", res.attributes[0].key);
        assert_eq!("instantiate", res.attributes[0].value);
        assert_eq!("admin", res.attributes[1].key);
        assert_eq!(OWNER, res.attributes[1].value);

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(OWNER, value.admin);
    }

    #[test]
    fn init_and_update_admin() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info(OWNER, &[]);

        // we can just call .unwrap() to assert this was a success
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::UpdateConfig {
            admin: Some(CHANGE_OWNER.to_string()),
        };

        execute(deps.as_mut(), mock_env(), mock_info(OWNER, &[]), msg).unwrap();
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(CHANGE_OWNER, value.admin);
    }

    #[test]
    #[should_panic]
    fn init_and_update_admin_with_unknown_addr_will_panic() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info(OWNER, &[]);

        // we can just call .unwrap() to assert this was a success
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::UpdateConfig {
            admin: Some("x".to_string()),
        };

        execute(deps.as_mut(), mock_env(), mock_info(OWNER, &[]), msg).unwrap();
    }

    #[test]
    #[should_panic]
    fn diffrent_admin_changed_config_will_panic() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {};
        let info = mock_info(OWNER, &[]);

        // we can just call .unwrap() to assert this was a success
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = ExecuteMsg::UpdateConfig {
            admin: Some(CHANGE_OWNER.to_string()),
        };

        execute(deps.as_mut(), mock_env(), mock_info(CHANGE_OWNER, &[]), msg).unwrap();
    }
}

#[cfg(test)]
mod planet {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{attr, from_binary};

    static OWNER: &str = "owner0000";

    static POOL_CONTRACT: &str = "planet0000";
    static TITLE: &str = "TITLE";
    static DESCRIPTION: &str = "DESCRIPTION";

    static CHANGE_TITLE: &str = "CHANGE TITLE";
    static CHANGE_DESCRIPTION: &str = "CHANGE DESCRIPTION";

    fn init(deps: DepsMut) {
        let msg = InstantiateMsg {};
        instantiate(deps, mock_env(), mock_info(OWNER, &[]), msg).unwrap();
    }

    #[test]
    fn create_update_planet_and_query() {
        let mut deps = mock_dependencies(&[]);

        init(deps.as_mut());

        let msg = ExecuteMsg::AddPlanet {
            contract_addr: POOL_CONTRACT.to_string(),
            title: TITLE.to_string(),
            description: DESCRIPTION.to_string(),
        };

        let res = execute(deps.as_mut(), mock_env(), mock_info(OWNER, &[]), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "add_planet"),
                attr("contract_addr", POOL_CONTRACT),
                attr("title", TITLE),
                attr("description", DESCRIPTION),
            ]
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Planet {
                planet_contract: Addr::unchecked(POOL_CONTRACT.to_string()),
            },
        )
        .unwrap();
        let value: PlanetResponse = from_binary(&res).unwrap();
        assert_eq!(
            value,
            PlanetResponse {
                contract_addr: POOL_CONTRACT.to_string(),
                title: TITLE.to_string(),
                description: DESCRIPTION.to_string(),
            }
        );

        let msg = ExecuteMsg::EditPlanet {
            contract_addr: POOL_CONTRACT.to_string(),
            title: Some(CHANGE_TITLE.to_string()),
            description: Some(CHANGE_DESCRIPTION.to_string()),
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info(OWNER, &[]), msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "edit_planet"),
                attr("contract_addr", POOL_CONTRACT),
                attr("title", CHANGE_TITLE),
                attr("description", CHANGE_DESCRIPTION),
            ]
        );

        let msg = ExecuteMsg::RemovePlanet {
            contract_addr: POOL_CONTRACT.to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), mock_info(OWNER, &[]), msg).unwrap();

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "remove_planet"),
                attr("contract_addr", POOL_CONTRACT),
            ]
        );

        let res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Planets {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
        let value: PlanetsResponse = from_binary(&res).unwrap();
        assert_eq!(value.planets.len(), 0)
    }

    #[test]
    #[should_panic]
    fn remove_planet_query_will_panic() {
        let mut deps = mock_dependencies(&[]);

        init(deps.as_mut());

        let msg = ExecuteMsg::AddPlanet {
            contract_addr: POOL_CONTRACT.to_string(),
            title: TITLE.to_string(),
            description: DESCRIPTION.to_string(),
        };

        execute(deps.as_mut(), mock_env(), mock_info(OWNER, &[]), msg).unwrap();

        let msg = ExecuteMsg::RemovePlanet {
            contract_addr: POOL_CONTRACT.to_string(),
        };
        execute(deps.as_mut(), mock_env(), mock_info(OWNER, &[]), msg).unwrap();

        query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::Planet {
                planet_contract: Addr::unchecked(POOL_CONTRACT),
            },
        )
        .unwrap();
    }

    #[test]
    #[should_panic]
    fn unknown_admin_add_pair_will_panic() {
        let mut deps = mock_dependencies(&[]);

        init(deps.as_mut());

        let msg = ExecuteMsg::AddPlanet {
            contract_addr: POOL_CONTRACT.to_string(),
            title: TITLE.to_string(),
            description: DESCRIPTION.to_string(),
        };

        execute(deps.as_mut(), mock_env(), mock_info("A", &[]), msg).unwrap();
    }

    #[test]
    #[should_panic]
    fn unknown_admin_edit_pair_will_panic() {
        let mut deps = mock_dependencies(&[]);

        init(deps.as_mut());

        let msg = ExecuteMsg::EditPlanet {
            contract_addr: POOL_CONTRACT.to_string(),
            title: Some(TITLE.to_string()),
            description: Some(DESCRIPTION.to_string()),
        };

        execute(deps.as_mut(), mock_env(), mock_info("A", &[]), msg).unwrap();
    }

    #[test]
    #[should_panic]
    fn unknown_admin_remove_pair_will_panic() {
        let mut deps = mock_dependencies(&[]);

        init(deps.as_mut());

        let msg = ExecuteMsg::RemovePlanet {
            contract_addr: POOL_CONTRACT.to_string(),
        };

        execute(deps.as_mut(), mock_env(), mock_info("A", &[]), msg).unwrap();
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
