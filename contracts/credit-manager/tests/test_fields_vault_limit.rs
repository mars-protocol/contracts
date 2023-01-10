use cosmwasm_std::{coin, Addr, Decimal};
use mars_rover::{
    error::ContractError,
    msg::execute::Action::{Deposit, EnterVault},
};

use crate::helpers::{
    assert_err, lp_token_info, unlocked_vault_info, AccountToFund, CoinInfo, MockEnv, VaultTestInfo,
};

pub mod helpers;

#[test]
fn test_can_only_have_a_single_vault_position() {
    let lp_token = lp_token_info();
    let leverage_vault = unlocked_vault_info();

    let degen_vault_token = CoinInfo {
        denom: "udegen452".to_string(),
        price: Decimal::from_atomics(121u128, 3).unwrap(),
        max_ltv: Decimal::from_atomics(4u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(5u128, 1).unwrap(),
        liquidation_bonus: Decimal::from_atomics(2u128, 1).unwrap(),
    };
    let degen_vault = VaultTestInfo {
        vault_token_denom: "udegen".to_string(),
        lockup: None,
        base_token_denom: degen_vault_token.denom.clone(),
        deposit_cap: coin(10_000_000, "uusdc"),
        max_ltv: Decimal::from_atomics(6u128, 1).unwrap(),
        liquidation_threshold: Decimal::from_atomics(7u128, 1).unwrap(),
    };

    let user = Addr::unchecked("user");
    let mut mock = MockEnv::new()
        .allowed_coins(&[lp_token.clone(), degen_vault_token.clone()])
        .vault_configs(&[leverage_vault.clone(), degen_vault.clone()])
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: vec![lp_token.to_coin(300), degen_vault_token.to_coin(300)],
        })
        .build()
        .unwrap();

    let lev_vault = mock.get_vault(&leverage_vault);
    let degen_vault = mock.get_vault(&degen_vault);
    let account_id = mock.create_credit_account(&user).unwrap();

    let res = mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Deposit(lp_token.to_coin(200)),
            EnterVault {
                vault: lev_vault,
                coin: lp_token.to_action_coin(200),
            },
            Deposit(degen_vault_token.to_coin(200)),
            EnterVault {
                vault: degen_vault,
                coin: degen_vault_token.to_action_coin(200),
            },
        ],
        &[lp_token.to_coin(200), degen_vault_token.to_coin(200)],
    );
    assert_err(res, ContractError::OnlyOneVaultPositionAllowed);
}
