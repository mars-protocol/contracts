use cosmwasm_std::testing::{mock_info, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    attr, coin, from_binary, to_binary, Addr, CosmosMsg, Decimal, Event, StdError, SubMsg, Uint128,
    WasmMsg,
};
use cw20::MinterResponse;
use cw20_base::msg::InstantiateMarketingInfo;

use mars_outpost::error::MarsError;
use mars_outpost::helpers::zero_address;
use mars_outpost::ma_token;
use mars_outpost::red_bank::{
    get_liquidity_rate, linear_get_borrow_rate, ConfigResponse, CreateOrUpdateConfig,
    DynamicInterestRateModelParams, DynamicInterestRateModelState, ExecuteMsg,
    InitOrUpdateAssetParams, InstantiateMsg, InterestRateModel, InterestRateModelError,
    InterestRateModelParams, LinearInterestRateModelParams, Market, MarketError, QueryMsg,
};
use mars_testing::{mock_dependencies, mock_env, mock_env_at_block_time, MockEnvParams};

use crate::contract::{execute, instantiate, query};
use crate::error::ContractError;
use crate::interest_rates::{compute_scaled_amount, compute_underlying_amount, ScalingOperation};
use crate::state::{CONFIG, GLOBAL_STATE, MARKETS, MARKET_DENOMS_BY_INDEX};

use super::helpers::{th_get_expected_indices, th_init_market, th_setup};

#[test]
fn test_proper_initialization() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env(MockEnvParams::default());

    // Config with base params valid (just update the rest)
    let base_config = CreateOrUpdateConfig {
        owner: Some("owner".to_string()),
        address_provider_address: Some("address_provider".to_string()),
        ma_token_code_id: Some(10u64),
        close_factor: None,
    };

    // *
    // init config with empty params
    // *
    let empty_config = CreateOrUpdateConfig {
        owner: None,
        address_provider_address: None,
        ma_token_code_id: None,
        close_factor: None,
    };
    let msg = InstantiateMsg {
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
        config,
    };
    let info = mock_info("owner", &[]);
    let error_res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert_eq!(
        error_res,
        MarsError::InvalidParam {
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
        config,
    };

    // we can just call .unwrap() to assert this was a success
    let info = mock_info("owner", &[]);
    let res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(10, value.ma_token_code_id);
    assert_eq!(0, value.market_count);
}

