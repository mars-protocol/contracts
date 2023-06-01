use cosmwasm_std::{attr, coin, from_binary, testing::mock_info, Addr, Decimal, Event, Uint128};
use mars_owner::{OwnerError::NotOwner, OwnerUpdate};
use mars_red_bank::{
    contract::{execute, instantiate, query},
    error::ContractError,
    interest_rates::{compute_scaled_amount, compute_underlying_amount, ScalingOperation},
    state::{COLLATERALS, MARKETS},
};
use mars_red_bank_types::{
    address_provider::MarsAddressType,
    error::MarsError,
    red_bank::{
        ConfigResponse, CreateOrUpdateConfig, ExecuteMsg, InitOrUpdateAssetParams, InstantiateMsg,
        InterestRateModel, Market, QueryMsg,
    },
};
use mars_testing::{mock_dependencies, mock_env, mock_env_at_block_time, MockEnvParams};
use mars_utils::error::ValidationError;

use crate::helpers::{th_get_expected_indices, th_init_market, th_setup};

mod helpers;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env(MockEnvParams::default());

    // Config with base params valid (just update the rest)
    let base_config = CreateOrUpdateConfig {
        address_provider: Some("address_provider".to_string()),
        close_factor: None,
    };

    // *
    // init config with empty params
    // *
    let empty_config = CreateOrUpdateConfig {
        address_provider: None,
        close_factor: None,
    };
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        config: empty_config,
    };
    let info = mock_info("owner", &[]);
    let error_res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert_eq!(error_res, MarsError::InstantiateParamsUnavailable {}.into());

    // *
    // init config with close_factor greater than 1
    // *
    let mut close_factor = Decimal::from_ratio(13u128, 10u128);
    let config = CreateOrUpdateConfig {
        close_factor: Some(close_factor),
        ..base_config.clone()
    };
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        config,
    };
    let info = mock_info("owner", &[]);
    let error_res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert_eq!(
        error_res,
        ValidationError::InvalidParam {
            param_name: "close_factor".to_string(),
            invalid_value: "1.3".to_string(),
            predicate: "<= 1".to_string(),
        }
        .into()
    );

    // *
    // init config with valid params
    // *
    close_factor = Decimal::from_ratio(1u128, 2u128);
    let config = CreateOrUpdateConfig {
        close_factor: Some(close_factor),
        ..base_config
    };
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        config,
    };

    // we can just call .unwrap() to assert this was a success
    let info = mock_info("owner", &[]);
    let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(value.owner.unwrap(), "owner");
    assert_eq!(value.address_provider, "address_provider");
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env(MockEnvParams::default());

    // *
    // init config with valid params
    // *
    let mut close_factor = Decimal::from_ratio(1u128, 4u128);
    let init_config = CreateOrUpdateConfig {
        address_provider: Some("address_provider".to_string()),
        close_factor: Some(close_factor),
    };
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        config: init_config.clone(),
    };
    // we can just call .unwrap() to assert this was a success
    let info = mock_info("owner", &[]);
    let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    // *
    // non owner is not authorized
    // *
    let msg = ExecuteMsg::UpdateConfig {
        config: init_config.clone(),
    };
    let info = mock_info("somebody", &[]);
    let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert_eq!(error_res, ContractError::Owner(NotOwner {}));

    // *
    // update config with close_factor
    // *
    close_factor = Decimal::from_ratio(13u128, 10u128);
    let config = CreateOrUpdateConfig {
        close_factor: Some(close_factor),
        ..init_config
    };
    let msg = ExecuteMsg::UpdateConfig {
        config,
    };
    let info = mock_info("owner", &[]);
    let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert_eq!(
        error_res,
        ValidationError::InvalidParam {
            param_name: "close_factor".to_string(),
            invalid_value: "1.3".to_string(),
            predicate: "<= 1".to_string(),
        }
        .into()
    );

    // *
    // update config with all new params
    // *
    close_factor = Decimal::from_ratio(1u128, 20u128);
    let config = CreateOrUpdateConfig {
        address_provider: Some("new_address_provider".to_string()),
        close_factor: Some(close_factor),
    };
    let msg = ExecuteMsg::UpdateConfig {
        config: config.clone(),
    };

    // we can just call .unwrap() to assert this was a success
    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // Read config from state
    let res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
    let new_config: ConfigResponse = from_binary(&res).unwrap();

    assert_eq!(new_config.owner.unwrap(), "owner".to_string());
    assert_eq!(new_config.address_provider, Addr::unchecked(config.address_provider.unwrap()));
    assert_eq!(new_config.close_factor, config.close_factor.unwrap());
}

