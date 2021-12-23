use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_bignumber::Decimal256;
use cosmwasm_std::{Addr, Deps, DepsMut, StdResult};
use cw_storage_plus::Item;

use std::ops::Sub;
use terraswap::asset::AssetInfo;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub commission_rate: Decimal256,
    pub asset_info: AssetInfo,
    pub token_code_id: u64,
    pub token_address: Option<Addr>,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub fn get_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

pub fn set_config(deps: DepsMut, config: Config) -> StdResult<()> {
    CONFIG.save(deps.storage, &config)
}

pub const VAULTS: Item<Decimal256> = Item::new("vaults");
pub const COMMISSION: Item<Decimal256> = Item::new("commission");

pub fn init(deps: DepsMut) -> StdResult<()> {
    VAULTS.save(deps.storage, &Decimal256::zero()).unwrap();
    COMMISSION.save(deps.storage, &Decimal256::zero())
}

pub fn add_vaults(deps: DepsMut, amount: Decimal256) -> StdResult<()> {
    let mut valuts = VAULTS.load(deps.storage).unwrap();
    valuts += amount;

    VAULTS.save(deps.storage, &valuts)
}

pub fn sub_vaults(deps: DepsMut, amount: Decimal256) -> StdResult<()> {
    let mut valuts = VAULTS.load(deps.storage).unwrap();
    valuts = valuts.sub(amount);

    VAULTS.save(deps.storage, &valuts)
}

pub fn set_vaults(deps: DepsMut, amount: Decimal256) -> StdResult<()> {
    VAULTS.save(deps.storage, &amount)
}

pub fn get_vaults(deps: Deps) -> StdResult<Decimal256> {
    VAULTS.load(deps.storage)
}

pub fn add_commission(deps: DepsMut, amount: Decimal256) -> StdResult<()> {
    let mut commission = COMMISSION.load(deps.storage).unwrap();
    commission += amount;

    COMMISSION.save(deps.storage, &commission)
}

pub fn sub_commission(deps: DepsMut, amount: Decimal256) -> StdResult<()> {
    let mut commission = COMMISSION.load(deps.storage).unwrap();
    commission = commission.sub(amount);

    COMMISSION.save(deps.storage, &commission)
}

pub fn sub_all_commission(deps: DepsMut) -> StdResult<Decimal256> {
    let commission = get_commission(deps.as_ref()).unwrap();
    COMMISSION.save(deps.storage, &Decimal256::zero()).unwrap();

    Ok(commission)
}

pub fn get_commission(deps: Deps) -> StdResult<Decimal256> {
    COMMISSION.load(deps.storage)
}

#[cfg(test)]
mod vaults {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn test_store_add_commission() {
        let mut deps = mock_dependencies(&[]);

        let amount = Decimal256::one();

        init(deps.as_mut()).unwrap();

        let res = add_vaults(deps.as_mut(), amount);
        assert!(res.is_ok());

        let res = get_vaults(deps.as_ref());
        assert_eq!(res.unwrap(), amount);
    }

    #[test]
    fn test_store_sub_commission() {
        let mut deps = mock_dependencies(&[]);

        let amount = Decimal256::one();

        init(deps.as_mut()).unwrap();

        let res = add_vaults(deps.as_mut(), amount);
        assert!(res.is_ok());

        let res = add_vaults(deps.as_mut(), amount);
        assert!(res.is_ok());

        let res = get_vaults(deps.as_ref());
        assert_eq!(res.unwrap(), amount + amount);

        let res = sub_vaults(deps.as_mut(), amount);
        assert!(res.is_ok());

        let res = get_vaults(deps.as_ref());
        assert_eq!(res.unwrap(), amount);
    }
}

#[cfg(test)]
mod commission {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;

    #[test]
    fn test_store_add_commission() {
        let mut deps = mock_dependencies(&[]);

        let amount = Decimal256::one();

        init(deps.as_mut()).unwrap();

        let res = add_commission(deps.as_mut(), amount);
        assert!(res.is_ok());

        let res = get_commission(deps.as_ref());
        assert_eq!(res.unwrap(), amount);
    }

    #[test]
    fn test_store_all_commission() {
        let mut deps = mock_dependencies(&[]);

        let amount = Decimal256::one();

        init(deps.as_mut()).unwrap();

        let res = add_commission(deps.as_mut(), amount);
        assert!(res.is_ok());

        let res = add_commission(deps.as_mut(), amount);
        assert!(res.is_ok());

        let res = get_commission(deps.as_ref());
        assert_eq!(res.unwrap(), amount + amount);

        let res = sub_all_commission(deps.as_mut());
        assert!(res.is_ok());
    }
}