#[test]
fn test_update_config() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env(MockEnvParams::default());

    // *
    // init config with valid params
    // *
    let mut close_factor = Decimal::from_ratio(1u128, 4u128);
    let init_config = CreateOrUpdateConfig {
        owner: Some("owner".to_string()),
        address_provider_address: Some("address_provider".to_string()),
        ma_token_code_id: Some(20u64),
        close_factor: Some(close_factor),
    };
    let msg = InstantiateMsg {
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
    assert_eq!(error_res, MarsError::Unauthorized {}.into());

    // *
    // update config with close_factor
    // *
    close_factor = Decimal::from_ratio(13u128, 10u128);
    let config = CreateOrUpdateConfig {
        owner: None,
        close_factor: Some(close_factor),
        ..init_config.clone()
    };
    let msg = ExecuteMsg::UpdateConfig {
        config,
    };
    let info = mock_info("owner", &[]);
    let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
    assert_eq!(
        error_res,
        MarsError::InvalidParam {
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
        owner: Some("new_owner".to_string()),
        address_provider_address: Some("new_address_provider".to_string()),
        ma_token_code_id: Some(40u64),
        close_factor: Some(close_factor),
    };
    let msg = ExecuteMsg::UpdateConfig {
        config: config.clone(),
    };

    // we can just call .unwrap() to assert this was a success
    let info = mock_info("owner", &[]);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // Read config from state
    let new_config = CONFIG.load(&deps.storage).unwrap();

    assert_eq!(new_config.owner, Addr::unchecked("new_owner"));
    assert_eq!(
        new_config.address_provider_address,
        Addr::unchecked(config.address_provider_address.unwrap())
    );
    assert_eq!(new_config.ma_token_code_id, config.ma_token_code_id.unwrap());
    assert_eq!(new_config.close_factor, config.close_factor.unwrap());
}

#[test]
fn test_init_asset() {
    let mut deps = mock_dependencies(&[]);
    let env = mock_env(MockEnvParams::default());

    let config = CreateOrUpdateConfig {
        owner: Some("owner".to_string()),
        address_provider_address: Some("address_provider".to_string()),
        ma_token_code_id: Some(5u64),
        close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
    };
    let msg = InstantiateMsg {
        config,
    };
    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    let dynamic_ir_params = DynamicInterestRateModelParams {
        min_borrow_rate: Decimal::from_ratio(5u128, 100u128),
        max_borrow_rate: Decimal::from_ratio(50u128, 100u128),
        kp_1: Decimal::from_ratio(3u128, 1u128),
        optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
        kp_augmentation_threshold: Decimal::from_ratio(2000u128, 1u128),
        kp_2: Decimal::from_ratio(2u128, 1u128),
        update_threshold_seconds: 1,
        update_threshold_txs: 1,
    };
    let asset_params = InitOrUpdateAssetParams {
        initial_borrow_rate: Some(Decimal::from_ratio(20u128, 100u128)),
        max_loan_to_value: Some(Decimal::from_ratio(8u128, 10u128)),
        reserve_factor: Some(Decimal::from_ratio(1u128, 100u128)),
        liquidation_threshold: Some(Decimal::one()),
        liquidation_bonus: Some(Decimal::zero()),
        interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
            dynamic_ir_params.clone(),
        )),
        active: Some(true),
        deposit_enabled: Some(true),
        borrow_enabled: Some(true),
    };

    // non owner is not authorized
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            asset_params: asset_params.clone(),
            asset_symbol: None,
        };
        let info = mock_info("somebody", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, MarsError::Unauthorized {}.into());
    }

    // init asset with empty params
    {
        let empty_asset_params = InitOrUpdateAssetParams {
            max_loan_to_value: None,
            liquidation_threshold: None,
            liquidation_bonus: None,
            ..asset_params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),

            asset_params: empty_asset_params,
            asset_symbol: None,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, MarsError::InstantiateParamsUnavailable {}.into());
    }

    // init asset with max_loan_to_value greater than 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(11u128, 10u128)),
            ..asset_params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            asset_params: invalid_asset_params,
            asset_symbol: None,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::Market(
                MarsError::InvalidParam {
                    param_name: "max_loan_to_value".to_string(),
                    invalid_value: "1.1".to_string(),
                    predicate: "<= 1".to_string(),
                }
                .into()
            )
        );
    }

    // init asset with liquidation_threshold greater than 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            liquidation_threshold: Some(Decimal::from_ratio(11u128, 10u128)),
            ..asset_params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            asset_params: invalid_asset_params,
            asset_symbol: None,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::Market(
                MarsError::InvalidParam {
                    param_name: "liquidation_threshold".to_string(),
                    invalid_value: "1.1".to_string(),
                    predicate: "<= 1".to_string(),
                }
                .into()
            )
        );
    }

    // init asset with liquidation_bonus greater than 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            liquidation_bonus: Some(Decimal::from_ratio(11u128, 10u128)),
            ..asset_params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            asset_params: invalid_asset_params,
            asset_symbol: None,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::Market(
                MarsError::InvalidParam {
                    param_name: "liquidation_bonus".to_string(),
                    invalid_value: "1.1".to_string(),
                    predicate: "<= 1".to_string(),
                }
                .into()
            )
        );
    }

    // init asset where LTV >= liquidity threshold
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(5u128, 10u128)),
            liquidation_threshold: Some(Decimal::from_ratio(5u128, 10u128)),
            ..asset_params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            asset_params: invalid_asset_params,
            asset_symbol: None,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::Market(MarketError::InvalidLiquidationThreshold {
                liquidation_threshold: Decimal::from_ratio(1u128, 2u128),
                max_loan_to_value: Decimal::from_ratio(1u128, 2u128)
            })
        );
    }

    // init asset where min borrow rate >= max borrow rate
    {
        let invalid_dynamic_ir_params = DynamicInterestRateModelParams {
            min_borrow_rate: Decimal::from_ratio(5u128, 10u128),
            max_borrow_rate: Decimal::from_ratio(4u128, 10u128),
            ..dynamic_ir_params.clone()
        };
        let invalid_asset_params = InitOrUpdateAssetParams {
            interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                invalid_dynamic_ir_params,
            )),
            ..asset_params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            asset_params: invalid_asset_params,
            asset_symbol: None,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::InterestRateModel(InterestRateModelError::InvalidMinMaxBorrowRate {
                min_borrow_rate: Decimal::from_ratio(5u128, 10u128),
                max_borrow_rate: Decimal::from_ratio(4u128, 10u128)
            })
        );
    }

    // init asset where optimal utilization rate > 1
    {
        let invalid_dynamic_ir_params = DynamicInterestRateModelParams {
            optimal_utilization_rate: Decimal::from_ratio(11u128, 10u128),
            ..dynamic_ir_params.clone()
        };
        let invalid_asset_params = InitOrUpdateAssetParams {
            interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                invalid_dynamic_ir_params,
            )),
            ..asset_params.clone()
        };
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            asset_params: invalid_asset_params,
            asset_symbol: None,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::InterestRateModel(
                InterestRateModelError::InvalidOptimalUtilizationRate {}
            )
        );
    }

    // owner is authorized
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            asset_params: asset_params.clone(),
            asset_symbol: None,
        };
        let info = mock_info("owner", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // should have asset market with Canonical default address
        let market = MARKETS.load(&deps.storage, "someasset").unwrap();
        assert_eq!(zero_address(), market.ma_token_address);
        // should have 0 index
        assert_eq!(0, market.index);

        // should store reference in market index
        let denom = MARKET_DENOMS_BY_INDEX.load(&deps.storage, 0).unwrap();
        assert_eq!("someasset", &denom);

        // Should have market count of 1
        let money_market = GLOBAL_STATE.load(&deps.storage).unwrap();
        assert_eq!(money_market.market_count, 1);

        // should instantiate a liquidity token
        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some("protocol_admin".to_string()),
                code_id: 5u64,
                msg: to_binary(&ma_token::msg::InstantiateMsg {
                    name: String::from("Mars someasset Liquidity Token"),
                    symbol: String::from("masomeasset"),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: MOCK_CONTRACT_ADDR.to_string(),
                        cap: None,
                    }),
                    init_hook: Some(ma_token::msg::InitHook {
                        contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                        msg: to_binary(&ExecuteMsg::InitAssetTokenCallback {
                            denom,
                        })
                        .unwrap()
                    }),
                    marketing: Some(InstantiateMarketingInfo {
                        project: Some("Mars Protocol".to_string()),
                        description: Some(
                            "Interest earning token representing deposits for someasset"
                                .to_string()
                        ),

                        marketing: Some("protocol_admin".to_string()),
                        logo: None,
                    }),
                    red_bank_address: MOCK_CONTRACT_ADDR.to_string(),
                    incentives_address: "incentives".to_string(),
                })
                .unwrap(),
                funds: vec![],
                label: "masomeasset".to_string()
            })),]
        );

        assert_eq!(res.attributes, vec![attr("action", "init_asset"), attr("denom", "someasset")]);
    }

    // can't init more than once
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            asset_params: asset_params.clone(),
            asset_symbol: None,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::AssetAlreadyInitialized {});
    }

    // callback comes back with created token
    {
        let msg = ExecuteMsg::InitAssetTokenCallback {
            denom: "someasset".to_string(),
        };
        let info = mock_info("mtokencontract", &[]);
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // should have asset market with contract address
        let market = MARKETS.load(&deps.storage, "someasset").unwrap();
        assert_eq!(Addr::unchecked("mtokencontract"), market.ma_token_address);
        assert_eq!(Decimal::one(), market.liquidity_index);
    }

    // calling this again should not be allowed
    {
        let msg = ExecuteMsg::InitAssetTokenCallback {
            denom: "someasset".to_string(),
        };
        let info = mock_info("mtokencontract", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, MarsError::Unauthorized {}.into());
    }
}

