use cosmwasm_std::{
    testing::{mock_dependencies, mock_env},
    Addr, Decimal,
};
use cw2::VersionError;
use mars_credit_manager::{
    contract::migrate,
    migrations::v2_0_0::{v1_state, v1_state::OwnerSetNoneProposed},
    state::{
        ACCOUNT_NFT, HEALTH_CONTRACT, INCENTIVES, MAX_SLIPPAGE, OWNER, PARAMS, RED_BANK,
        REWARDS_COLLECTOR, SWAPPER,
    },
};
use mars_rover::{
    adapters::{
        health::HealthContractUnchecked, incentives::IncentivesUnchecked, params::ParamsUnchecked,
        swap::SwapperUnchecked,
    },
    error::ContractError,
    msg::{migrate::V2Updates, MigrateMsg},
};

pub mod helpers;

#[test]
fn wrong_contract_name() {
    let mut deps = mock_dependencies();
    cw2::set_contract_version(deps.as_mut().storage, "contract_xyz", "1.0.0").unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V1_0_0ToV2_0_0(V2Updates {
            health_contract: HealthContractUnchecked::new("health".to_string()),
            params: ParamsUnchecked::new("params".to_string()),
            incentives: IncentivesUnchecked::new("incentives".to_string()),
            swapper: SwapperUnchecked::new("swapper".to_string()),
            max_slippage: Decimal::percent(1),
        }),
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongContract {
            expected: "crates.io:mars-credit-manager".to_string(),
            found: "contract_xyz".to_string()
        })
    );
}

#[test]
fn wrong_contract_version() {
    let mut deps = mock_dependencies();
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-credit-manager", "4.1.0")
        .unwrap();

    let err = migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V1_0_0ToV2_0_0(V2Updates {
            health_contract: HealthContractUnchecked::new("health".to_string()),
            params: ParamsUnchecked::new("params".to_string()),
            incentives: IncentivesUnchecked::new("incentives".to_string()),
            swapper: SwapperUnchecked::new("swapper".to_string()),
            max_slippage: Decimal::percent(1),
        }),
    )
    .unwrap_err();

    assert_eq!(
        err,
        ContractError::Version(VersionError::WrongVersion {
            expected: "1.0.0".to_string(),
            found: "4.1.0".to_string()
        })
    );
}

#[test]
fn successful_migration() {
    let mut deps = mock_dependencies();
    cw2::set_contract_version(deps.as_mut().storage, "crates.io:mars-credit-manager", "1.0.0")
        .unwrap();

    let old_owner = "spiderman_246";
    v1_state::OWNER
        .save(
            deps.as_mut().storage,
            &v1_state::OwnerState::B(OwnerSetNoneProposed {
                owner: Addr::unchecked(old_owner),
            }),
        )
        .unwrap();

    let old_account_nft = "account_nft_addr_123";
    v1_state::ACCOUNT_NFT.save(deps.as_mut().storage, &Addr::unchecked(old_account_nft)).unwrap();

    let old_red_bank = "red-bank-addr-456";
    v1_state::RED_BANK.save(deps.as_mut().storage, &Addr::unchecked(old_red_bank)).unwrap();

    let health_contract = "health_addr_123".to_string();
    let params = "params_addr_456".to_string();
    let incentives = "incentives_addr_789".to_string();
    let swapper = "swapper_addr_012".to_string();
    let max_slippage = Decimal::percent(5);

    migrate(
        deps.as_mut(),
        mock_env(),
        MigrateMsg::V1_0_0ToV2_0_0(V2Updates {
            health_contract: HealthContractUnchecked::new(health_contract.clone()),
            params: ParamsUnchecked::new(params.clone()),
            incentives: IncentivesUnchecked::new(incentives.clone()),
            swapper: SwapperUnchecked::new(swapper.clone()),
            max_slippage,
        }),
    )
    .unwrap();

    let set_health_contract =
        HEALTH_CONTRACT.load(deps.as_ref().storage).unwrap().address().to_string();
    assert_eq!(health_contract, set_health_contract);

    let set_params = PARAMS.load(deps.as_ref().storage).unwrap().address().to_string();
    assert_eq!(params, set_params);

    let set_incentives = INCENTIVES.load(deps.as_ref().storage).unwrap().addr.to_string();
    assert_eq!(incentives, set_incentives);

    let set_swapper = SWAPPER.load(deps.as_ref().storage).unwrap().address().to_string();
    assert_eq!(swapper, set_swapper);

    let set_rewards = REWARDS_COLLECTOR.may_load(deps.as_ref().storage).unwrap();
    assert_eq!(None, set_rewards);

    let set_slippage = MAX_SLIPPAGE.load(deps.as_ref().storage).unwrap();
    assert_eq!(max_slippage, set_slippage);

    let o = OWNER.query(deps.as_ref().storage).unwrap();
    assert_eq!(old_owner.to_string(), o.owner.unwrap());
    assert!(o.proposed.is_none());
    assert!(o.initialized);
    assert!(!o.abolished);
    assert!(o.emergency_owner.is_none());

    let set_acc_nft = ACCOUNT_NFT.load(deps.as_ref().storage).unwrap();
    assert_eq!(old_account_nft, set_acc_nft.address().to_string());

    let set_red_bank = RED_BANK.load(deps.as_ref().storage).unwrap();
    assert_eq!(old_red_bank, set_red_bank.addr.as_str());
}
