use std::{collections::HashMap, str::FromStr};

use cosmwasm_std::{
    attr,
    testing::{mock_env, mock_info},
    Addr, Decimal, Empty, Event, Order, StdResult, Uint128,
};
use cw2::{ContractVersion, VersionError};
use mars_red_bank::{
    contract::{execute, migrate},
    error::ContractError,
    migrations::v2_0_0::v1_state,
    state::{COLLATERALS, CONFIG, MARKETS, MIGRATION_GUARD, OWNER},
};
use mars_testing::mock_dependencies;
use mars_types::{
    keys::{UserId, UserIdKey},
    red_bank::{Collateral, ExecuteMsg, InterestRateModel, Market, MigrateV1ToV2},
};
use mars_utils::error::GuardError;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "1.2.1").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-red-bank".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-red-bank", "4.1.0").unwrap();

    let err = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "1.2.1".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn full_migration() {
    let mut deps = mock_dependencies(&[]);
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-red-bank", "1.2.1").unwrap();

    let old_owner = "spiderman_246";
    let deps_muted = deps.as_mut();
    OWNER
        .initialize(
            deps_muted.storage,
            deps_muted.api,
            mars_owner::OwnerInit::SetInitialOwner {
                owner: old_owner.to_string(),
            },
        )
        .unwrap();

    let v1_config = v1_state::Config {
        address_provider: Addr::unchecked("address_provider"),
        close_factor: Decimal::percent(50),
    };
    v1_state::CONFIG.save(deps.as_mut().storage, &v1_config).unwrap();

    let atom_market = atom_market();
    v1_state::MARKETS.save(deps.as_mut().storage, &atom_market.denom, &atom_market).unwrap();
    let lp_market = lp_market();
    v1_state::MARKETS.save(deps.as_mut().storage, &lp_market.denom, &lp_market).unwrap();
    let osmo_market = osmo_market();
    v1_state::MARKETS.save(deps.as_mut().storage, &osmo_market.denom, &osmo_market).unwrap();

    let user_1_atom_collateral = Collateral {
        amount_scaled: Uint128::new(12345),
        enabled: true,
    };
    v1_state::COLLATERALS
        .save(deps.as_mut().storage, (&Addr::unchecked("user_1"), "uatom"), &user_1_atom_collateral)
        .unwrap();
    let user_1_osmo_collateral = Collateral {
        amount_scaled: Uint128::new(345678903),
        enabled: false,
    };
    v1_state::COLLATERALS
        .save(deps.as_mut().storage, (&Addr::unchecked("user_1"), "uosmo"), &user_1_osmo_collateral)
        .unwrap();
    let user_2_atom_collateral = Collateral {
        amount_scaled: Uint128::new(1),
        enabled: true,
    };
    v1_state::COLLATERALS
        .save(deps.as_mut().storage, (&Addr::unchecked("user_2"), "uatom"), &user_2_atom_collateral)
        .unwrap();

    let res = migrate(deps.as_mut(), mock_env(), Empty {}).unwrap();

    assert_eq!(res.messages, vec![]);
    assert_eq!(res.events, vec![] as Vec<Event>);
    assert!(res.data.is_none());
    assert_eq!(
        res.attributes,
        vec![attr("action", "migrate"), attr("from_version", "1.2.1"), attr("to_version", "2.0.1")]
    );

    let new_contract_version = ContractVersion {
        contract: "crates.io:mars-red-bank".to_string(),
        version: "2.0.1".to_string(),
    };
    assert_eq!(cw2::get_contract_version(deps.as_ref().storage).unwrap(), new_contract_version);

    let config = CONFIG.load(&deps.storage).unwrap();
    assert_eq!(v1_config.address_provider, config.address_provider);

    // check markets data, lp tokens should be filtered out
    let markets = MARKETS
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<StdResult<HashMap<_, _>>>()
        .unwrap();
    assert_eq!(markets.len(), 2);
    assert!(compare_markers(&atom_market, markets.get(&atom_market.denom).unwrap()));
    assert!(compare_markers(&osmo_market, markets.get(&osmo_market.denom).unwrap()));

    // check if guard is active for user actions
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("depositor", &[]),
        ExecuteMsg::Deposit {
            account_id: None,
            on_behalf_of: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Guard(GuardError::Active {}));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("withdrawer", &[]),
        ExecuteMsg::Withdraw {
            denom: "uosmo".to_string(),
            amount: None,
            recipient: None,
            account_id: None,
            liquidation_related: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Guard(GuardError::Active {}));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("borrower", &[]),
        ExecuteMsg::Borrow {
            denom: "uosmo".to_string(),
            amount: Uint128::one(),
            recipient: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Guard(GuardError::Active {}));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("borrower", &[]),
        ExecuteMsg::Repay {
            on_behalf_of: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Guard(GuardError::Active {}));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("liquidator", &[]),
        ExecuteMsg::Liquidate {
            user: "liquidatee".to_string(),
            collateral_denom: "uosmo".to_string(),
            recipient: None,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Guard(GuardError::Active {}));

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("liquidator", &[]),
        ExecuteMsg::UpdateAssetCollateralStatus {
            denom: "uosmo".to_string(),
            enable: false,
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Guard(GuardError::Active {}));

    // non-owner is unauthorized to clear state
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("random_user", &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::ClearV1State {}),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Owner(mars_owner::OwnerError::NotOwner {}));

    // can't clear old V1 collaterals state if migration in progress - guard is active
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(old_owner, &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::ClearV1State {}),
    )
    .unwrap_err();
    assert_eq!(err, ContractError::Guard(GuardError::Active {}));

    // check users collaterals after using `migrate` entrypoint
    assert!(!v1_state::COLLATERALS.is_empty(&deps.storage));
    assert!(COLLATERALS.is_empty(&deps.storage));

    // migrate collaterals
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("ranom_user", &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::Collaterals {
            limit: 100,
        }),
    )
    .unwrap();

    // check v1 users collaterals after full migration
    assert!(!v1_state::COLLATERALS.is_empty(&deps.storage));

    // check users collaterals with new user key (addr + account id)
    let collaterals = COLLATERALS
        .range(deps.as_ref().storage, None, None, Order::Ascending)
        .collect::<StdResult<HashMap<_, _>>>()
        .unwrap();
    assert_eq!(collaterals.len(), 3);
    let user_id = UserId::credit_manager(Addr::unchecked("user_1"), "".to_string());
    let user_1_id_key: UserIdKey = user_id.try_into().unwrap();
    assert_eq!(
        collaterals.get(&(user_1_id_key.clone(), "uatom".to_string())).unwrap(),
        &user_1_atom_collateral
    );
    assert_eq!(
        collaterals.get(&(user_1_id_key, "uosmo".to_string())).unwrap(),
        &user_1_osmo_collateral
    );
    let user_id = UserId::credit_manager(Addr::unchecked("user_2"), "".to_string());
    let user_2_id_key: UserIdKey = user_id.try_into().unwrap();
    assert_eq!(
        collaterals.get(&(user_2_id_key, "uatom".to_string())).unwrap(),
        &user_2_atom_collateral
    );

    // Clear old V1 collaterals state
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info(old_owner, &[]),
        ExecuteMsg::Migrate(MigrateV1ToV2::ClearV1State {}),
    )
    .unwrap();

    // check users collaterals after clearing
    assert!(v1_state::COLLATERALS.is_empty(&deps.storage));
    assert!(!COLLATERALS.is_empty(&deps.storage));

    // guard should be unlocked after migration
    assert!(MIGRATION_GUARD.assert_unlocked(&deps.storage).is_ok());
}