#[test]
fn test_init_asset_with_msg_symbol() {
    let mut deps = th_setup(&[]);
    let dynamic_ir_params = DynamicInterestRateModelParams {
        min_borrow_rate: Decimal::from_ratio(5u128, 100u128),
        max_borrow_rate: Decimal::from_ratio(50u128, 100u128),
        kp_1: Decimal::from_ratio(3u128, 1u128),
        optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
        kp_augmentation_threshold: Decimal::from_ratio(2000u128, 1u128),
        kp_2: Decimal::from_ratio(2u128, 1u128),
        update_threshold_seconds: 1,
        update_threshold_txs: 1,
    };
    let asset_params = InitOrUpdateAssetParams {
        initial_borrow_rate: Some(Decimal::from_ratio(20u128, 100u128)),
        max_loan_to_value: Some(Decimal::from_ratio(8u128, 10u128)),
        reserve_factor: Some(Decimal::from_ratio(1u128, 100u128)),
        liquidation_threshold: Some(Decimal::one()),
        liquidation_bonus: Some(Decimal::zero()),
        interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
            dynamic_ir_params.clone(),
        )),
        active: Some(true),
        deposit_enabled: Some(true),
        borrow_enabled: Some(true),
    };
    let msg = ExecuteMsg::InitAsset {
        denom: "someasset".to_string(),
        asset_params: asset_params.clone(),
        asset_symbol: Some("COIN".to_string()),
    };
    let info = mock_info("owner", &[]);
    let env = mock_env(MockEnvParams::default());
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // should instantiate a liquidity token
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: Some("protocol_admin".to_string()),
            code_id: 1u64,
            msg: to_binary(&ma_token::msg::InstantiateMsg {
                name: String::from("Mars COIN Liquidity Token"),
                symbol: String::from("maCOIN"),
                decimals: 6,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: MOCK_CONTRACT_ADDR.to_string(),
                    cap: None,
                }),
                init_hook: Some(ma_token::msg::InitHook {
                    contract_addr: MOCK_CONTRACT_ADDR.to_string(),
                    msg: to_binary(&ExecuteMsg::InitAssetTokenCallback {
                        denom: "someasset".to_string(),
                    })
                    .unwrap()
                }),
                marketing: Some(InstantiateMarketingInfo {
                    project: Some("Mars Protocol".to_string()),
                    description: Some(
                        "Interest earning token representing deposits for COIN".to_string()
                    ),

                    marketing: Some("protocol_admin".to_string()),
                    logo: None,
                }),
                red_bank_address: MOCK_CONTRACT_ADDR.to_string(),
                incentives_address: "incentives".to_string(),
            })
            .unwrap(),
            funds: vec![],
            label: "maCOIN".to_string()
        })),]
    );
}

