use std::str::FromStr;

use cosmwasm_std::{Coin, Decimal, StdError, Uint128};
use mars_types::{
    credit_manager::{DebtAmount, Positions},
    health::AccountKind,
    oracle::ActionKind,
    params::{
        AssetParamsUnchecked, AssetParamsUpdate::AddOrUpdate, CmSettings, HlsParamsUnchecked,
        LiquidationBonus, RedBankSettings,
    },
};

use super::helpers::MockEnv;

#[test]
fn uses_liquidation_pricing() {
    let mut mock = MockEnv::new().build().unwrap();

    let umars = "umars";
    mock.set_price(umars, Decimal::one(), ActionKind::Liquidation);

    let update = AddOrUpdate {
        params: AssetParamsUnchecked {
            denom: umars.to_string(),
            credit_manager: CmSettings {
                whitelisted: false,
                hls: Some(HlsParamsUnchecked {
                    max_loan_to_value: Decimal::from_str("0.8").unwrap(),
                    liquidation_threshold: Decimal::from_str("0.9").unwrap(),
                    correlations: vec![],
                }),
            },
            red_bank: RedBankSettings {
                deposit_enabled: false,
                borrow_enabled: false,
            },
            max_loan_to_value: Decimal::from_atomics(4523u128, 4).unwrap(),
            liquidation_threshold: Decimal::from_atomics(5u128, 1).unwrap(),
            liquidation_bonus: LiquidationBonus {
                starting_lb: Decimal::percent(1u64),
                slope: Decimal::from_atomics(2u128, 0).unwrap(),
                min_lb: Decimal::percent(2u64),
                max_lb: Decimal::percent(10u64),
            },
            protocol_liquidation_fee: Decimal::percent(2u64),
            deposit_cap: Default::default(),
        },
    };

    mock.update_asset_params(update);

    let account_id = "123";
    mock.set_positions_response(
        account_id,
        &Positions {
            account_id: account_id.to_string(),
            account_kind: AccountKind::Default,
            deposits: vec![Coin {
                denom: umars.to_string(),
                amount: Uint128::new(30),
            }],
            debts: vec![DebtAmount {
                denom: umars.to_string(),
                shares: Default::default(),
                amount: Uint128::new(2),
            }],
            lends: vec![],
            vaults: vec![],
            staked_astro_lps: vec![],
        },
    );

    // Default pricing should error
    let err: StdError =
        mock.query_health_state(account_id, AccountKind::Default, ActionKind::Default).unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err(
            "Querier contract error: Generic error: Querier contract error: type: cosmwasm_std::math::decimal::Decimal; key: [00, 12, 64, 65, 66, 61, 75, 6C, 74, 5F, 63, 6F, 69, 6E, 5F, 70, 72, 69, 63, 65, 75, 6D, 61, 72, 73] not found".to_string()
        )
    );
    let err: StdError = mock
        .query_health_values(account_id, AccountKind::Default, ActionKind::Default)
        .unwrap_err();
    assert_eq!(
        err,
        StdError::generic_err(
            "Querier contract error: Generic error: Querier contract error: type: cosmwasm_std::math::decimal::Decimal; key: [00, 12, 64, 65, 66, 61, 75, 6C, 74, 5F, 63, 6F, 69, 6E, 5F, 70, 72, 69, 63, 65, 75, 6D, 61, 72, 73] not found".to_string()
        )
    );

    // Liquidation pricing is used and succeeds
    mock.query_health_state(account_id, AccountKind::Default, ActionKind::Liquidation).unwrap();
    mock.query_health_values(account_id, AccountKind::Default, ActionKind::Liquidation).unwrap();
}