fn atom_market() -> v1_state::Market {
    v1_state::Market {
        denom: "uatom".to_string(),
        reserve_factor: Decimal::percent(10),
        interest_rate_model: InterestRateModel {
            optimal_utilization_rate: Decimal::from_str("0.6").unwrap(),
            base: Decimal::zero(),
            slope_1: Decimal::from_str("0.15").unwrap(),
            slope_2: Decimal::from_str("3").unwrap(),
        },
        borrow_index: Decimal::from_str("1.095285439526354046").unwrap(),
        liquidity_index: Decimal::from_str("1.044176487308390288").unwrap(),
        borrow_rate: Decimal::from_str("0.196215745883701305").unwrap(),
        liquidity_rate: Decimal::from_str("0.121276949637996324").unwrap(),
        indexes_last_updated: 1695042123,
        collateral_total_scaled: Uint128::new(107605849836144570),
        debt_total_scaled: Uint128::new(70450559958286857),
        max_loan_to_value: Decimal::percent(60),
        liquidation_threshold: Decimal::percent(50),
        liquidation_bonus: Decimal::percent(5),
        deposit_enabled: true,
        borrow_enabled: true,
        deposit_cap: Uint128::MAX,
    }
}

fn lp_market() -> v1_state::Market {
    v1_state::Market {
        denom: "gamm/pool/1".to_string(),
        reserve_factor: Decimal::percent(10),
        interest_rate_model: InterestRateModel {
            optimal_utilization_rate: Decimal::from_str("0.6").unwrap(),
            base: Decimal::zero(),
            slope_1: Decimal::from_str("0.15").unwrap(),
            slope_2: Decimal::from_str("3").unwrap(),
        },
        borrow_index: Decimal::one(),
        liquidity_index: Decimal::one(),
        borrow_rate: Decimal::zero(),
        liquidity_rate: Decimal::zero(),
        indexes_last_updated: 1695042123,
        collateral_total_scaled: Uint128::zero(),
        debt_total_scaled: Uint128::zero(),
        max_loan_to_value: Decimal::percent(60),
        liquidation_threshold: Decimal::percent(50),
        liquidation_bonus: Decimal::percent(5),
        deposit_enabled: false,
        borrow_enabled: false,
        deposit_cap: Uint128::MAX,
    }
}