#[test]
fn test_update_asset() {
    let mut deps = mock_dependencies(&[]);
    let start_time = 100000000;
    let env = mock_env_at_block_time(start_time);

    let config = CreateOrUpdateConfig {
        owner: Some("owner".to_string()),
        address_provider_address: Some("address_provider".to_string()),
        ma_token_code_id: Some(5u64),
        close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
    };
    let msg = InstantiateMsg {
        config,
    };
    let info = mock_info("owner", &[]);
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    let dynamic_ir_params = DynamicInterestRateModelParams {
        min_borrow_rate: Decimal::from_ratio(5u128, 100u128),
        max_borrow_rate: Decimal::from_ratio(50u128, 100u128),
        kp_1: Decimal::from_ratio(3u128, 1u128),
        optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
        kp_augmentation_threshold: Decimal::from_ratio(2000u128, 1u128),
        kp_2: Decimal::from_ratio(2u128, 1u128),

        update_threshold_txs: 1,
        update_threshold_seconds: 1,
    };

    let asset_params = InitOrUpdateAssetParams {
        initial_borrow_rate: Some(Decimal::from_ratio(20u128, 100u128)),
        max_loan_to_value: Some(Decimal::from_ratio(50u128, 100u128)),
        reserve_factor: Some(Decimal::from_ratio(1u128, 100u128)),
        liquidation_threshold: Some(Decimal::from_ratio(80u128, 100u128)),
        liquidation_bonus: Some(Decimal::from_ratio(10u128, 100u128)),
        interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
            dynamic_ir_params.clone(),
        )),
        active: Some(true),
        deposit_enabled: Some(true),
        borrow_enabled: Some(true),
    };

    // non owner is not authorized
    {
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            asset_params: asset_params.clone(),
        };
        let info = mock_info("somebody", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, MarsError::Unauthorized {}.into());
    }

    // owner is authorized but can't update asset if not initialized first
    {
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            asset_params: asset_params.clone(),
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(error_res, ContractError::AssetNotInitialized {});
    }

    // initialize asset
    {
        let msg = ExecuteMsg::InitAsset {
            denom: "someasset".to_string(),
            asset_params: asset_params.clone(),
            asset_symbol: None,
        };
        let info = mock_info("owner", &[]);
        let _res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
    }

    // update asset with max_loan_to_value greater than 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(11u128, 10u128)),
            ..asset_params.clone()
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            asset_params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::Market(
                MarsError::InvalidParam {
                    param_name: "max_loan_to_value".to_string(),
                    invalid_value: "1.1".to_string(),
                    predicate: "<= 1".to_string(),
                }
                .into()
            )
        );
    }

    // update asset with liquidation_threshold greater than 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            liquidation_threshold: Some(Decimal::from_ratio(11u128, 10u128)),
            ..asset_params.clone()
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            asset_params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::Market(
                MarsError::InvalidParam {
                    param_name: "liquidation_threshold".to_string(),
                    invalid_value: "1.1".to_string(),
                    predicate: "<= 1".to_string(),
                }
                .into()
            )
        );
    }

    // update asset with liquidation_bonus greater than 1
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            liquidation_bonus: Some(Decimal::from_ratio(11u128, 10u128)),
            ..asset_params.clone()
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            asset_params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::Market(
                MarsError::InvalidParam {
                    param_name: "liquidation_bonus".to_string(),
                    invalid_value: "1.1".to_string(),
                    predicate: "<= 1".to_string(),
                }
                .into()
            )
        );
    }

    // update asset where LTV >= liquidity threshold
    {
        let invalid_asset_params = InitOrUpdateAssetParams {
            max_loan_to_value: Some(Decimal::from_ratio(6u128, 10u128)),
            liquidation_threshold: Some(Decimal::from_ratio(5u128, 10u128)),
            ..asset_params
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            asset_params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::Market(MarketError::InvalidLiquidationThreshold {
                liquidation_threshold: Decimal::from_ratio(1u128, 2u128),
                max_loan_to_value: Decimal::from_ratio(6u128, 10u128)
            })
        );
    }

    // update asset where min borrow rate >= max borrow rate
    {
        let invalid_dynamic_ir_params = DynamicInterestRateModelParams {
            min_borrow_rate: Decimal::from_ratio(5u128, 10u128),
            max_borrow_rate: Decimal::from_ratio(4u128, 10u128),
            ..dynamic_ir_params
        };
        let invalid_asset_params = InitOrUpdateAssetParams {
            interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                invalid_dynamic_ir_params.clone(),
            )),
            ..asset_params
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            asset_params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::InterestRateModel(InterestRateModelError::InvalidMinMaxBorrowRate {
                min_borrow_rate: Decimal::from_ratio(5u128, 10u128),
                max_borrow_rate: Decimal::from_ratio(4u128, 10u128)
            })
        );
    }

    // update asset where optimal utilization rate > 1
    {
        let invalid_dynamic_ir_params = DynamicInterestRateModelParams {
            optimal_utilization_rate: Decimal::from_ratio(11u128, 10u128),
            ..dynamic_ir_params
        };
        let invalid_asset_params = InitOrUpdateAssetParams {
            interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                invalid_dynamic_ir_params.clone(),
            )),
            ..asset_params
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            asset_params: invalid_asset_params,
        };
        let info = mock_info("owner", &[]);
        let error_res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::InterestRateModel(
                InterestRateModelError::InvalidOptimalUtilizationRate {}
            )
        );
    }

    // update asset with new params
    {
        let dynamic_ir_params = DynamicInterestRateModelParams {
            min_borrow_rate: Decimal::from_ratio(5u128, 100u128),
            max_borrow_rate: Decimal::from_ratio(50u128, 100u128),
            kp_1: Decimal::from_ratio(3u128, 1u128),
            optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
            kp_augmentation_threshold: Decimal::from_ratio(2000u128, 1u128),
            kp_2: Decimal::from_ratio(2u128, 1u128),
            update_threshold_txs: 1,
            update_threshold_seconds: 1,
        };
        let asset_params = InitOrUpdateAssetParams {
            initial_borrow_rate: Some(Decimal::from_ratio(20u128, 100u128)),
            max_loan_to_value: Some(Decimal::from_ratio(60u128, 100u128)),
            reserve_factor: Some(Decimal::from_ratio(10u128, 100u128)),
            liquidation_threshold: Some(Decimal::from_ratio(90u128, 100u128)),
            liquidation_bonus: Some(Decimal::from_ratio(12u128, 100u128)),
            interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
                dynamic_ir_params.clone(),
            )),
            active: Some(true),
            deposit_enabled: Some(true),
            borrow_enabled: Some(true),
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            asset_params: asset_params.clone(),
        };
        let info = mock_info("owner", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let new_market = MARKETS.load(&deps.storage, "someasset").unwrap();
        assert_eq!(0, new_market.index);
        assert_eq!(asset_params.max_loan_to_value.unwrap(), new_market.max_loan_to_value);
        assert_eq!(asset_params.reserve_factor.unwrap(), new_market.reserve_factor);
        assert_eq!(asset_params.liquidation_threshold.unwrap(), new_market.liquidation_threshold);
        assert_eq!(asset_params.liquidation_bonus.unwrap(), new_market.liquidation_bonus);
        assert_eq!(
            InterestRateModel::Dynamic {
                params: dynamic_ir_params,
                state: DynamicInterestRateModelState {
                    txs_since_last_borrow_rate_update: 1,
                    borrow_rate_last_updated: env.block.time.seconds(),
                }
            },
            new_market.interest_rate_model
        );

        let new_market_denom = MARKET_DENOMS_BY_INDEX.load(&deps.storage, 0).unwrap();
        assert_eq!("someasset", &new_market_denom);

        let new_money_market = GLOBAL_STATE.load(&deps.storage).unwrap();
        assert_eq!(new_money_market.market_count, 1);

        assert_eq!(res.messages, vec![],);

        assert_eq!(
            res.attributes,
            vec![attr("action", "update_asset"), attr("denom", "someasset")],
        );
    }

    // update asset with empty params
    {
        let market_before = MARKETS.load(&deps.storage, "someasset").unwrap();

        let empty_asset_params = InitOrUpdateAssetParams {
            initial_borrow_rate: None,
            max_loan_to_value: None,
            reserve_factor: None,
            liquidation_threshold: None,
            liquidation_bonus: None,
            interest_rate_model_params: None,
            active: None,
            deposit_enabled: None,
            borrow_enabled: None,
        };
        let msg = ExecuteMsg::UpdateAsset {
            denom: "someasset".to_string(),
            asset_params: empty_asset_params,
        };
        let info = mock_info("owner", &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // no interest updated event
        assert_eq!(res.events.len(), 0);

        let new_market = MARKETS.load(&deps.storage, "someasset").unwrap();
        // should keep old params
        assert_eq!(0, new_market.index);
        assert_eq!(market_before.borrow_rate, new_market.borrow_rate);
        assert_eq!(market_before.max_loan_to_value, new_market.max_loan_to_value);
        assert_eq!(market_before.reserve_factor, new_market.reserve_factor);
        assert_eq!(market_before.liquidation_threshold, new_market.liquidation_threshold);
        assert_eq!(market_before.liquidation_bonus, new_market.liquidation_bonus);
        if let InterestRateModel::Dynamic {
            params: market_dynamic_ir_params,
            state: market_dynamic_ir_state,
        } = new_market.interest_rate_model
        {
            assert_eq!(dynamic_ir_params.min_borrow_rate, market_dynamic_ir_params.min_borrow_rate);
            assert_eq!(dynamic_ir_params.max_borrow_rate, market_dynamic_ir_params.max_borrow_rate);
            assert_eq!(dynamic_ir_params.kp_1, market_dynamic_ir_params.kp_1);
            assert_eq!(
                dynamic_ir_params.kp_augmentation_threshold,
                market_dynamic_ir_params.kp_augmentation_threshold
            );
            assert_eq!(dynamic_ir_params.kp_2, market_dynamic_ir_params.kp_2);
            assert_eq!(
                dynamic_ir_params.update_threshold_txs,
                market_dynamic_ir_params.update_threshold_txs
            );
            assert_eq!(
                dynamic_ir_params.update_threshold_seconds,
                market_dynamic_ir_params.update_threshold_seconds
            );

            assert_eq!(1, market_dynamic_ir_state.txs_since_last_borrow_rate_update);
            assert_eq!(env.block.time.seconds(), market_dynamic_ir_state.borrow_rate_last_updated);
        } else {
            panic!("INCORRECT STRATEGY")
        }
    }
}