#[test]
fn init_asset() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env(MockEnvParams::default());

    let config = CreateOrUpdateConfig {
        address_provider: Some("address_provider".to_string()),
        close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
    };
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        config,
    };
    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    let ir_model = InterestRateModel {
        optimal_utilization_rate: Decimal::one(),
        base: Decimal::percent(5),
        slope_1: Decimal::zero(),
        slope_2: Decimal::zero(),
    };

    let params = InitOrUpdateAssetParams {
        max_loan_to_value: Some(Decimal::from_ratio(8u128, 10u128)),
        reserve_factor: Some(Decimal::from_ratio(1u128, 100u128)),
        liquidation_threshold: Some(Decimal::one()),
        liquidation_bonus: Some(Decimal::zero()),
        interest_rate_model: Some(ir_model.clone()),
        deposit_enabled: Some(true),
        borrow_enabled: Some(true),
        deposit_cap: None,
    };

    // non owner is not authorized
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("somebody", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::Owner(NotOwner {}));
    }

    // init incorrect asset denom - error 1
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "!ksdfakefb*.s-".to_string(),
            params: params.clone(),
        };
        let info = mock_info("owner", &[]);
        let err = execute(deps.as_mut(), env.clone(), info, msg);
        assert_eq!(
            err,
            Err(ContractError::Validation(ValidationError::InvalidDenom {
                reason: "First character is not ASCII alphabetic".to_string()
            }))
        );
    }

    // init incorrect asset denom - error 2
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "ahdbufenf&*!-".to_string(),
            params: params.clone(),
        };
        let info = mock_info("owner", &[]);
        let err = execute(deps.as_mut(), env.clone(), info, msg);
        assert_eq!(
            err,
            Err(ContractError::Validation(ValidationError::InvalidDenom {
                reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                    .to_string()
            }))
        );
    }

    // init incorrect asset denom - error 3
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "ab".to_string(),
            params: params.clone(),
        };
        let info = mock_info("owner", &[]);
        let err = execute(deps.as_mut(), env.clone(), info, msg);
        assert_eq!(
            err,
            Err(ContractError::Validation(ValidationError::InvalidDenom {
                reason: "Invalid denom length".to_string()
            }))
        );
    }

    // init asset with empty params
    {
        let empty_asset_params = InitOrUpdateAssetParams {
            max_loan_to_value: None,
            liquidation_threshold: None,
            liquidation_bonus: None,
            ..params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: empty_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, MarsError::InstantiateParamsUnavailable {}.into());
    }

    // init asset with reserve_factor equal to 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            reserve_factor: Some(Decimal::one()),
            ..params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ValidationError::InvalidParam {
                param_name: "reserve_factor".to_string(),
                invalid_value: "1".to_string(),
                predicate: "< 1".to_string(),
            }
            .into()
        );
    }

    // init asset with max_loan_to_value greater than 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(11u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ValidationError::InvalidParam {
                param_name: "max_loan_to_value".to_string(),
                invalid_value: "1.1".to_string(),
                predicate: "<= 1".to_string(),
            }
            .into()
        );
    }

    // init asset with liquidation_threshold greater than 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            liquidation_threshold: Some(Decimal::from_ratio(11u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ValidationError::InvalidParam {
                param_name: "liquidation_threshold".to_string(),
                invalid_value: "1.1".to_string(),
                predicate: "<= 1".to_string(),
            }
            .into()
        );
    }

    // init asset with liquidation_bonus greater than 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            liquidation_bonus: Some(Decimal::from_ratio(11u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ValidationError::InvalidParam {
                param_name: "liquidation_bonus".to_string(),
                invalid_value: "1.1".to_string(),
                predicate: "<= 1".to_string(),
            }
            .into()
        );
    }

    // init asset where LTV >= liquidity threshold
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(5u128, 10u128)),
            liquidation_threshold: Some(Decimal::from_ratio(5u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ValidationError::InvalidParam {
                param_name: "liquidation_threshold".to_string(),
                invalid_value: "0.5".to_string(),
                predicate: "> 0.5 (max LTV)".to_string()
            }
            .into()
        );
    }

    // init asset where optimal utilization rate > 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            interest_rate_model: Some(InterestRateModel {
                optimal_utilization_rate: Decimal::percent(110),
                ..ir_model
            }),
            ..params
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ValidationError::InvalidParam {
                param_name: "optimal_utilization_rate".to_string(),
                invalid_value: "1.1".to_string(),
                predicate: "<= 1".to_string()
            }
            .into()
        );
    }

    // owner is authorized
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("owner", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // should have asset market with Canonical default address
        let market = MARKETS.load(&deps.storage, "someasset").unwrap();
        assert_eq!(market.denom, "someasset");

        // should have unlimited deposit cap
        assert_eq!(market.deposit_cap, Uint128::MAX);

        assert_eq!(res.attributes, vec![attr("action", "init_asset"), attr("denom", "someasset")]);
    }

    // can't init more than once
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::AssetAlreadyInitialized {});
    }
}

