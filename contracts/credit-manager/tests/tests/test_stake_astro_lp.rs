use cosmwasm_std::{coins, Addr, Coin, Uint128};
use mars_credit_manager::error::ContractError;
use mars_testing::multitest::helpers::AccountToFund;
use mars_types::credit_manager::{Action, ActionAmount, ActionCoin};

use super::helpers::MockEnv;

#[test]
fn stake_claims_rewards() {
    let user = Addr::unchecked("user");
    let lp_denom = "factory12345";

    let mut mock = MockEnv::new()
        .fund_account(AccountToFund {
            addr: user.clone(),
            funds: coins(200, lp_denom),
        })
        .build()
        .unwrap();

    let account_id: String = mock.create_credit_account(&user).unwrap();

    // Query staked LP, verify is 0
    let staked_lp_response = mock.query_staked_lp_position(&account_id, lp_denom);
    assert!(staked_lp_response.lp_coin.amount.is_zero());

    let lp_coin = Coin {
        denom: lp_denom.to_string(),
        amount: Uint128::new(200),
    };

    // should fail when trying to stake 0 amount
    let err: ContractError = mock
        .update_credit_account(
            &account_id,
            &user,
            vec![
                Action::Deposit(lp_coin.clone()),
                Action::StakeAstroLp {
                    lp_token: ActionCoin {
                        denom: lp_denom.to_string(),
                        amount: ActionAmount::Exact(Uint128::new(0)),
                    },
                },
            ],
            &[lp_coin.clone()],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert_eq!(err, ContractError::NoAmount);

    // stake
    mock.update_credit_account(
        &account_id,
        &user,
        vec![
            Action::Deposit(lp_coin.clone()),
            Action::StakeAstroLp {
                lp_token: ActionCoin {
                    denom: lp_denom.to_string(),
                    amount: ActionAmount::Exact(Uint128::new(100)),
                },
            },
        ],
        &[lp_coin.clone()],
    )
    .unwrap();

    // Add rewards
    let reward = Coin {
        denom: "uastro".to_string(),
        amount: Uint128::new(10000000),
    };

    mock.add_astro_incentive_reward(&account_id, lp_denom, reward.clone());

    let unclaimed = mock.query_staked_astro_lp_rewards(&account_id, lp_denom);
    assert_eq!(unclaimed.len(), 1);

    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.staked_astro_lps.len(), 1);
    // 100 staked LP,  100 unstaked lp
    assert_eq!(positions.staked_astro_lps[0].denom, lp_coin.denom);
    assert_eq!(positions.staked_astro_lps[0].amount, Uint128::new(100));
    assert_eq!(positions.deposits.len(), 1);
    assert_eq!(positions.deposits[0].denom, lp_coin.denom);
    assert_eq!(positions.deposits[0].amount, Uint128::new(100));

    // Assert correct lp balance in contract
    let lp_coin = mock.query_balance(&mock.rover, lp_denom);
    assert_eq!(positions.deposits[0].amount, lp_coin.amount);

    // stake again
    mock.update_credit_account(
        &account_id,
        &user,
        vec![Action::StakeAstroLp {
            lp_token: ActionCoin {
                denom: lp_denom.to_string(),
                amount: ActionAmount::AccountBalance,
            },
        }],
        &[],
    )
    .unwrap();

    let positions = mock.query_positions(&account_id);
    assert_eq!(positions.staked_astro_lps.len(), 1);
    assert_eq!(positions.staked_astro_lps[0].denom, lp_denom);
    assert_eq!(positions.staked_astro_lps[0].amount, Uint128::new(200));
    assert_eq!(positions.deposits.len(), 1);
    assert_eq!(positions.deposits[0].denom, reward.denom);
    assert_eq!(positions.deposits[0].amount, reward.amount);

    // Assert correct lp balance in contract
    let lp_coin = mock.query_balance(&mock.rover, lp_denom);
    assert!(lp_coin.amount.is_zero());
}
