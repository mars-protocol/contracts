use cosmwasm_std::{
    coin, testing::mock_env, CosmosMsg, IbcMsg, IbcTimeout, SubMsg, Timestamp, Uint128,
};
use mars_rewards_collector_base::ContractError;
use mars_rewards_collector_osmosis::entry::execute;
use mars_testing::{mock_env as mock_env_at_height_and_time, mock_info, MockEnvParams};
use mars_types::rewards_collector::ExecuteMsg;

use super::helpers;

#[test]
fn distributing_rewards() {
    let mut deps = helpers::setup_test();

    let env = mock_env_at_height_and_time(MockEnvParams {
        block_height: 10000,
        block_time: Timestamp::from_seconds(17000000),
    });

    // distribute uusdc to safety fund
    let res = execute(
        deps.as_mut(),
        env.clone(),
        mock_info("jake"),
        ExecuteMsg::DistributeRewards {
            denom: "uusdc".to_string(),
            amount: Some(Uint128::new(123)),
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: "channel-69".to_string(),
            to_address: "safety_fund".to_string(),
            amount: coin(123, "uusdc"),
            timeout: IbcTimeout::with_timestamp(Timestamp::from_seconds(17000300))
        }))
    );

    // distribute umars to fee collector
    let res = execute(
        deps.as_mut(),
        env,
        mock_info("jake"),
        ExecuteMsg::DistributeRewards {
            denom: "umars".to_string(),
            amount: None,
        },
    )
    .unwrap();
    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::new(CosmosMsg::Ibc(IbcMsg::Transfer {
            channel_id: "channel-69".to_string(),
            to_address: "fee_collector".to_string(),
            amount: coin(8964, "umars"),
            timeout: IbcTimeout::with_timestamp(Timestamp::from_seconds(17000300))
        }))
    );

    // distribute uatom; should fail
    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::DistributeRewards {
            denom: "uatom".to_string(),
            amount: Some(Uint128::new(123)),
        },
    )
    .unwrap_err();
    assert_eq!(
        err,
        ContractError::AssetNotEnabledForDistribution {
            denom: "uatom".to_string()
        }
    );
}
