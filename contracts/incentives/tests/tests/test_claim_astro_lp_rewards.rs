use astroport::asset::AssetInfo;
use cosmwasm_std::{
    testing::{mock_env, mock_info, MockApi},
    Addr, Coin, Deps, DepsMut, Env, MemoryStorage, OwnedDeps, Response, Uint128,
};
use cw_it::astroport::astroport_v3::asset::{Asset, AssetInfo as AstroAssetInfo};
use mars_incentives::{contract::execute, query, state::TOTAL_LP_DEPOSITS, ContractError};
use mars_incentives::contract::query;
use mars_testing::{assert_eq_vec, MarsMockQuerier};
use mars_types::incentives::{ExecuteMsg, QueryMsg};

use crate::tests::helpers::th_setup;

fn set_pending_astro_rewards(
    deps: &mut OwnedDeps<MemoryStorage, MockApi, MarsMockQuerier>,
    lp_denom: &str,
    mars_incentives_contract: &str,
    rewards: Vec<Asset>,
) {
    deps.querier.set_unclaimed_astroport_lp_rewards(lp_denom, mars_incentives_contract, rewards);
}
fn deposit_for_user(
    deps: &mut OwnedDeps<MemoryStorage, MockApi, MarsMockQuerier>,
    env: Env,
    sender: &str,
    account_id: String,
    lp_coin: Coin,
) -> Result<Response, ContractError> {
    let info = mock_info(sender, &[lp_coin.clone()]);
    let msg = ExecuteMsg::StakeAstroLp {
        account_id,
        lp_coin,
    };

    execute(deps.as_mut(), env, info, msg)
}

fn claim_for_user(
    deps: &mut OwnedDeps<MemoryStorage, MockApi, MarsMockQuerier>,
    env: Env,
    sender: &str,
    account_id: String,
    lp_denom: String,
) -> Result<Response, ContractError> {
    let info = mock_info(sender, &[]);
    let msg = ExecuteMsg::ClaimAstroLpRewards {
        account_id,
        lp_denom,
    };

    execute(deps.as_mut(), env, info, msg)
}

fn unstake_for_user(
    deps: &mut OwnedDeps<MemoryStorage, MockApi, MarsMockQuerier>,
    env: Env,
    sender: &str,
    account_id: String,
    lp_coin: Coin,
) -> Result<Response, ContractError> {
    let info = mock_info(
        sender,
        &[]
    );
    let msg = ExecuteMsg::UnstakeAstroLp {
        account_id,
        lp_coin
    };

    execute(deps.as_mut(), env, info, msg)
}

fn assert_user_rewards(
    deps: Deps,
    env: Env,
    astroport_incentives_addr: Addr,
    user_id: &str,
    lp_coin: Coin,
    rewards: Vec<Coin>,
) {
    let actual_rewards = query::query_lp_rewards_for_position(
        deps,
        &env,
        &astroport_incentives_addr,
        user_id,
        &lp_coin,
    )
    .unwrap();
    assert_eq_vec(rewards, actual_rewards);
}

