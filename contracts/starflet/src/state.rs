use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Deps, DepsMut, Order, StdError, StdResult};
use cw_storage_plus::{Bound, Item, Map};

use starflet_protocol::starflet::PlanetResponse;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const PLANETS: Map<Addr, PlanetInfo> = Map::new("planet");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PlanetInfo {
    pub contract_addr: Addr,
    pub title: String,
    pub description: String,
}

impl PlanetInfo {
    pub fn to_normal(&self) -> StdResult<PlanetResponse> {
        Ok(PlanetResponse {
            contract_addr: self.contract_addr.to_string(),
            title: self.title.to_string(),
            description: self.description.to_string(),
        })
    }
}

const MAX_TITLE: usize = 80;
const MAX_DESCRIPTION: usize = 200;
pub fn store_planet(deps: DepsMut, planet_info: PlanetInfo) -> StdResult<()> {
    if planet_info.title.len() > MAX_TITLE {
        return Err(StdError::generic_err(format!(
            "Title must be less than {}. ({})",
            MAX_TITLE,
            planet_info.title.len()
        )));
    }

    if planet_info.description.len() > MAX_DESCRIPTION {
        return Err(StdError::generic_err(format!(
            "Description must be less than {}. ({})",
            MAX_DESCRIPTION,
            planet_info.description.len()
        )));
    }

    PLANETS.save(
        deps.storage,
        planet_info.contract_addr.clone(),
        &planet_info,
    )
}

pub fn load_planet(deps: Deps, contract_addr: Addr) -> StdResult<PlanetInfo> {
    PLANETS.load(deps.storage, contract_addr)
}

pub fn remove_planet(deps: DepsMut, contract_addr: Addr) {
    PLANETS.remove(deps.storage, contract_addr)
}

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn load_planets(
    deps: Deps,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<Vec<PlanetResponse>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let start = start_after.map(|s| Bound::exclusive(s.as_bytes().to_vec()));

    PLANETS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            v.to_normal()
        })
        .collect::<StdResult<Vec<PlanetResponse>>>()
}

#[cfg(test)]
mod planet {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    static POOL_CONTRACT: &str = "planet000";
    static TITLE: &str = "title";
    static DESCRIPTION: &str = "description";

    static CHANGE_TITLE: &str = "changed title";
    static CHANGE_DESCRIPTION: &str = "changed description";

    #[test]
    fn store_and_load_planet_and_planets() {
        let mut deps = mock_dependencies(&[]);

        let planet_info1 = PlanetInfo {
            contract_addr: Addr::unchecked(POOL_CONTRACT.to_string()),
            title: TITLE.to_string(),
            description: DESCRIPTION.to_string(),
        };

        store_planet(deps.as_mut(), planet_info1.clone()).unwrap();

        let res = load_planet(deps.as_ref(), Addr::unchecked(POOL_CONTRACT.to_string())).unwrap();
        assert_eq!(res, planet_info1);

        let res = load_planets(deps.as_ref(), None, None).unwrap();
        assert_eq!(
            res,
            vec![PlanetResponse {
                contract_addr: POOL_CONTRACT.to_string(),
                title: TITLE.to_string(),
                description: DESCRIPTION.to_string(),
            }]
        );

        remove_planet(deps.as_mut(), Addr::unchecked(POOL_CONTRACT.to_string()));

        load_planet(deps.as_ref(), Addr::unchecked(POOL_CONTRACT.to_string())).unwrap_err();
    }

