use cosmwasm_std::{attr, Decimal, Uint128};
use mars_owner::OwnerError::NotOwner;
use mars_red_bank_types::error::MarsError;
use mars_red_bank_types::red_bank::InterestRateModel;
use mars_testing::{mock_dependencies, mock_info};
use mars_utils::error::ValidationError;
use mars_params::contract::{execute, instantiate};
use mars_params::error::ContractError;
use mars_params::msg::{ExecuteMsg, InstantiateMsg};
use mars_params::types::AssetParams;

#[test]
fn init_asset() {
    let mut deps = mock_dependencies(&[]);
    let mut close_factor = Decimal::from_ratio(1u128, 4u128);
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        emergency_owner: "emergency_owner".to_string(),
        close_factor
    };
    let info = mock_info("owner");
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    let ir_model = InterestRateModel {
        optimal_utilization_rate: Decimal::one(),
        base: Decimal::percent(5),
        slope_1: Decimal::zero(),
        slope_2: Decimal::zero(),
    };

    let params = AssetParams {
        max_loan_to_value: Some(Decimal::from_ratio(8u128, 10u128)),
        reserve_factor: Some(Decimal::from_ratio(1u128, 100u128)),
        liquidation_threshold: Some(Decimal::one()),
        liquidation_bonus: Some(Decimal::zero()),
        rover_whitelisted: false,
        red_bank_deposit_enabled: false,
        red_bank_borrow_enabled: false,
        red_bank_deposit_cap: Default::default(),
        interest_rate_model: Some(ir_model.clone()),
        uncollateralized_loan_limit: Default::default(),
    };

    // non owner is not authorized
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("somebody");
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::Owner(NotOwner {}));
    }

    // init incorrect asset denom - error 2
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "ahdbufenf&*!-".to_string(),
            params: params.clone(),
        };
        let info = mock_info("owner");
        let err = execute(deps.as_mut(), info, msg);
        assert_eq!(
            err,
            Err(ContractError::Validation(ValidationError::InvalidDenom {
                reason: "Not all characters are ASCII alphanumeric or one of:  /  :  .  _  -"
                    .to_string()
            }))
        );
    }
    // init asset with empty params
    // NEED OPTIONAL STRUCT
    {
        let empty_asset_params = AssetParams {
            max_loan_to_value: Default::default(),
            liquidation_threshold: Default::default(),
            liquidation_bonus: Default::default(),
            rover_whitelisted: false,
            red_bank_deposit_enabled: false,
            red_bank_borrow_enabled: false,
            red_bank_deposit_cap: Default::default(),
            interest_rate_model: Default::default(),
            reserve_factor: Default::default(),
            uncollateralized_loan_limit: Default::default(),
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: empty_asset_params,
        };
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
        assert_eq!(error_res, MarsError::InstantiateParamsUnavailable {}.into());
    }

    // init asset with max_loan_to_value greater than 1
    {
        let invalid_asset_params = AssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(11u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
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
        let invalid_asset_params = AssetParams {
            liquidation_threshold: Some(Decimal::from_ratio(11u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
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
        let invalid_asset_params = AssetParams {
            liquidation_bonus: Some(Decimal::from_ratio(11u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
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
        let invalid_asset_params = AssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(5u128, 10u128)),
            liquidation_threshold: Some(Decimal::from_ratio(5u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
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
        let invalid_asset_params = AssetParams {
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
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
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
        let info = mock_info("owner");
        let res = execute(deps.as_mut(), info, msg).unwrap();

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
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::AssetAlreadyInitialized {});
    }
}

#[test]
fn update_asset() {
    let mut deps = mock_dependencies(&[]);
    let start_time = 100000000;

    let mut close_factor = Decimal::from_ratio(1u128, 4u128);
    let msg = InstantiateMsg {
        owner: "owner".to_string(),
        emergency_owner: "emergency_owner".to_string(),
        close_factor
    };
    let info = mock_info("owner");
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    let ir_model = InterestRateModel {
        optimal_utilization_rate: Decimal::one(),
        base: Decimal::percent(5),
        slope_1: Decimal::zero(),
        slope_2: Decimal::zero(),
    };

    let params = AssetParams {
        max_loan_to_value: Some(Decimal::from_ratio(50u128, 100u128)),
        reserve_factor: Some(Decimal::from_ratio(1u128, 100u128)),
        liquidation_threshold: Some(Decimal::from_ratio(80u128, 100u128)),
        liquidation_bonus: Some(Decimal::from_ratio(10u128, 100u128)),
        rover_whitelisted: false,
        red_bank_deposit_enabled: false,
        red_bank_borrow_enabled: false,
        red_bank_deposit_cap: Default::default(),
        interest_rate_model: Some(ir_model.clone()),
        uncollateralized_loan_limit: Default::default(),
    };

    // non owner is not authorized
    {
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("somebody");
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::Owner(NotOwner {}));
    }

    // owner is authorized but can't update asset if not initialized first
    {
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::AssetNotInitialized {});
    }

    // initialize asset
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("owner");
        let _res = execute(deps.as_mut(), info, msg).unwrap();
    }

    // update asset with max_loan_to_value greater than 1
    {
        let invalid_asset_params = AssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(11u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
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
        let invalid_asset_params = AssetParams {
            liquidation_threshold: Some(Decimal::from_ratio(11u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
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
        let invalid_asset_params = AssetParams {
            liquidation_bonus: Some(Decimal::from_ratio(11u128, 10u128)),
            ..params.clone()
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
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
        let invalid_asset_params = AssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(6u128, 10u128)),
            liquidation_threshold: Some(Decimal::from_ratio(5u128, 10u128)),
            ..params
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: invalid_asset_params,
        };
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), info, msg).unwrap_err();
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
        let invalid_asset_params = AssetParams {
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
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(),info, msg).unwrap_err();
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
        let params = AssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(60u128, 100u128)),
            reserve_factor: Some(Decimal::from_ratio(10u128, 100u128)),
            liquidation_threshold: Some(Decimal::from_ratio(90u128, 100u128)),
            liquidation_bonus: Some(Decimal::from_ratio(12u128, 100u128)),
            rover_whitelisted: false,
            red_bank_deposit_enabled: false,
            red_bank_borrow_enabled: false,
            red_bank_deposit_cap: Default::default(),
            interest_rate_model: Some(ir_model),
            uncollateralized_loan_limit: Default::default(),
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            params: params.clone(),
        };
        let info = mock_info("owner");

        let res = execute(deps.as_mut(), info, msg).unwrap();
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

        let empty_asset_params = AssetParams {
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
        let info = mock_info("owner");
        let res = execute(deps.as_mut(), info, msg).unwrap();

        // no interest updated event
        assert_eq!(res.events.len(), 0);
    }
}
