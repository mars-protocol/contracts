use super::*;

    use cosmwasm_std::{
        attr, coin, coins, from_binary,
        testing::{mock_env, MockApi, MockStorage},
        BankMsg, Coin, Decimal, OwnedDeps, SubMsg,
    };

    use mars_outpost::testing::{mock_dependencies, mock_info, MarsMockQuerier};
    use osmo_bindings::Swap;

    #[test]
    fn test_proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        // Config with base params valid (just update the rest)
        let base_config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            safety_tax_rate: Some(Decimal::from_ratio(5u128, 10u128)),
            safety_fund_asset: Some(Asset::Native {
                denom: "uusdc".to_string(),
            }),
            fee_collector_asset: Some(Asset::Native {
                denom: "umars".to_string(),
            }),
            channel_id: Some("channel-110".to_string()),
            revision: Some(1),
            block_timeout: Some(50),
        };

        let info = mock_info("owner");

        // *
        // init config with empty params
        // *
        let empty_config = CreateOrUpdateConfig {
            owner: None,
            address_provider_address: None,
            safety_tax_rate: None,
            safety_fund_asset: None,
            fee_collector_asset: None,
            channel_id: None,
            revision: None,
            block_timeout: None,
        };
        let msg = InstantiateMsg {
            config: empty_config,
        };
        let err = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(err, MarsError::InstantiateParamsUnavailable {}.into());

        // *
        // init config with safety_tax_rate greater than 1
        // *
        let mut safety_tax_rate = Decimal::from_ratio(11u128, 10u128);
        let config = CreateOrUpdateConfig {
            safety_tax_rate: Some(safety_tax_rate),
            ..base_config.clone()
        };
        let msg = InstantiateMsg {
            config,
        };
        let response = instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(
            response,
            ConfigError::Mars(MarsError::InvalidParam {
                param_name: "safety_tax_rate".to_string(),
                invalid_value: safety_tax_rate.to_string(),
                predicate: "<= 1".to_string(),
            })
            .into()
        );

        // *
        // init config with valid params
        // *
        safety_tax_rate = Decimal::from_ratio(5u128, 10u128);
        let config = CreateOrUpdateConfig {
            safety_tax_rate: Some(safety_tax_rate),
            ..base_config
        };
        let msg = InstantiateMsg {
            config,
        };

        // we can just call .unwrap() to assert this was a success
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let value: Config = from_binary(&res).unwrap();
        assert_eq!(value.owner, "owner");
        assert_eq!(value.address_provider_address, "address_provider");
        assert_eq!(value.safety_tax_rate, safety_tax_rate);
        assert_eq!(
            value.safety_fund_asset,
            Asset::Native {
                denom: "uusdc".to_string()
            }
        );
        assert_eq!(
            value.fee_collector_asset,
            Asset::Native {
                denom: "umars".to_string()
            }
        );
    }

    #[test]
    fn test_update_config() {
        let mut deps = th_setup(&[]);

        let mut safety_tax_rate = Decimal::percent(10);
        let base_config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            safety_tax_rate: Some(safety_tax_rate),
            safety_fund_asset: Some(Asset::Native {
                denom: "uusdc".to_string(),
            }),
            fee_collector_asset: Some(Asset::Native {
                denom: "umars".to_string(),
            }),
            channel_id: Some("channel-182".to_string()),
            revision: Some(1),
            block_timeout: Some(50),
        };

        // *
        // non owner is not authorized
        // *
        let msg = ExecuteMsg::UpdateConfig {
            config: base_config.clone(),
        };
        let info = mock_info("somebody");
        let error_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(error_res, MarsError::Unauthorized {}.into());

        // *
        // update config with safety_tax_rate greater than 1
        // *
        let info = mock_info("owner");

        safety_tax_rate = Decimal::from_ratio(11u128, 10u128);
        let config = CreateOrUpdateConfig {
            owner: None,
            safety_tax_rate: Some(safety_tax_rate),
            ..base_config.clone()
        };
        let msg = ExecuteMsg::UpdateConfig {
            config,
        };
        let error_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ConfigError::Mars(MarsError::InvalidParam {
                param_name: "safety_tax_rate".to_string(),
                invalid_value: safety_tax_rate.to_string(),
                predicate: "<= 1".to_string(),
            })
            .into()
        );

        // *
        // update config with safety_tax_rate greater than 1
        // *
        safety_tax_rate = Decimal::from_ratio(12u128, 10u128);
        let config = CreateOrUpdateConfig {
            owner: None,
            safety_tax_rate: Some(safety_tax_rate),
            ..base_config
        };
        let msg = ExecuteMsg::UpdateConfig {
            config,
        };
        let info = mock_info("owner");
        let error_res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(
            error_res,
            ConfigError::Mars(MarsError::InvalidParam {
                param_name: "safety_tax_rate".to_string(),
                invalid_value: safety_tax_rate.to_string(),
                predicate: "<= 1".to_string(),
            })
            .into()
        );

        // *
        // update config with all new params
        // *
        safety_tax_rate = Decimal::from_ratio(5u128, 100u128);
        let config = CreateOrUpdateConfig {
            owner: Some("new_owner".to_string()),
            address_provider_address: Some("new_address_provider".to_string()),
            safety_tax_rate: Some(safety_tax_rate),
            safety_fund_asset: Some(Asset::Native {
                denom: "uatom".to_string(),
            }),
            fee_collector_asset: Some(Asset::Native {
                denom: "uosmo".to_string(),
            }),
            channel_id: Some("channel-182".to_string()),
            revision: Some(1),
            block_timeout: Some(50),
        };
        let msg = ExecuteMsg::UpdateConfig {
            config: config.clone(),
        };
        // we can just call .unwrap() to assert this was a success
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Read config from state
        let new_config = CONFIG.load(&deps.storage).unwrap();

        assert_eq!(new_config.owner, config.owner.unwrap());
        assert_eq!(new_config.address_provider_address, config.address_provider_address.unwrap());
        assert_eq!(new_config.safety_tax_rate, config.safety_tax_rate.unwrap());
        assert_eq!(new_config.safety_tax_rate, config.safety_tax_rate.unwrap());
        assert_eq!(new_config.safety_fund_asset, config.safety_fund_asset.unwrap());
        assert_eq!(new_config.fee_collector_asset, config.fee_collector_asset.unwrap());
    }

    #[test]
    fn test_execute_withdraw_from_red_bank() {
        let mut deps = th_setup(&[]);

        // *
        // anyone can execute a withdrawal
        // *
        let asset = Asset::Native {
            denom: "somecoin".to_string(),
        };
        let amount = Uint128::new(123_456);
        let msg = ExecuteMsg::WithdrawFromRedBank {
            asset: asset.clone(),
            amount: Some(amount),
        };
        let info = mock_info("anybody");
        // we can just call .unwrap() to assert this was a success
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "red_bank".to_string(),
                msg: to_binary(&red_bank::msg::ExecuteMsg::Withdraw {
                    asset,
                    amount: Some(amount),
                    recipient: None
                })
                .unwrap(),
                funds: vec![]
            }))]
        );
        assert_eq!(res.attributes, vec![attr("action", "withdraw_from_red_bank"),]);
    }

    #[test]
    fn test_distribute_protocol_rewards() {
        let balance = 2_000_000_000u128;

        // initialize contract with balance
        let mut deps = th_setup(&[coin(balance, "uusdc"), coin(1_000_000, "umars")]);

        // call function on an asset that isn't enabled for distribution
        let permissible_amount = Uint128::new(1_500_000_000);
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: Asset::Native {
                denom: "uosmo".to_string(),
            },
            amount: Some(permissible_amount),
        };
        let info = mock_info("anybody");
        let error_res = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::AssetNotEnabledForDistribution {
                asset_label: "uosmo".to_string()
            }
        );

        // call function providing amount exceeding balance
        let exceeding_amount = Uint128::new(2_000_000_001);
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: Asset::Native {
                denom: "uusdc".to_string(),
            },
            amount: Some(exceeding_amount),
        };
        let info = mock_info("anybody");
        let error_res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap_err();
        assert_eq!(
            error_res,
            ContractError::AmountToDistributeTooLarge {
                amount: exceeding_amount,
                balance: Uint128::new(balance)
            }
        );

        // call function providing an amount less than the balance, and distribute safety fund rewards ("uusdc")
        let permissible_amount = Uint128::new(1_500_000_000);
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: Asset::Native {
                denom: "uusdc".to_string(),
            },
            amount: Some(permissible_amount),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Ibc(IbcMsg::Transfer {
                channel_id: "channel-182".to_string(),
                to_address: "safety_fund".to_string(),
                amount: Coin {
                    denom: "uusdc".to_string(),
                    amount: permissible_amount
                },
                timeout: IbcTimeout::with_block(IbcTimeoutBlock {
                    revision: 1,
                    height: 12395,
                })
            }))]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "distribute_protocol_income"),
                attr("asset", "uusdc"),
                attr("amount_to_distribute", permissible_amount),
            ]
        );

        // call function without providing an amount, and distribute fee collector rewards ("umars")
        let msg = ExecuteMsg::DistributeProtocolRewards {
            asset: Asset::Native {
                denom: "umars".to_string(),
            },
            amount: None,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Ibc(IbcMsg::Transfer {
                channel_id: "channel-182".to_string(),
                to_address: "fee_collector".to_string(),
                amount: Coin {
                    denom: "umars".to_string(),
                    amount: Uint128::new(1_000_000)
                },
                timeout: IbcTimeout::with_block(IbcTimeoutBlock {
                    revision: 1,
                    height: 12395,
                })
            }))]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "distribute_protocol_income"),
                attr("asset", "umars"),
                attr("amount_to_distribute", Uint128::new(1_000_000)),
            ]
        );
    }

    #[test]
    fn test_execute_swap_msg() {
        // initialize contract with balance
        let mut deps = th_setup(&coins(500_000, "uatom"));
        let info = mock_info("owner");

        let msg = ExecuteMsg::SwapAsset {
            asset_in: Asset::Native {
                denom: "uatom".to_string(),
            },
            amount: None,
            fee_collector_asset_steps: vec![
                Step {
                    pool_id: 1,
                    denom_out: "uosmo".to_string(),
                },
                Step {
                    pool_id: 3,
                    denom_out: "umars".to_string(),
                },
            ],
            safety_fund_asset_steps: vec![
                Step {
                    pool_id: 1,
                    denom_out: "uosmo".to_string(),
                },
                Step {
                    pool_id: 2,
                    denom_out: "uusdc".to_string(),
                },
            ],
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), msg.clone()).unwrap();

        assert_eq!(
            res.messages,
            vec![
                SubMsg::new(CosmosMsg::Custom(OsmosisMsg::Swap {
                    first: Swap {
                        pool_id: 1,
                        denom_in: "uatom".to_string(),
                        denom_out: "uosmo".to_string(),
                    },
                    route: vec![Step {
                        pool_id: 2,
                        denom_out: "uusdc".to_string(),
                    }],
                    amount: osmo_bindings::SwapAmountWithLimit::ExactIn {
                        input: Uint128::new(250_000),
                        min_output: Uint128::zero()
                    }
                })),
                SubMsg::new(CosmosMsg::Custom(OsmosisMsg::Swap {
                    first: Swap {
                        pool_id: 1,
                        denom_in: "uatom".to_string(),
                        denom_out: "uosmo".to_string(),
                    },
                    route: vec![Step {
                        pool_id: 3,
                        denom_out: "umars".to_string(),
                    }],
                    amount: osmo_bindings::SwapAmountWithLimit::ExactIn {
                        input: Uint128::new(250_000),
                        min_output: Uint128::zero()
                    }
                }))
            ]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "swap"),
                attr("denom_in", "uatom"),
                attr("amount_to_swap", "500000"),
                attr("safety_fund_share", "250000"),
                attr("fee_collector_share", "250000"),
            ]
        );

        // test swap all the amount to safety fund (safety fund tax rate = 1)
        let config = CreateOrUpdateConfig {
            owner: None,
            address_provider_address: None,
            fee_collector_asset: None,
            safety_fund_asset: None,
            channel_id: None,
            revision: None,
            block_timeout: None,
            safety_tax_rate: Some(Decimal::percent(100)),
        };
        let conf_msg = ExecuteMsg::UpdateConfig {
            config,
        };

        // change the safety_tax_rate to 1
        let _ = execute(deps.as_mut(), mock_env(), info.clone(), conf_msg).unwrap();

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert_eq!(
            res.messages,
            vec![SubMsg::new(CosmosMsg::Custom(OsmosisMsg::Swap {
                first: Swap {
                    pool_id: 1,
                    denom_in: "uatom".to_string(),
                    denom_out: "uosmo".to_string(),
                },
                route: vec![Step {
                    pool_id: 2,
                    denom_out: "uusdc".to_string(),
                }],
                amount: osmo_bindings::SwapAmountWithLimit::ExactIn {
                    input: Uint128::new(500_000),
                    min_output: Uint128::zero()
                }
            })),]
        );

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "swap"),
                attr("denom_in", "uatom"),
                attr("amount_to_swap", "500000"),
                attr("safety_fund_share", "500000"),
                attr("fee_collector_share", "0"),
            ]
        );
    }

    #[test]
    fn test_execute_cosmos_msg() {
        let mut deps = th_setup(&[]);

        let bank = BankMsg::Send {
            to_address: "destination".to_string(),
            amount: vec![Coin {
                denom: "uluna".to_string(),
                amount: Uint128::new(123456),
            }],
        };
        let cosmos_msg = CosmosMsg::Bank(bank);
        let msg = ExecuteMsg::ExecuteCosmosMsg(cosmos_msg.clone());

        // *
        // non owner is not authorized
        // *
        let info = mock_info("somebody");
        let error_res = execute(deps.as_mut(), mock_env(), info, msg.clone()).unwrap_err();
        assert_eq!(error_res, MarsError::Unauthorized {}.into());

        // *
        // can execute Cosmos msg
        // *
        let info = mock_info("owner");
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.messages, vec![SubMsg::new(cosmos_msg)]);
        assert_eq!(res.attributes, vec![attr("action", "execute_cosmos_msg")]);
    }

    // TEST HELPERS

    fn th_setup(contract_balances: &[Coin]) -> OwnedDeps<MockStorage, MockApi, MarsMockQuerier> {
        let mut deps = mock_dependencies(contract_balances);
        let info = mock_info("owner");
        let config = CreateOrUpdateConfig {
            owner: Some("owner".to_string()),
            address_provider_address: Some("address_provider".to_string()),
            safety_tax_rate: Some(Decimal::percent(50)),
            safety_fund_asset: Some(Asset::Native {
                denom: "uusdc".to_string(),
            }),
            fee_collector_asset: Some(Asset::Native {
                denom: "umars".to_string(),
            }),
            channel_id: Some("channel-182".to_string()),
            revision: Some(1),
            block_timeout: Some(50),
        };
        let msg = InstantiateMsg {
            config,
        };
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        deps
    }