#[test]
fn update_asset() {
    let mut deps = mock_dependencies(&[]);
    let start_time = 100000000;
    let env = mock_env_at_block_time(start_time);

    let config = CreateOrUpdateConfig {
        address_provider: Some("address_provider".to_string()),
        close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
    };
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        config,
    };
    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    let ir_model = InterestRateModel {
        optimal_utilization_rate: Decimal::one(),
        base: Decimal::percent(5),
        slope_1: Decimal::zero(),
        slope_2: Decimal::zero(),
    };

    let params = InitOrUpdateAssetParams {
        max_loan_to_value: Some(Decimal::from_ratio(50u128, 100u128)),
        reserve_factor: Some(Decimal::from_ratio(1u128, 100u128)),
        liquidation_threshold: Some(Decimal::from_ratio(80u128, 100u128)),
        liquidation_bonus: Some(Decimal::from_ratio(10u128, 100u128)),
        interest_rate_model: Some(ir_model.clone()),
        deposit_enabled: Some(true),
        borrow_enabled: Some(true),
        deposit_cap: None,
    };

    // non owner is not authorized
    {
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("somebody", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::Owner(NotOwner {}));
    }

    // owner is authorized but can't update asset if not initialized first
    {
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::AssetNotInitialized {});
    }

    // initialize asset
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("owner", &[]);
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    }

    // update asset with max_loan_to_value greater than 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(11u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ValidationError::InvalidParam {
                param_name: "max_loan_to_value".to_string(),
                invalid_value: "1.1".to_string(),
                predicate: "<= 1".to_string(),
            }
            .into()
        );
    }

    // update asset with liquidation_threshold greater than 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            liquidation_threshold: Some(Decimal::from_ratio(11u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ValidationError::InvalidParam {
                param_name: "liquidation_threshold".to_string(),
                invalid_value: "1.1".to_string(),
                predicate: "<= 1".to_string(),
            }
            .into()
        );
    }

    // update asset with liquidation_bonus greater than 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            liquidation_bonus: Some(Decimal::from_ratio(11u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ValidationError::InvalidParam {
                param_name: "liquidation_bonus".to_string(),
                invalid_value: "1.1".to_string(),
                predicate: "<= 1".to_string(),
            }
            .into()
        );
    }

    // update asset where LTV >= liquidity threshold
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(6u128, 10u128)),
            liquidation_threshold: Some(Decimal::from_ratio(5u128, 10u128)),
            ..params
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ValidationError::InvalidParam {
                param_name: "liquidation_threshold".to_string(),
                invalid_value: "0.5".to_string(),
                predicate: "> 0.6 (max LTV)".to_string()
            }
            .into()
        );
    }

    // update asset where optimal utilization rate > 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            interest_rate_model: Some(InterestRateModel {
                optimal_utilization_rate: Decimal::percent(110),
                ..ir_model
            }),
            ..params
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ValidationError::InvalidParam {
                param_name: "optimal_utilization_rate".to_string(),
                invalid_value: "1.1".to_string(),
                predicate: "<= 1".to_string()
            }
            .into()
        );
    }

    // update asset with new params
    {
        let params = InitOrUpdateAssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(60u128, 100u128)),
            reserve_factor: Some(Decimal::from_ratio(10u128, 100u128)),
            liquidation_threshold: Some(Decimal::from_ratio(90u128, 100u128)),
            liquidation_bonus: Some(Decimal::from_ratio(12u128, 100u128)),
            interest_rate_model: Some(ir_model),
            deposit_enabled: Some(true),
            borrow_enabled: Some(true),
            deposit_cap: Some(Uint128::new(10_000_000)),
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("owner", &[]);

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(res.messages, vec![],);
        assert_eq!(
            res.attributes,
            vec![attr("action", "update_asset"), attr("denom", "someasset")],
        );

        let new_market = MARKETS.load(&deps.storage, "someasset").unwrap();
        assert_eq!(params.max_loan_to_value.unwrap(), new_market.max_loan_to_value);
        assert_eq!(params.reserve_factor.unwrap(), new_market.reserve_factor);
        assert_eq!(params.liquidation_threshold.unwrap(), new_market.liquidation_threshold);
        assert_eq!(params.liquidation_bonus.unwrap(), new_market.liquidation_bonus);
        assert_eq!(params.interest_rate_model.unwrap(), new_market.interest_rate_model);
    }

    // update asset with empty params
    {
        let market_before = MARKETS.load(&deps.storage, "someasset").unwrap();

        let empty_asset_params = InitOrUpdateAssetParams {
            max_loan_to_value: None,
            reserve_factor: None,
            liquidation_threshold: None,
            liquidation_bonus: None,
            interest_rate_model: None,
            deposit_enabled: None,
            borrow_enabled: None,
            deposit_cap: None,
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: empty_asset_params,
        };
        let info = mock_info("owner", &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        // no interest updated event
        assert_eq!(res.events.len(), 0);

        let new_market = MARKETS.load(&deps.storage, "someasset").unwrap();
        // should keep old params
        assert_eq!(market_before.borrow_rate, new_market.borrow_rate);
        assert_eq!(market_before.max_loan_to_value, new_market.max_loan_to_value);
        assert_eq!(market_before.reserve_factor, new_market.reserve_factor);
        assert_eq!(market_before.liquidation_threshold, new_market.liquidation_threshold);
        assert_eq!(market_before.liquidation_bonus, new_market.liquidation_bonus);
        assert_eq!(market_before.deposit_cap, new_market.deposit_cap);
        assert_eq!(market_before.interest_rate_model, new_market.interest_rate_model);
    }
}