    #[test]
    fn update_planet() {
        let mut deps = mock_dependencies(&[]);

        let planet_info1 = PlanetInfo {
            contract_addr: Addr::unchecked(POOL_CONTRACT.to_string()),
            title: TITLE.to_string(),
            description: DESCRIPTION.to_string(),
        };

        store_planet(deps.as_mut(), planet_info1.clone()).unwrap();

        let res = load_planet(deps.as_ref(), Addr::unchecked(POOL_CONTRACT.to_string())).unwrap();
        assert_eq!(res, planet_info1);

        let planet_info2 = PlanetInfo {
            contract_addr: Addr::unchecked(POOL_CONTRACT.to_string()),
            title: CHANGE_TITLE.to_string(),
            description: CHANGE_DESCRIPTION.to_string(),
        };

        store_planet(deps.as_mut(), planet_info2).unwrap();

        let res = load_planets(deps.as_ref(), None, None).unwrap();
        assert_eq!(
            res,
            vec![PlanetResponse {
                contract_addr: POOL_CONTRACT.to_string(),
                title: CHANGE_TITLE.to_string(),
                description: CHANGE_DESCRIPTION.to_string(),
            }]
        );
    }

    #[test]
    fn max_title_will_err() {
        let mut deps = mock_dependencies(&[]);

        let planet_info1 = PlanetInfo {
            contract_addr: Addr::unchecked(POOL_CONTRACT.to_string()),
            title: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            description: DESCRIPTION.to_string(),
        };

        store_planet(deps.as_mut(), planet_info1).unwrap_err();
    }

    #[test]
    fn max_description_will_err() {
        let mut deps = mock_dependencies(&[]);

        let planet_info1 = PlanetInfo {
            contract_addr: Addr::unchecked(POOL_CONTRACT.to_string()),
            title: TITLE.to_string(),
            description: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        };

        store_planet(deps.as_mut(), planet_info1).unwrap_err();
    }

    fn init(mut deps: DepsMut) {
        for i in 1..40 {
            store_planet(
                deps.branch(),
                PlanetInfo {
                    contract_addr: Addr::unchecked(format!("{}{:02}", POOL_CONTRACT, i)),
                    title: format!("{}{:02}", TITLE, i),
                    description: format!("{}{:02}", DESCRIPTION, i),
                },
            )
            .unwrap();
        }
    }

    #[test]
    fn range_planets_limit_1() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        let res = load_planets(deps.as_ref(), None, Some(1u32)).unwrap();
        assert_eq!(
            res,
            vec![PlanetResponse {
                contract_addr: "planet00001".to_string(),
                title: "title01".to_string(),
                description: "description01".to_string(),
            }]
        );
    }

    #[test]
    fn range_planets_limit_2() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        let res = load_planets(deps.as_ref(), None, Some(2u32)).unwrap();
        assert_eq!(
            res,
            vec![
                PlanetResponse {
                    contract_addr: "planet00001".to_string(),
                    title: "title01".to_string(),
                    description: "description01".to_string(),
                },
                PlanetResponse {
                    contract_addr: "planet00002".to_string(),
                    title: "title02".to_string(),
                    description: "description02".to_string(),
                }
            ]
        );
    }

    #[test]
    fn range_planets_start_after() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());
        let res = load_planets(
            deps.as_ref(),
            Some(Addr::unchecked("planet00010".to_string())),
            Some(3u32),
        )
        .unwrap();
        assert_eq!(
            res,
            vec![
                PlanetResponse {
                    contract_addr: "planet00011".to_string(),
                    title: "title11".to_string(),
                    description: "description11".to_string(),
                },
                PlanetResponse {
                    contract_addr: "planet00012".to_string(),
                    title: "title12".to_string(),
                    description: "description12".to_string(),
                },
                PlanetResponse {
                    contract_addr: "planet00013".to_string(),
                    title: "title13".to_string(),
                    description: "description13".to_string(),
                },
            ]
        );
    }

    #[test]
    fn range_planets_max_limit() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        // MAX_LIMIT
        let res = load_planets(deps.as_ref(), None, Some(31u32)).unwrap();
        assert_eq!(res.len() as u32, MAX_LIMIT);
    }

    #[test]
    fn range_planets_default_range() {
        let mut deps = mock_dependencies(&[]);
        init(deps.as_mut());

        // DEFAULT RAGNE
        let res = load_planets(deps.as_ref(), None, None).unwrap();
        assert_eq!(res.len() as u32, DEFAULT_LIMIT);
    }
}
