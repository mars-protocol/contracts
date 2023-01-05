use cosmwasm_std::testing::mock_env;
use cosmwasm_std::Decimal;

use mars_outpost::error::MarsError;
use mars_outpost::oracle::QueryMsg;
use mars_testing::mock_info;

use mars_oracle_osmosis::contract::entry::execute;
use mars_oracle_osmosis::msg::{ExecuteMsg, PriceSourceResponse};
use mars_oracle_osmosis::OsmosisPriceSource;

mod helpers;

#[test]
fn test_remove_price_source_by_non_owner() {
    let mut deps = helpers::setup_test();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("jake"),
        ExecuteMsg::RemovePriceSource {
            denom: "uosmo".to_string(),
        },
    )
    .unwrap_err();
    assert_eq!(err, MarsError::Unauthorized {}.into())
}

#[test]
fn test_removing_price_source() {
    let mut deps = helpers::setup_test();

    helpers::set_price_source(
        deps.as_mut(),
        "uosmo",
        OsmosisPriceSource::Fixed {
            price: Decimal::one(),
        },
    );
    helpers::set_price_source(
        deps.as_mut(),
        "uatom",
        OsmosisPriceSource::Spot {
            pool_id: 1,
        },
    );
    helpers::set_price_source(
        deps.as_mut(),
        "umars",
        OsmosisPriceSource::Spot {
            pool_id: 89,
        },
    );

    // check if there is correct number of entries
    let res: Vec<PriceSourceResponse> = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSources {
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(res.len(), 3);

    // Try to remove non-existing entry
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::RemovePriceSource {
            denom: "ibc-coin".to_string(),
        },
    )
    .unwrap();
    let res: Vec<PriceSourceResponse> = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSources {
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(res.len(), 3);

    // Remove entry
    execute(
        deps.as_mut(),
        mock_env(),
        mock_info("owner"),
        ExecuteMsg::RemovePriceSource {
            denom: "uatom".to_string(),
        },
    )
    .unwrap();
    let res: Vec<PriceSourceResponse> = helpers::query(
        deps.as_ref(),
        QueryMsg::PriceSources {
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(res.len(), 2);
    assert!(!res.iter().any(|ps| &ps.denom == "uatom"))
}