#[test]
fn update_asset_with_new_interest_rate_model_params() {
    let mut deps = mock_dependencies(&[]);

    let config = CreateOrUpdateConfig {
        address_provider: Some("address_provider".to_string()),
        close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
    };
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        config,
    };
    let info = mock_info("owner", &[]);
    let env = mock_env(MockEnvParams::default());
    instantiate(deps.as_mut(), env, info, msg).unwrap();

    let ir_model = InterestRateModel {
        optimal_utilization_rate: Decimal::one(),
        base: Decimal::percent(5),
        slope_1: Decimal::zero(),
        slope_2: Decimal::zero(),
    };

    let params = InitOrUpdateAssetParams {
        max_loan_to_value: Some(Decimal::from_ratio(50u128, 100u128)),
        reserve_factor: Some(Decimal::from_ratio(2u128, 100u128)),
        liquidation_threshold: Some(Decimal::from_ratio(80u128, 100u128)),
        liquidation_bonus: Some(Decimal::from_ratio(10u128, 100u128)),
        interest_rate_model: Some(ir_model.clone()),
        deposit_enabled: Some(true),
        borrow_enabled: Some(true),
        deposit_cap: None,
    };

    let msg = ExecuteMsg::InitAsset {
        denom: "someasset".to_string(),
        params: params.clone(),
    };
    let info = mock_info("owner", &[]);
    let env = mock_env_at_block_time(1_000_000);
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();

    // Verify if IR model is saved correctly
    let market_before = MARKETS.load(&deps.storage, "someasset").unwrap();
    assert_eq!(market_before.interest_rate_model, ir_model);

    // new IR model has a fixed borrow rate of 69%
    let new_ir_model = InterestRateModel {
        base: Decimal::percent(69),
        ..ir_model
    };
    let asset_params_with_new_ir_model = InitOrUpdateAssetParams {
        interest_rate_model: Some(new_ir_model.clone()),
        ..params
    };
    let msg = ExecuteMsg::UpdateAsset {
        denom: "someasset".to_string(),
        params: asset_params_with_new_ir_model,
    };
    let info = mock_info("owner", &[]);
    let env = mock_env_at_block_time(2_000_000);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // Verify if IR model is updated
    let new_market = MARKETS.load(&deps.storage, "someasset").unwrap();
    assert_eq!(new_market.interest_rate_model, new_ir_model);

    // Indices should have been updated using previous interest rate
    let expected_indices = th_get_expected_indices(&market_before, 2_000_000);
    assert_eq!(new_market.liquidity_index, expected_indices.liquidity);
    assert_eq!(new_market.borrow_index, expected_indices.borrow);
    assert_eq!(new_market.indexes_last_updated, 2_000_000);

    // Interest rate should have been recomputed using new strategy and values
    let expected_borrow_rate = new_ir_model.get_borrow_rate(Decimal::zero()).unwrap();
    let expected_liquidity_rate = new_ir_model
        .get_liquidity_rate(expected_borrow_rate, Decimal::zero(), Decimal::percent(2))
        .unwrap();
    assert_eq!(new_market.borrow_rate, expected_borrow_rate);
    assert_eq!(new_market.liquidity_rate, expected_liquidity_rate);

    // proper event is logged
    assert_eq!(
        res.events,
        vec![Event::new("interests_updated")
            .add_attribute("denom", "someasset")
            .add_attribute("borrow_index", new_market.borrow_index.to_string())
            .add_attribute("liquidity_index", new_market.liquidity_index.to_string())
            .add_attribute("borrow_rate", expected_borrow_rate.to_string())
            .add_attribute("liquidity_rate", expected_liquidity_rate.to_string())]
    );

    // mint message is not sent as debt is 0
    assert_eq!(res.messages, vec![])
}