fn osmo_market() -> v1_state::Market {
    v1_state::Market {
        denom: "uosmo".to_string(),
        reserve_factor: Decimal::percent(10),
        interest_rate_model: InterestRateModel {
            optimal_utilization_rate: Decimal::from_str("0.6").unwrap(),
            base: Decimal::zero(),
            slope_1: Decimal::from_str("0.15").unwrap(),
            slope_2: Decimal::from_str("3").unwrap(),
        },
        borrow_index: Decimal::from_str("1.048833892520260924").unwrap(),
        liquidity_index: Decimal::from_str("1.012932497073055883").unwrap(),
        borrow_rate: Decimal::from_str("0.080575412566632138").unwrap(),
        liquidity_rate: Decimal::from_str("0.023372629597018728").unwrap(),
        indexes_last_updated: 1695042123,
        collateral_total_scaled: Uint128::new(3475219753161696357),
        debt_total_scaled: Uint128::new(1081729307695065417),
        max_loan_to_value: Decimal::percent(60),
        liquidation_threshold: Decimal::percent(50),
        liquidation_bonus: Decimal::percent(5),
        deposit_enabled: true,
        borrow_enabled: true,
        deposit_cap: Uint128::MAX,
    }
}

fn compare_markers(old_market: &v1_state::Market, market: &Market) -> bool {
    old_market.denom == market.denom
        && old_market.reserve_factor == market.reserve_factor
        && old_market.interest_rate_model == market.interest_rate_model
        && old_market.borrow_index == market.borrow_index
        && old_market.liquidity_index == market.liquidity_index
        && old_market.borrow_rate == market.borrow_rate
        && old_market.liquidity_rate == market.liquidity_rate
        && old_market.indexes_last_updated == market.indexes_last_updated
        && old_market.collateral_total_scaled == market.collateral_total_scaled
        && old_market.debt_total_scaled == market.debt_total_scaled
}