#[test]
fn lp_lifecycle() {
    // SETUP
    let env = mock_env();
    let mut deps: OwnedDeps<MemoryStorage, MockApi, MarsMockQuerier> = th_setup();

    // users
    let user_a_id = "1";
    let user_b_id = "2";

    let credit_manager = Addr::unchecked("credit_manager");
    let astroport_incentives_addr = Addr::unchecked("astroport_incentives");
    deps.querier.set_astroport_incentives_address(astroport_incentives_addr.clone());

    let lp_denom = "uusd/ubtc";
    let unclaimed_rewards = vec![Asset::native("ibc/reward_1", 100u128)];

    let default_lp_coin = Coin {
        denom: lp_denom.to_string(),
        amount: Uint128::new(100u128),
    };

    // State:
    // - LP in incentives = 0
    // - Rewards available = 0
    assert_eq!(TOTAL_LP_DEPOSITS.may_load(&deps.storage, lp_denom).unwrap(), None);
    let rewards = query::query_unclaimed_astroport_rewards(
        deps.as_ref(),
        &env.contract.address.to_string(),
        &astroport_incentives_addr.to_string(),
        lp_denom,
    )
    .unwrap();
    assert_eq!(rewards.is_empty(), true);
    let mars_incentives_contract = &env.contract.address.to_string();

    // Deposit for user a
    let res = deposit_for_user(
        &mut deps,
        env.clone(),
        credit_manager.as_str(),
        user_a_id.to_string(),
        Coin::new(100u128, lp_denom),
    )
    .unwrap();

    // State:
    // - LP in incentives = 100
    // - Rewards available = 0
    assert_eq!(
        TOTAL_LP_DEPOSITS.may_load(&deps.storage, lp_denom).unwrap(),
        Some(Uint128::new(100u128))
    );

    set_pending_astro_rewards(
        &mut deps,
        lp_denom,
        mars_incentives_contract,
        unclaimed_rewards.clone(),
    );

    // State:
    // - LP in incentives = 100
    // - Rewards available for user_1 = 100

    assert_user_rewards(
        deps.as_ref(),
        env.clone(),
        astroport_incentives_addr.clone(),
        user_a_id,
        default_lp_coin.clone(),
        unclaimed_rewards.iter().map(|asset| asset.as_coin().unwrap()).collect(),
    );
    // deposit new user
    let res = deposit_for_user(
        &mut deps,
        env.clone(),
        credit_manager.as_str(),
        user_b_id.to_string(),
        Coin::new(100u128, lp_denom),
    )
    .unwrap();

    set_pending_astro_rewards(
        &mut deps,
        lp_denom,
        mars_incentives_contract,
        // Clear pending rewards
        vec![],
    );

    // State:
    // - LP in incentives = 200
    // - Rewards available for user_1 = 100
    // - Rewards available for user_2 = 0
    assert_user_rewards(
        deps.as_ref(),
        env.clone(),
        astroport_incentives_addr.clone(),
        user_a_id,
        default_lp_coin.clone(),
        unclaimed_rewards.iter().map(|asset| asset.as_coin().unwrap()).collect(),
    );
    // User b
    assert_user_rewards(
        deps.as_ref(),
        env.clone(),
        astroport_incentives_addr.clone(),
        user_b_id,
        default_lp_coin.clone(),
        vec![],
    );

    set_pending_astro_rewards(
        &mut deps,
        lp_denom,
        mars_incentives_contract,
        // Clear pending rewards
        unclaimed_rewards.clone(),
    );
    // State:
    // - LP in incentives = 200
    // - Rewards available for user_1 = 150
    // - Rewards available for user_2 = 50

    assert_user_rewards(
        deps.as_ref(),
        env.clone(),
        astroport_incentives_addr.clone(),
        user_a_id,
        default_lp_coin.clone(),
        vec![Coin {
            denom: "ibc/reward_1".to_string(),
            amount: Uint128::new(150u128),
        }],
    );

    // User b
    assert_user_rewards(
        deps.as_ref(),
        env.clone(),
        astroport_incentives_addr.clone(),
        user_b_id,
        default_lp_coin.clone(),
        vec![Coin {
            denom: "ibc/reward_1".to_string(),
            amount: Uint128::new(50u128),
        }],
    );

    // claim rewards, set as null
    let claim_res =
        claim_for_user(
            &mut deps,
            env.clone(),
            credit_manager.as_str(),
            user_a_id.to_string(),
            lp_denom.to_string()
        ).unwrap();

    set_pending_astro_rewards(
        &mut deps,
        lp_denom,
        mars_incentives_contract,
        // Clear pending rewards
        vec![],
    );

    // State:
    // - LP in incentives = 200
    // - Rewards available for user_1 = 0
    // - Rewards available for user_2 = 50
    assert_user_rewards(
        deps.as_ref(),
        env.clone(),
        astroport_incentives_addr.clone(),
        user_a_id,
        default_lp_coin.clone(),
        vec![],
    );

    // User b
    assert_user_rewards(
        deps.as_ref(),
        env.clone(),
        astroport_incentives_addr.clone(),
        user_b_id,
        default_lp_coin.clone(),
        vec![Coin {
            denom: "ibc/reward_1".to_string(),
            amount: Uint128::new(50u128),
        }],
    );

    // Add new unclaimed reward
    set_pending_astro_rewards(
        &mut deps,
        lp_denom,
        mars_incentives_contract,
        unclaimed_rewards.clone(),
    );

    // State:
    // - LP in incentives = 200
    // - Rewards available for user_1 = 50
    // - Rewards available for user_2 = 100
    assert_user_rewards(
        deps.as_ref(),
        env.clone(),
        astroport_incentives_addr.clone(),
        user_a_id,
        default_lp_coin.clone(),
        vec![Coin {
            denom: "ibc/reward_1".to_string(),
            amount: Uint128::new(50u128),
        }],
    );

    // User b
    assert_user_rewards(
        deps.as_ref(),
        env.clone(),
        astroport_incentives_addr.clone(),
        user_b_id,
        default_lp_coin.clone(),
        vec![Coin {
            denom: "ibc/reward_1".to_string(),
            amount: Uint128::new(100u128),
        }],
    );

    // test double stake
    deposit_for_user(&mut deps, env.clone(), credit_manager.as_str(), user_b_id.to_string(), default_lp_coin.clone())
        .unwrap();

    set_pending_astro_rewards(
        &mut deps,
        lp_denom,
        mars_incentives_contract,
        // Clear pending rewards
        vec![],
    );

    // State:
    // - LP in incentives = 300 (user_a 100, user_b 200)
    // - Rewards available for user_1 = 50
    // - Rewards available for user_2 = 0
    assert_user_rewards(
        deps.as_ref(),
        env.clone(),
        astroport_incentives_addr.clone(),
        user_a_id,
        default_lp_coin.clone(),
        vec![Coin {
            denom: "ibc/reward_1".to_string(),
            amount: Uint128::new(50u128),
        }],
    );

    // User b
    assert_user_rewards(
        deps.as_ref(),
        env.clone(),
        astroport_incentives_addr.clone(),
        user_b_id,
        default_lp_coin.clone(),
        vec![],
    );

    unstake_for_user(
        &mut deps,
        env.clone(),
        credit_manager.as_str(),
        user_a_id.to_string(),
        Coin {
            denom: lp_denom.to_string(),
            amount: Uint128::new(100u128),
        }
    ).unwrap();

    // State:
    // - LP in incentives = 300 (user_a 100, user_b 200)
    // - Rewards available for user_1 = 50
    // - Rewards available for user_2 = 0
    assert_user_rewards(
        deps.as_ref(),
        env.clone(),
        astroport_incentives_addr.clone(),
        user_a_id,
        default_lp_coin.clone(),
        vec![],
    );


}

#[test]
fn assert_only_credit_manager() {
    // SETUP
    let env = mock_env();
    let mut deps: OwnedDeps<MemoryStorage, MockApi, MarsMockQuerier> = th_setup();

    // users
    let user_a_id = "1";

    let astroport_incentives_addr = Addr::unchecked("astroport_incentives");
    deps.querier.set_astroport_incentives_address(astroport_incentives_addr.clone());

    let lp_denom = "uusd/ubtc";

    deposit_for_user(
        &mut deps,
        env.clone(),
        "not_credit_manager",
        user_a_id.to_string(),
        Coin::new(100u128, lp_denom),
    ).expect_err("Unauthorized");

    claim_for_user(
        &mut deps,
        env.clone(),
        "not_credit_manager",
        user_a_id.to_string(),
        lp_denom.to_string(),
    ).expect_err("Unauthorized");

    unstake_for_user(
        &mut deps,
        env.clone(),
        "not_credit_manager",
        user_a_id.to_string(),
        Coin::new(100u128, lp_denom),
    ).expect_err("Unauthorized");
}
