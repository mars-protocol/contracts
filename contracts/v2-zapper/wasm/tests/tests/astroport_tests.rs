mod tests {
    use apollo_cw_asset::{Asset, AssetInfo, AssetInfoBase, AssetList};
    use apollo_utils::assets::separate_natives_and_cw20s;
    use apollo_utils::coins::coin_from_str;
    use apollo_utils::submessages::{find_event, parse_attribute_value};
    use astroport::factory::PairType;
    use astroport_v3::asset::Asset as AstroportAsset;
    use cosmwasm_std::{assert_approx_eq, coin, coins, Addr, Coin, SubMsgResponse, Uint128};

    use cw_dex_test_contract::msg::{AstroportExecuteMsg, ExecuteMsg, QueryMsg};
    use cw_dex_test_helpers::astroport::setup_pool_and_test_contract;
    use cw_dex_test_helpers::{cw20_balance_query, cw20_transfer, query_asset_balance};
    use cw_it::astroport::utils::AstroportContracts;
    use cw_it::helpers::Unwrap;
    use cw_it::multi_test::MultiTestRunner;
    use cw_it::test_tube::cosmrs::proto::cosmwasm::wasm::v1::MsgExecuteContractResponse;
    use cw_it::test_tube::{
        Account, ExecuteResponse, Module, Runner, RunnerResult, SigningAccount, Wasm,
    };
    use cw_it::traits::CwItRunner;
    use cw_it::{OwnedTestRunner, TestRunner};
    use test_case::test_case;

    use cw_dex_astroport::AstroportPool;

    #[cfg(feature = "osmosis-test-tube")]
    use cw_it::osmosis_test_tube::OsmosisTestApp;

    pub fn get_test_runner<'a>() -> OwnedTestRunner<'a> {
        match option_env!("TEST_RUNNER").unwrap_or("multi-test") {
            "multi-test" => OwnedTestRunner::MultiTest(MultiTestRunner::new("osmo")),
            #[cfg(feature = "osmosis-test-tube")]
            "osmosis-test-tube" => OwnedTestRunner::OsmosisTestApp(OsmosisTestApp::new()),
            _ => panic!("Unsupported test runner type"),
        }
    }
    const TEST_CONTRACT_WASM_FILE_PATH: &str =
        "../target/wasm32-unknown-unknown/release/astroport_test_contract.wasm";

    fn setup_pool_and_testing_contract<'a>(
        runner: &'a TestRunner<'a>,
        pool_type: PairType,
        initial_liquidity: Vec<(&str, u64)>,
    ) -> RunnerResult<(
        Vec<SigningAccount>,
        String,
        String,
        String,
        AssetList,
        AstroportContracts,
    )> {
        setup_pool_and_test_contract(
            runner,
            pool_type,
            initial_liquidity,
            2,
            TEST_CONTRACT_WASM_FILE_PATH,
        )
    }

    #[test_case(PairType::Xyk { }, vec![("uluna",1_000_000), ("astro", 1_000_000)]; "provide_liquidity: native-cw20")]
    #[test_case(PairType::Xyk { }, vec![("apollo",1_000_000), ("astro", 1_000_000)]; "provide_liquidity: cw20-cw20")]
    #[test_case(PairType::Stable { }, vec![("uluna",1_000_000), ("astro", 1_000_000)]; "provide_liquidity: stableswap native-cw20")]
    #[test_case(PairType::Stable { }, vec![("apollo",1_000_000), ("astro", 1_000_000)]; "provide_liquidity: stableswap cw20-cw20")]
    #[test_case(PairType::Stable { }, vec![("uluna",1_000_000), ("uatom", 1_000_000)]; "provide_liquidity: stableswap native-native")]
    #[test_case(PairType::Custom("concentrated".to_string()), vec![("uluna",1_000_000), ("astro", 1_000_000)]; "provide_liquidity: concentrated native-cw20")]
    #[test_case(PairType::Custom("concentrated".to_string()), vec![("apollo",1_000_000), ("astro", 1_000_000)]; "provide_liquidity: concentrated cw20-cw20")]
    #[test_case(PairType::Custom("concentrated".to_string()), vec![("uluna",1_000_000), ("uatom", 1_000_000)]; "provide_liquidity: concentrated native-native")]
    pub fn test_provide_liquidity(pool_type: PairType, initial_liquidity: Vec<(&str, u64)>) {
        let owned_runner = get_test_runner();
        let runner = owned_runner.as_ref();
        let (accs, lp_token_addr, _pair_addr, contract_addr, asset_list, _) =
            setup_pool_and_testing_contract(&runner, pool_type.clone(), initial_liquidity).unwrap();
        let admin = &accs[0];
        let wasm = Wasm::new(&runner);

        // Check contract's LP token balance before providing liquidity
        let lp_token_before =
            cw20_balance_query(&runner, lp_token_addr.clone(), contract_addr.clone()).unwrap();
        assert_eq!(lp_token_before, Uint128::zero());

        // Simulate Provide Liquidity. Not supported for concentrated liquidity, so we
        // just make sure to use the right amounts of input assets
        let expected_out = match &pool_type {
            PairType::Custom(_) => Uint128::new(1000000),
            _ => {
                let simulate_query = QueryMsg::SimulateProvideLiquidity {
                    assets: asset_list.clone(),
                };
                wasm.query(&contract_addr, &simulate_query).unwrap()
            }
        };

        let (funds, cw20s) = separate_natives_and_cw20s(&asset_list);

        // Send cw20 tokens to the contract
        for cw20 in cw20s {
            cw20_transfer(
                &runner,
                cw20.address,
                contract_addr.clone(),
                cw20.amount,
                admin,
            )
            .unwrap();
        }

        // Provide liquidity with min_out one more than expected_out. Should fail.
        let unwrap = Unwrap::Err("Slippage is more than expected");
        let min_out = expected_out + Uint128::one();
        let provide_msg = ExecuteMsg::ProvideLiquidity {
            assets: asset_list.clone(),
            min_out,
        };
        unwrap.unwrap(runner.execute_cosmos_msgs::<MsgExecuteContractResponse>(
            &[provide_msg.into_cosmos_msg(contract_addr.clone(), funds.clone())],
            admin,
        ));

        // Provide liquidity with expected_out as min_out. Should succeed.
        let provide_msg = ExecuteMsg::ProvideLiquidity {
            assets: asset_list.clone(),
            min_out: expected_out,
        };
        let _res = runner
            .execute_cosmos_msgs::<MsgExecuteContractResponse>(
                &[provide_msg.into_cosmos_msg(contract_addr.clone(), funds)],
                admin,
            )
            .unwrap();

        // Query LP token balance after
        let lp_token_after =
            cw20_balance_query(&runner, lp_token_addr, contract_addr.clone()).unwrap();
        assert_eq!(lp_token_after, expected_out);

        // Query asset balances in contract, assert that all were used
        for asset in asset_list.into_iter() {
            let asset_balance = query_asset_balance(&runner, &asset.info, &contract_addr);
            assert_eq!(asset_balance, Uint128::zero());
        }
    }

    #[test_case(PairType::Xyk { }, vec![("uluna",1_000_000), ("astro", 1_000_000)]; "withdraw_liquidity: xyk native-cw20")]
    #[test_case(PairType::Xyk { }, vec![("apollo",1_000_000), ("astro", 1_000_000)]; "withdraw_liquidity: xyk cw20-cw20")]
    #[test_case(PairType::Stable { }, vec![("uluna",1_000_000), ("astro", 1_000_000)]; "withdraw_liquidity: stableswap native-cw20")]
    #[test_case(PairType::Stable { }, vec![("apollo",1_000_000), ("astro", 1_000_000)]; "withdraw_liquidity: stableswap cw20-cw20")]
    #[test_case(PairType::Stable { }, vec![("uluna",1_000_000), ("uatom", 1_000_000)]; "withdraw_liquidity: stableswap native-native")]
    #[test_case(PairType::Custom("concentrated".to_string()), vec![("uluna",1_000_000), ("astro", 1_000_000)]; "withdraw_liquidity: concentrated native-cw20")]
    #[test_case(PairType::Custom("concentrated".to_string()), vec![("apollo",1_000_000), ("astro", 1_000_000)]; "withdraw_liquidity: concentrated cw20-cw20")]
    #[test_case(PairType::Custom("concentrated".to_string()), vec![("uluna",1_000_000), ("uatom", 1_000_000)]; "withdraw_liquidity: concentrated native-native")]
    fn test_withdraw_liquidity(pool_type: PairType, initial_liquidity: Vec<(&str, u64)>) {
        let owned_runner = get_test_runner();
        let runner = owned_runner.as_ref();
        let (accs, lp_token_addr, _pair_addr, contract_addr, asset_list, _) =
            setup_pool_and_testing_contract(&runner, pool_type, initial_liquidity).unwrap();
        let admin = &accs[0];
        let wasm = Wasm::new(&runner);

        //Query admin LP token balance
        let admin_lp_token_balance =
            cw20_balance_query(&runner, lp_token_addr.clone(), admin.address()).unwrap();
        let amount_to_send = admin_lp_token_balance / Uint128::from(2u128);

        // Send LP tokens to contract
        cw20_transfer(
            &runner,
            lp_token_addr.clone(),
            contract_addr.clone(),
            amount_to_send,
            admin,
        )
        .unwrap();
        let contract_lp_token_balance =
            cw20_balance_query(&runner, lp_token_addr.clone(), contract_addr.clone()).unwrap();
        assert_eq!(contract_lp_token_balance, amount_to_send);

        // Simulate withdraw liquidity to get expected out assets
        let simulate_query = QueryMsg::SimulateWithdrawLiquidty {
            amount: contract_lp_token_balance,
        };
        let expected_out: AssetList = wasm.query(&contract_addr, &simulate_query).unwrap();

        // Withdraw liquidity with min_out one more than expected_out. Should fail.
        let unwrap = Unwrap::Err("but expected");
        let min_out: AssetList = expected_out
            .to_vec()
            .into_iter()
            .map(|mut a| {
                a.amount += Uint128::one();
                a
            })
            .collect::<Vec<_>>()
            .into();
        let withdraw_msg = ExecuteMsg::WithdrawLiquidity {
            amount: contract_lp_token_balance,
            min_out,
        };
        unwrap.unwrap(runner.execute_cosmos_msgs::<MsgExecuteContractResponse>(
            &[withdraw_msg.into_cosmos_msg(contract_addr.clone(), vec![])],
            admin,
        ));

        // Withdraw liquidity with expected_out as min_out. Should succeed.
        let withdraw_msg = ExecuteMsg::WithdrawLiquidity {
            amount: contract_lp_token_balance,
            min_out: expected_out.clone(),
        };
        runner
            .execute_cosmos_msgs::<MsgExecuteContractResponse>(
                &[withdraw_msg.into_cosmos_msg(contract_addr.clone(), vec![])],
                admin,
            )
            .unwrap();

        // Query LP token balance after
        let lp_token_balance_after =
            cw20_balance_query(&runner, lp_token_addr, contract_addr.clone()).unwrap();

        // Assert that LP token balance is zero after withdrawing all liquidity
        assert_eq!(lp_token_balance_after, Uint128::zero());

        // Query contract asset balances, assert that all were returned
        for asset in asset_list.into_iter() {
            let asset_balance = query_asset_balance(&runner, &asset.info, &contract_addr);
            let expected_balance = expected_out.find(&asset.info).unwrap().amount;
            assert_eq!(asset_balance, expected_balance);
        }
    }

    fn stake_all_lp_tokens<'a, R: Runner<'a>>(
        runner: &'a R,
        contract_addr: String,
        lp_token_addr: String,
        signer: &SigningAccount,
    ) -> ExecuteResponse<MsgExecuteContractResponse> {
        // Query LP token balance
        let lp_token_balance =
            cw20_balance_query(runner, lp_token_addr, contract_addr.clone()).unwrap();

        // Stake LP tokens
        let stake_msg = ExecuteMsg::Stake {
            amount: lp_token_balance,
        };

        runner
            .execute_cosmos_msgs::<MsgExecuteContractResponse>(
                &[stake_msg.into_cosmos_msg(contract_addr, vec![])],
                signer,
            )
            .unwrap()
    }

    #[test_case(PairType::Xyk {}, vec![("uluna",1_000_000), ("astro", 1_000_000)]; "stake_and_unstake: xyk native-cw20")]
    #[test_case(PairType::Xyk {}, vec![("apollo",1_000_000), ("astro", 1_000_000)]; "stake_and_unstake: xyk cw20-cw20")]
    #[test_case(PairType::Xyk {}, vec![("uluna",1_000_000), ("uatom", 1_000_000)]; "stake_and_unstake: xyk native-native")]
    #[test_case(PairType::Stable {}, vec![("uluna",1_000_000), ("astro", 1_000_000)]; "stake_and_unstake: stableswap native-cw20")]
    #[test_case(PairType::Stable {}, vec![("apollo",1_000_000), ("astro", 1_000_000)]; "stake_and_unstake: stableswap cw20-cw20")]
    #[test_case(PairType::Stable {}, vec![("uluna",1_000_000), ("uatom", 1_000_000)]; "stake_and_unstake: stableswap native-native")]
    #[test_case(PairType::Custom("concentrated".to_string()), vec![("uluna",1_000_000), ("astro", 1_000_000)]; "stake_and_unstake: concentrated native-cw20")]
    #[test_case(PairType::Custom("concentrated".to_string()), vec![("apollo",1_000_000), ("astro", 1_000_000)]; "stake_and_unstake: concentrated cw20-cw20")]
    #[test_case(PairType::Custom("concentrated".to_string()), vec![("uluna",1_000_000), ("uatom", 1_000_000)]; "stake_and_unstake: concentrated native-native")]
    fn test_stake_and_unstake(
        pool_type: PairType,
        initial_liquidity: Vec<(&str, u64)>,
    ) -> RunnerResult<()> {
        let owned_runner = get_test_runner();
        let runner = owned_runner.as_ref();
        let (accs, lp_token_addr, _pair_addr, contract_addr, _asset_list, _) =
            setup_pool_and_testing_contract(&runner, pool_type, initial_liquidity).unwrap();

        let admin = &accs[0];

        // Query LP token balance
        let lp_token_balance =
            cw20_balance_query(&runner, lp_token_addr.clone(), admin.address()).unwrap();

        // Send LP tokens to the test contract
        cw20_transfer(
            &runner,
            lp_token_addr.clone(),
            contract_addr.clone(),
            lp_token_balance,
            admin,
        )
        .unwrap();

        // Stake LP tokens
        let events =
            stake_all_lp_tokens(&runner, contract_addr.clone(), lp_token_addr.clone(), admin)
                .events;

        // Parse the event data
        let response = SubMsgResponse { events, data: None };

        let event = find_event(&response, "wasm").unwrap();
        let amount = coin_from_str(&parse_attribute_value::<String, _>(event, "amount").unwrap());

        // Assert the lock has correct amount
        assert_eq!(amount.amount, lp_token_balance);

        // Query LP token balance after
        let lp_token_balance_after =
            cw20_balance_query(&runner, lp_token_addr.clone(), contract_addr.to_string()).unwrap();

        // Assert that LP token balance is 0
        assert_eq!(lp_token_balance_after, Uint128::zero());

        // unstake LP tokens
        let unstake_msg = AstroportExecuteMsg::Unstake {
            amount: lp_token_balance,
        };
        runner
            .execute_cosmos_msgs::<MsgExecuteContractResponse>(
                &[unstake_msg.into_cosmos_msg(contract_addr.clone(), vec![])],
                admin,
            )
            .unwrap();

        // Query LP token balance
        let lp_token_balance_after_unstake =
            cw20_balance_query(&runner, lp_token_addr, contract_addr).unwrap();

        // Assert that LP tokens have been unstakeed
        assert_eq!(lp_token_balance_after_unstake, lp_token_balance);

        Ok(())
    }

    #[test_case(PairType::Xyk{},vec![("astro",1_000_000), ("uluna", 1_000_000)], Uint128::new(1_000_000); "swap_and_simulate_swap: basic pool")]
    #[test_case(PairType::Xyk{},vec![("uluna",1_000_000), ("astro", 1_000_000)], Uint128::new(2); "swap_and_simulate_swap: basic pool small amount")]
    #[test_case(PairType::Xyk{},vec![("uluna",1_000_000), ("astro", 1_000_000)], Uint128::new(100_000_000); "swap_and_simulate_swap: basic pool, high slippage")]
    #[test_case(PairType::Xyk{},vec![("uluna",68_582_147), ("astro", 3_467_256)], Uint128::new(1_000_000); "swap_and_simulate_swap: basic pool, random prices")]
    #[test_case(PairType::Stable { },vec![("uluna",1_000_000), ("astro", 1_000_000)], Uint128::new(1_000_000); "swap_and_simulate_swap: stable swap pool")]
    #[test_case(PairType::Stable { },vec![("uluna",1_000_000), ("astro", 1_000_000)], Uint128::new(100_000_000); "swap_and_simulate_swap: stable swap pool, high slippage")]
    #[test_case(PairType::Stable { },vec![("uluna",68_582_147), ("astro", 3_467_256)], Uint128::new(1_000_000); "swap_and_simulate_swap: stable swap pool, random prices")]
    #[test_case(PairType::Stable { },vec![("uluna",1_000_000), ("uatom", 1_000_000)], Uint128::new(1_000_000); "swap_and_simulate_swap: stable swap pool, native-native")]
    #[test_case(PairType::Stable { },vec![("uluna",1_000_000), ("uatom", 1_000_000)], Uint128::new(100_000_000); "swap_and_simulate_swap: stable swap pool, high slippage, native-native")]
    #[test_case(PairType::Stable { },vec![("uluna",68_582_147), ("uatom", 3_467_256)], Uint128::new(1_000_000); "swap_and_simulate_swap: stable swap pool, random prices, native-native")]
    #[test_case(PairType::Custom("concentrated".to_string()),vec![("uluna",1_000_000), ("astro", 1_000_000)], Uint128::new(1_000_000); "swap_and_simulate_swap: concentrated pool, native-cw20")]
    #[test_case(PairType::Custom("concentrated".to_string()),vec![("uluna",1_000_000), ("astro", 1_000_000)], Uint128::new(100_000_000); "swap_and_simulate_swap: concentrated pool, high slippage, native-cw20")]
    #[test_case(PairType::Custom("concentrated".to_string()),vec![("uluna",68_582_147), ("astro", 3_467_256)], Uint128::new(1_000_000); "swap_and_simulate_swap: concentrated pool, random prices, native-cw20")]
    #[test_case(PairType::Custom("concentrated".to_string()),vec![("apollo",1_000_000), ("astro", 1_000_000)], Uint128::new(1_000_000); "swap_and_simulate_swap: concentrated pool, cw20-cw20")]
    #[test_case(PairType::Custom("concentrated".to_string()),vec![("apollo",1_000_000), ("astro", 1_000_000)], Uint128::new(100_000_000); "swap_and_simulate_swap: concentrated pool, high slippage, cw20-cw20")]
    #[test_case(PairType::Custom("concentrated".to_string()),vec![("apollo",68_582_147), ("astro", 3_467_256)], Uint128::new(1_000_000); "swap_and_simulate_swap: concentrated pool, random prices, cw20-cw20")]
    #[test_case(PairType::Custom("concentrated".to_string()),vec![("uluna",1_000_000), ("uatom", 1_000_000)], Uint128::new(1_000_000); "swap_and_simulate_swap: concentrated pool, native-native")]
    #[test_case(PairType::Custom("concentrated".to_string()),vec![("uluna",1_000_000), ("uatom", 1_000_000)], Uint128::new(100_000_000); "swap_and_simulate_swap: concentrated pool, high slippage, native-native")]
    #[test_case(PairType::Custom("concentrated".to_string()),vec![("uluna",68_582_147), ("uatom", 3_467_256)], Uint128::new(1_000_000); "swap_and_simulate_swap: concentrated pool, random prices, native-native")]
    fn test_swap_and_simulate_swap(
        pool_type: PairType,
        initial_liquidity: Vec<(&str, u64)>,
        amount: Uint128,
    ) {
        let owned_runner = get_test_runner();
        let runner = owned_runner.as_ref();
        let (accs, _lp_token_addr, _pair_addr, contract_addr, asset_list, _) =
            setup_pool_and_testing_contract(&runner, pool_type, initial_liquidity).unwrap();

        let admin = &accs[0];
        let wasm = Wasm::new(&runner);

        let offer_info = &asset_list.to_vec()[0].info;
        let ask_info = &asset_list.to_vec()[1].info;

        // Simulate swap
        let offer = Asset {
            info: offer_info.clone(),
            amount,
        };
        let simulate_query = QueryMsg::SimulateSwap {
            offer: offer.clone(),
            ask: ask_info.clone(),
        };

        let expected_out = wasm.query(&contract_addr, &simulate_query).unwrap();

        // Swap
        let swap_msg = ExecuteMsg::Swap {
            offer: offer.clone(),
            ask: ask_info.clone(),
            min_out: expected_out,
        };
        let native_coins = match offer.info {
            AssetInfoBase::Native(denom) => {
                vec![Coin {
                    denom,
                    amount: offer.amount,
                }]
            }
            AssetInfoBase::Cw20(cw20_addr) => {
                // Transfer cw20 tokens to the contract
                cw20_transfer(
                    &runner,
                    cw20_addr.to_string(),
                    contract_addr.clone(),
                    offer.amount,
                    admin,
                )
                .unwrap();
                vec![]
            }
        };
        runner
            .execute_cosmos_msgs::<MsgExecuteContractResponse>(
                &[swap_msg.into_cosmos_msg(contract_addr.clone(), native_coins)],
                admin,
            )
            .unwrap();

        // Query offer and ask balances
        let offer_balance = query_asset_balance(&runner, offer_info, &contract_addr);
        let ask_balance = query_asset_balance(&runner, ask_info, &contract_addr);

        // Assert that offer and ask balances are correct
        assert_eq!(ask_balance, expected_out);
        assert_eq!(offer_balance, Uint128::zero());
    }

    #[test_case(vec![(coin(2_000_000_000, "uluna"), 1)], vec![]; "one native incentive one period")]
    #[test_case(vec![(coin(4_000_000_000, "uluna"), 2)], vec![]; "one native incentive two periods")]
    #[test_case(vec![(coin(4_000_000_000, "uluna"), 2), (coin(2_000_000_000, "untrn"), 1)], vec![]; "two native incentive different periods")]
    #[test_case(vec![(coin(4_000_000_000, "uluna"), 2), (coin(2_000_000_000, "untrn"), 1)], vec![(4_000_000_000u128.into(), 2)]; "two native incentive different periods one cw20 incentive")]
    fn test_claim_rewards(
        native_incentives: Vec<(Coin, u64)>,
        cw20_incentives: Vec<(Uint128, u64)>,
    ) -> RunnerResult<()> {
        let pool_type = PairType::Xyk {};
        let initial_liquidity = vec![("uluna", 1_000_000), ("astro", 1_000_000)];

        let owned_runner = get_test_runner();
        let runner = owned_runner.as_ref();
        let (
            accs,
            lp_token_addr,
            _pair_addr,
            testing_contract_addr,
            _asset_list,
            astroport_contracts,
        ) = setup_pool_and_testing_contract(&runner, pool_type, initial_liquidity).unwrap();

        let admin = &accs[0];

        // Increase time to current time. For some reason the start time of the
        // incenitve epochs is hard coded in the astroport incentives contract
        // and the logic doesn't work for earlier timestamps
        let block_time = runner.query_block_time_nanos() / 1_000_000_000;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        runner
            .increase_time((current_time.saturating_sub(block_time)) as u64)
            .unwrap();

        // Setup wasm runner
        let wasm = Wasm::new(&runner);

        // Initialize account with balances of all native incentives
        let incentives_provider = runner
            .init_account(
                &native_incentives
                    .clone()
                    .into_iter()
                    .map(|(coin, _)| coin)
                    .chain(coins(10000000000, "uosmo")) // for gas
                    .collect::<Vec<_>>(),
            )
            .unwrap();

        // Convert native incentives to AstroportAssets
        let mut incentives: Vec<(AstroportAsset, u64)> = native_incentives
            .clone()
            .into_iter()
            .map(|(coin, duration)| (coin.into(), duration))
            .collect();

        // Create Cw20 tokens for each Cw20 incentive, mint incentive amount to
        // incentives_provider and add to incentives
        let cw20_code_id = astroport_contracts.astro_token.code_id;
        for (i, (amount, duration)) in cw20_incentives.iter().enumerate() {
            // Instantiate Cw20 token
            let cw20_addr = wasm
                .instantiate(
                    cw20_code_id,
                    &cw20_base::msg::InstantiateMsg {
                        name: format!("cw20_incentive_{}", i),
                        symbol: "incentive".to_string(),
                        decimals: 6,
                        initial_balances: vec![cw20::Cw20Coin {
                            address: incentives_provider.address(),
                            amount: *amount,
                        }],
                        mint: None,
                        marketing: None,
                    },
                    Some(&admin.address()),
                    Some("incentive"),
                    &[],
                    admin,
                )
                .unwrap()
                .data
                .address;

            // Add Cw20 incentive to incentives
            incentives.push((
                AstroportAsset::cw20(Addr::unchecked(cw20_addr), *amount),
                *duration,
            ));
        }

        // Setup incentives for the pool
        for (incentive, periods) in incentives.clone() {
            // Increase allowance for cw20 incentives and construct funds
            let funds = match incentive.info.clone() {
                astroport_v3::asset::AssetInfo::Token { contract_addr } => {
                    // Increase allowance for incentives contract
                    wasm.execute(
                        contract_addr.as_str(),
                        &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                            spender: astroport_contracts.incentives.address.clone(),
                            amount: incentive.amount,
                            expires: None,
                        },
                        &[],
                        &incentives_provider,
                    )
                    .unwrap();
                    vec![]
                }
                astroport_v3::asset::AssetInfo::NativeToken { denom } => {
                    vec![coin(incentive.amount.u128(), &denom)]
                }
            };
            wasm.execute(
                &astroport_contracts.incentives.address,
                &astroport_v3::incentives::ExecuteMsg::Incentivize {
                    lp_token: lp_token_addr.clone(),
                    schedule: astroport_v3::incentives::InputSchedule {
                        reward: incentive,
                        duration_periods: periods,
                    },
                },
                &funds,
                &incentives_provider,
            )
            .unwrap();
        }

        // Query LP token balance
        let lp_token_balance =
            cw20_balance_query(&runner, lp_token_addr.clone(), admin.address()).unwrap();

        // Send LP tokens to the test contract
        cw20_transfer(
            &runner,
            lp_token_addr.clone(),
            testing_contract_addr.clone(),
            lp_token_balance,
            admin,
        )
        .unwrap();

        // Stake LP tokens
        let _events = stake_all_lp_tokens(
            &runner,
            testing_contract_addr.clone(),
            lp_token_addr.clone(),
            admin,
        )
        .events;

        // Increase time by 1 week
        runner.increase_time(60 * 60 * 24).unwrap();

        // Query incentives contract for admin users pending rewards
        let pending_rewards: Vec<AstroportAsset> = wasm
            .query(
                &astroport_contracts.incentives.address,
                &astroport_v3::incentives::QueryMsg::PendingRewards {
                    lp_token: lp_token_addr.clone(),
                    user: testing_contract_addr.clone(),
                },
            )
            .unwrap();

        // Query pending rewards through CwDex testing contract
        let cw_dex_pending_rewards: AssetList = wasm
            .query(
                &testing_contract_addr,
                &cw_dex_test_contract::msg::QueryMsg::PendingRewards {},
            )
            .unwrap();

        // Assert that both pending rewards queries return the same result
        for asset in pending_rewards.clone() {
            // Convert astroport asset info to asset info
            let asset_info = match asset.info {
                astroport_v3::asset::AssetInfo::Token { contract_addr } => {
                    AssetInfo::Cw20(contract_addr)
                }
                astroport_v3::asset::AssetInfo::NativeToken { denom } => AssetInfo::Native(denom),
            };

            let amount = cw_dex_pending_rewards.find(&asset_info).unwrap().amount;

            assert_eq!(amount, asset.amount);
        }

        // Claim rewards
        wasm.execute(
            &testing_contract_addr,
            &cw_dex_test_contract::msg::AstroportExecuteMsg::ClaimRewards {},
            &[],
            admin,
        )
        .unwrap();

        // Assert that testing contract has correct asset balances
        for reward in cw_dex_pending_rewards.to_vec() {
            let asset_balance = query_asset_balance(&runner, &reward.info, &testing_contract_addr);
            assert_approx_eq!(asset_balance, reward.amount, "0.0001"); // TODO: Why is there a diff here?
        }

        Ok(())
    }

    #[test]
    fn test_get_pool_for_lp_token() {
        let owned_runner = get_test_runner();
        let runner = owned_runner.as_ref();
        let (_accs, lp_token_addr, pair_addr, contract_addr, asset_list, _) =
            setup_pool_and_testing_contract(
                &runner,
                PairType::Xyk {},
                vec![("uluna", 1_000_000), ("uatom", 1_000_000)],
            )
            .unwrap();

        let wasm = Wasm::new(&runner);

        let query = QueryMsg::GetPoolForLpToken {
            lp_token: AssetInfo::Cw20(Addr::unchecked(lp_token_addr.clone())),
        };
        let pool = wasm
            .query::<_, AstroportPool>(&contract_addr, &query)
            .unwrap();

        assert_eq!(pool.lp_token_addr, Addr::unchecked(lp_token_addr));
        assert_eq!(pool.pair_addr, Addr::unchecked(pair_addr));
        assert_eq!(
            pool.pool_assets,
            asset_list
                .into_iter()
                .map(|x| x.info.clone())
                .collect::<Vec<AssetInfo>>()
        );
    }
}