#[test]
fn test_update_asset_with_new_interest_rate_model_params() {
    let mut deps = mock_dependencies(&[]);

    let config = CreateOrUpdateConfig {
        owner: Some("owner".to_string()),
        address_provider_address: Some("address_provider".to_string()),
        ma_token_code_id: Some(5u64),
        close_factor: Some(Decimal::from_ratio(1u128, 2u128)),
    };
    let msg = InstantiateMsg {
        config,
    };
    let info = mock_info("owner", &[]);
    let env = mock_env(MockEnvParams::default());
    instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

    let dynamic_ir_params = DynamicInterestRateModelParams {
        min_borrow_rate: Decimal::from_ratio(10u128, 100u128),
        max_borrow_rate: Decimal::from_ratio(60u128, 100u128),
        kp_1: Decimal::from_ratio(4u128, 1u128),
        optimal_utilization_rate: Decimal::from_ratio(90u128, 100u128),
        kp_augmentation_threshold: Decimal::from_ratio(2000u128, 1u128),
        kp_2: Decimal::from_ratio(3u128, 1u128),
        update_threshold_txs: 1,
        update_threshold_seconds: 1,
    };

    let asset_params_with_dynamic_ir = InitOrUpdateAssetParams {
        initial_borrow_rate: Some(Decimal::from_ratio(15u128, 100u128)),
        max_loan_to_value: Some(Decimal::from_ratio(50u128, 100u128)),
        reserve_factor: Some(Decimal::from_ratio(2u128, 100u128)),
        liquidation_threshold: Some(Decimal::from_ratio(80u128, 100u128)),
        liquidation_bonus: Some(Decimal::from_ratio(10u128, 100u128)),
        interest_rate_model_params: Some(InterestRateModelParams::Dynamic(
            dynamic_ir_params.clone(),
        )),
        active: Some(true),
        deposit_enabled: Some(true),
        borrow_enabled: Some(true),
    };

    let msg = ExecuteMsg::InitAsset {
        denom: "someasset".to_string(),
        asset_params: asset_params_with_dynamic_ir.clone(),
        asset_symbol: None,
    };
    let info = mock_info("owner", &[]);
    let env = mock_env_at_block_time(1_000_000);
    let _res = execute(deps.as_mut(), env, info, msg).unwrap();

    // Verify if IR model is saved correctly
    let market_before = MARKETS.load(&deps.storage, "someasset").unwrap();
    assert_eq!(
        market_before.interest_rate_model,
        InterestRateModel::Dynamic {
            params: dynamic_ir_params,
            state: DynamicInterestRateModelState {
                txs_since_last_borrow_rate_update: 0,
                borrow_rate_last_updated: 1_000_000
            }
        }
    );

    let linear_ir_params = LinearInterestRateModelParams {
        optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
        base: Decimal::from_ratio(0u128, 100u128),
        slope_1: Decimal::from_ratio(8u128, 100u128),
        slope_2: Decimal::from_ratio(48u128, 100u128),
    };
    let asset_params_with_linear_ir = InitOrUpdateAssetParams {
        interest_rate_model_params: Some(InterestRateModelParams::Linear(linear_ir_params.clone())),
        ..asset_params_with_dynamic_ir
    };
    let msg = ExecuteMsg::UpdateAsset {
        denom: "someasset".to_string(),
        asset_params: asset_params_with_linear_ir.clone(),
    };
    let info = mock_info("owner", &[]);
    let env = mock_env_at_block_time(2_000_000);
    let res = execute(deps.as_mut(), env, info, msg).unwrap();

    // Verify if IR model is updated
    let new_market = MARKETS.load(&deps.storage, "someasset").unwrap();
    assert_eq!(
        new_market.interest_rate_model,
        InterestRateModel::Linear {
            params: linear_ir_params.clone()
        }
    );

    // Indices should have been updated using previous interest rate
    let expected_indices = th_get_expected_indices(&market_before, 2_000_000);
    assert_eq!(new_market.liquidity_index, expected_indices.liquidity);
    assert_eq!(new_market.borrow_index, expected_indices.borrow);
    assert_eq!(new_market.indexes_last_updated, 2_000_000);

    // Interest rate should have been recomputed using new strategy and values
    let expected_borrow_rate = linear_get_borrow_rate(&linear_ir_params, Decimal::zero()).unwrap();
    let expected_liquidity_rate = Decimal::zero(); // zero utilization rate
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
fn test_update_asset_new_reserve_factor_accrues_interest_rate() {
    let asset_liquidity = Uint128::from(10_000_000_000000_u128);
    let mut deps = th_setup(&[coin(asset_liquidity.into(), "somecoin")]);

    let ma_token_address = Addr::unchecked("ma_token");

    let linear_ir_model_params = LinearInterestRateModelParams {
        optimal_utilization_rate: Decimal::from_ratio(80u128, 100u128),
        base: Decimal::zero(),
        slope_1: Decimal::from_ratio(1_u128, 2_u128),
        slope_2: Decimal::from_ratio(2_u128, 1_u128),
    };
    let linear_ir_model = InterestRateModel::Linear {
        params: linear_ir_model_params.clone(),
    };

    let asset_initial_debt = Uint128::new(2_000_000_000000);
    let market_before = th_init_market(
        deps.as_mut(),
        "somecoin",
        &Market {
            reserve_factor: Decimal::from_ratio(1_u128, 10_u128),
            borrow_index: Decimal::one(),
            liquidity_index: Decimal::one(),
            indexes_last_updated: 1_000_000,
            borrow_rate: Decimal::from_ratio(12u128, 100u128),
            liquidity_rate: Decimal::from_ratio(12u128, 100u128),
            debt_total_scaled: compute_scaled_amount(
                asset_initial_debt,
                Decimal::one(),
                ScalingOperation::Ceil,
            )
            .unwrap(),
            ma_token_address: ma_token_address,
            interest_rate_model: linear_ir_model.clone(),
            ..Default::default()
        },
    );

    let asset_params = InitOrUpdateAssetParams {
        initial_borrow_rate: None,
        max_loan_to_value: None,
        reserve_factor: Some(Decimal::from_ratio(2_u128, 10_u128)),
        liquidation_threshold: None,
        liquidation_bonus: None,
        interest_rate_model_params: None,
        active: None,
        deposit_enabled: None,
        borrow_enabled: None,
    };
    let msg = ExecuteMsg::UpdateAsset {
        denom: "somecoin".to_string(),
        asset_params: asset_params,
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
    let expected_utilization_rate =
        Decimal::from_ratio(expected_debt, expected_liquidity + expected_debt);

    let expected_borrow_rate =
        linear_get_borrow_rate(&linear_ir_model_params, expected_utilization_rate).unwrap();

    let expected_liquidity_rate = get_liquidity_rate(
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
    let expected_protocol_rewards = interest_accrued * market_before.reserve_factor;
    // mint message is sent because debt is non zero
    assert_eq!(
        res.messages,
        vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: market_before.ma_token_address.to_string(),
            msg: to_binary(&ma_token::msg::ExecuteMsg::Mint {
                recipient: "protocol_rewards_collector".to_string(),
                amount: compute_scaled_amount(
                    expected_protocol_rewards,
                    new_market.liquidity_index,
                    ScalingOperation::Truncate
                )
                .unwrap()
            })
            .unwrap(),
            funds: vec![]
        })),]
    );
}

#[test]
fn test_init_asset_callback_cannot_be_called_on_its_own() {
    let mut deps = th_setup(&[]);

    let env = mock_env(MockEnvParams::default());
    let info = mock_info("mtokencontract", &[]);
    let msg = ExecuteMsg::InitAssetTokenCallback {
        denom: "uluna".to_string(),
    };
    let error_res = execute(deps.as_mut(), env, info, msg).unwrap_err();
    assert_eq!(error_res, StdError::not_found("mars_outpost::red_bank::Market").into());
}