#[test]
fn update_asset_new_reserve_factor_accrues_interest_rate() {
    let asset_liquidity = Uint128::from(10_000_000_000_000_u128);
    let mut deps = th_setup(&[coin(asset_liquidity.into(), "somecoin")]);

    let reserve_factor = Decimal::from_ratio(1_u128, 10_u128);

    let ir_model = InterestRateModel {
        optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
        base: Decimal::zero(),
        slope_1: Decimal::from_ratio(1_u128, 2_u128),
        slope_2: Decimal::from_ratio(2_u128, 1_u128),
    };

    let asset_initial_debt = Uint128::new(2_000_000_000_000);
    let debt_total_scaled =
        compute_scaled_amount(asset_initial_debt, Decimal::one(), ScalingOperation::Ceil).unwrap();

    let asset_initial_collateral = asset_liquidity + asset_initial_debt;
    let collateral_total_scaled =
        compute_scaled_amount(asset_initial_collateral, Decimal::one(), ScalingOperation::Ceil)
            .unwrap();

    let initial_utilization_rate = Decimal::from_ratio(debt_total_scaled, collateral_total_scaled);
    let borrow_rate = ir_model.get_borrow_rate(initial_utilization_rate).unwrap();
    let liquidity_rate =
        ir_model.get_liquidity_rate(borrow_rate, initial_utilization_rate, reserve_factor).unwrap();

    let market_before = th_init_market(
        deps.as_mut(),
        "somecoin",
        &Market {
            reserve_factor,
            borrow_index: Decimal::one(),
            liquidity_index: Decimal::one(),
            indexes_last_updated: 1_000_000,
            borrow_rate,
            liquidity_rate,
            collateral_total_scaled,
            debt_total_scaled,
            interest_rate_model: ir_model.clone(),
            ..Default::default()
        },
    );

    let params = InitOrUpdateAssetParams {
        max_loan_to_value: None,
        reserve_factor: Some(Decimal::from_ratio(2_u128, 10_u128)),
        liquidation_threshold: None,
        liquidation_bonus: None,
        interest_rate_model: None,
        deposit_enabled: None,
        borrow_enabled: None,
        deposit_cap: None,
    };
    let msg = ExecuteMsg::UpdateAsset {
        denom: "somecoin".to_string(),
        params,
    };
    let info = mock_info("owner", &[]);
    let env = mock_env_at_block_time(1_500_000);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    let new_market = MARKETS.load(&deps.storage, "somecoin").unwrap();

    // Indices should have been updated using previous interest rate
    let expected_indices = th_get_expected_indices(&market_before, 1_500_000);
    assert_eq!(new_market.liquidity_index, expected_indices.liquidity);
    assert_eq!(new_market.borrow_index, expected_indices.borrow);
    assert_eq!(new_market.indexes_last_updated, 1_500_000);

    // Interest rate should have been recomputed using new strategy and values
    let expected_debt = compute_underlying_amount(
        new_market.debt_total_scaled,
        new_market.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();
    let expected_liquidity = asset_liquidity;
    // in this particular example, we have to subtract 1 from the total underlying
    // collateral amount here, because of rounding error.
    let expected_collateral = expected_liquidity + expected_debt - Uint128::new(1);
    let expected_utilization_rate = Decimal::from_ratio(expected_debt, expected_collateral);

    let expected_borrow_rate = ir_model.get_borrow_rate(expected_utilization_rate).unwrap();

    let expected_liquidity_rate = ir_model
        .get_liquidity_rate(
            expected_borrow_rate,
            expected_utilization_rate,
            new_market.reserve_factor,
        )
        .unwrap();

    assert_eq!(new_market.borrow_rate, expected_borrow_rate);
    assert_eq!(new_market.liquidity_rate, expected_liquidity_rate);

    // proper event is logged
    assert_eq!(
        res.events,
        vec![Event::new("interests_updated")
            .add_attribute("denom", "somecoin")
            .add_attribute("borrow_index", new_market.borrow_index.to_string())
            .add_attribute("liquidity_index", new_market.liquidity_index.to_string())
            .add_attribute("borrow_rate", expected_borrow_rate.to_string())
            .add_attribute("liquidity_rate", expected_liquidity_rate.to_string())]
    );

    let current_debt_total = compute_underlying_amount(
        new_market.debt_total_scaled,
        new_market.borrow_index,
        ScalingOperation::Ceil,
    )
    .unwrap();
    let interest_accrued = current_debt_total - asset_initial_debt;
    let expected_rewards = interest_accrued * market_before.reserve_factor;
    let expected_rewards_scaled = compute_scaled_amount(
        expected_rewards,
        new_market.liquidity_index,
        ScalingOperation::Truncate,
    )
    .unwrap();

    // the rewards collector previously did not have a collateral possition
    // now it should have one with the expected rewards scaled amount
    let collateral = COLLATERALS
        .load(
            deps.as_ref().storage,
            (&Addr::unchecked(MarsAddressType::RewardsCollector.to_string()), "somecoin"),
        )
        .unwrap();
    assert_eq!(collateral.amount_scaled, expected_rewards_scaled);
}

#[test]
fn update_asset_by_emergency_owner() {
    let mut deps = mock_dependencies(&[]);
    let start_time = 100000000;
    let env = mock_env_at_block_time(start_time);

    let config = CreateOrUpdateConfig {
        address_provider: Some("address_provider".to_string()),
        close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
    };
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        config,
    };
    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    let ir_model = InterestRateModel {
        optimal_utilization_rate: Decimal::one(),
        base: Decimal::percent(5),
        slope_1: Decimal::zero(),
        slope_2: Decimal::zero(),
    };

    let params = InitOrUpdateAssetParams {
        max_loan_to_value: Some(Decimal::from_ratio(50u128, 100u128)),
        reserve_factor: Some(Decimal::from_ratio(1u128, 100u128)),
        liquidation_threshold: Some(Decimal::from_ratio(80u128, 100u128)),
        liquidation_bonus: Some(Decimal::from_ratio(10u128, 100u128)),
        interest_rate_model: Some(ir_model.clone()),
        deposit_enabled: Some(true),
        borrow_enabled: Some(true),
        deposit_cap: None,
    };

    execute(
        deps.as_mut(),
        env.clone(),
        mock_info("owner", &[]),
        ExecuteMsg::UpdateOwner(OwnerUpdate::SetEmergencyOwner {
            emergency_owner: "emergency_owner".to_string(),
        }),
    )
    .unwrap();

    // emergency owner is authorized but can't update asset if not initialized first
    {
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("emergency_owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::AssetNotInitialized {});
    }

    // initialize asset
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("owner", &[]);
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    }

    // update asset with borrow_enabled = true, should have not effect on the saved market
    {
        let old_market = MARKETS.load(&deps.storage, "someasset").unwrap();

        let new_asset_params = InitOrUpdateAssetParams {
            borrow_enabled: Some(true),
            ..params
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: new_asset_params,
        };
        let info = mock_info("emergency_owner", &[]);
        let res_err = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(res_err, ContractError::Mars(MarsError::Unauthorized {}));

        let new_market = MARKETS.load(&deps.storage, "someasset").unwrap();
        assert_eq!(old_market, new_market)
    }

    // update asset with new params, only borrow_enabled = false should have effect on the saved market
    {
        let mut old_market = MARKETS.load(&deps.storage, "someasset").unwrap();

        let params = InitOrUpdateAssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(60u128, 100u128)),
            reserve_factor: Some(Decimal::from_ratio(10u128, 100u128)),
            liquidation_threshold: Some(Decimal::from_ratio(90u128, 100u128)),
            liquidation_bonus: Some(Decimal::from_ratio(12u128, 100u128)),
            interest_rate_model: Some(ir_model),
            deposit_enabled: Some(false),
            borrow_enabled: Some(false),
            deposit_cap: Some(Uint128::new(10_000_000)),
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params,
        };
        let info = mock_info("emergency_owner", &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert!(res.messages.is_empty());
        assert_eq!(
            res.attributes,
            vec![attr("action", "emergency_update_asset"), attr("denom", "someasset")],
        );

        let new_market = MARKETS.load(&deps.storage, "someasset").unwrap();
        // old market should have only borrow_enabled updated
        old_market.borrow_enabled = false;
        assert_eq!(old_market, new_market);
    }
}
