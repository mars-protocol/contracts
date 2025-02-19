use cosmwasm_std::{coin, Coin, CosmosMsg, Decimal, IbcMsg, IbcTimeout, SubMsg, Timestamp};
use mars_rewards_collector_osmosis::entry::execute;
use mars_testing::{mock_env as mock_env_at_height_and_time, mock_info, MockEnvParams};
use mars_types::rewards_collector::{ExecuteMsg, UpdateConfig};
use test_case::test_case;

use super::helpers;

#[test_case(
    &[coin(1234, "uusdc")],
    "umars".to_string(),
    vec![],
    None;
    "Distribute nothing sends no messages"
)]
#[test_case(
    &[coin(1234, "umars")],
    "umars".to_string(),
    vec![
        SubMsg::new(CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: "channel-69".to_string(),
            to_address: "fee_collector".to_string(),
            amount: coin(1234, "umars"),
            timeout: IbcTimeout::with_timestamp(Timestamp::from_seconds(17000300))
        }))
    ],
    None;
    "Distribute single denom"
)]
#[test_case(
    &[
        coin(1234, "uusdc"),
    ],
    "uusdc".to_string(),
    // uusdc balance in contract = 1234
    // safety fund = 0.25 / (0.1+0.25) = 0.7142857142857143
    // rev share = 0.1 / (0.1+0.25) = 0.28571428571
    // 1234 * 0.7142857142857143 = 881.4 = 881
    // 1234 - 881 = 353
    vec![
        SubMsg::new(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
            to_address: "safety_fund".to_string(),
            amount: vec![coin(881, "uusdc")],
        })),
        SubMsg::new(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
            to_address: "revenue_share".to_string(),
            amount: vec![coin(353, "uusdc")],
        }))
    ],
    None;
    "distribute same denom to safety fund and rev share"
)]
#[test_case(
    &[
        coin(1234, "uusdc"),
    ],
    "uusdc".to_string(),
    // uusdc balance in contract = 1234
    // safety fund = 0.25 / (0.25) = 1
    // 1234 * 1 = 1234
    // 1234 - 1234 = 0
    vec![
        SubMsg::new(CosmosMsg::Bank(cosmwasm_std::BankMsg::Send {
            to_address: "safety_fund".to_string(),
            amount: vec![coin(1234, "uusdc")],
        })),
    ],
    Some(UpdateConfig{
        revenue_share_tax_rate: Some(Decimal::zero()),
        ..Default::default()
    });
    "distribute when rev share is zero"
)]

fn assert_rewards_distribution(
    initial_balances: &[Coin],
    denom_to_distribute: String,
    expected_msgs: Vec<SubMsg>,
    config: Option<UpdateConfig>,
) {
    let mut deps: cosmwasm_std::OwnedDeps<
        cosmwasm_std::MemoryStorage,
        cosmwasm_std::testing::MockApi,
        mars_testing::MarsMockQuerier,
    > = helpers::setup_test();
    deps.querier.set_contract_balances(initial_balances);

    let env = mock_env_at_height_and_time(MockEnvParams {
        block_height: 10000,
        block_time: Timestamp::from_seconds(17000000),
    });

    if let Some(cfg) = config {
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info("owner"),
            ExecuteMsg::UpdateConfig {
                new_cfg: cfg,
            },
        )
        .unwrap();
    }

    // distribute uusdc to safety fund and rev share
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info("jake"),
        ExecuteMsg::DistributeRewards {
            denom: denom_to_distribute,
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), expected_msgs.len());

    assert_eq!(res.messages, expected_msgs);
}
