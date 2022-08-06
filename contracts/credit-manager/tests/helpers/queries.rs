use cosmwasm_std::Addr;
use cw_multi_test::{App, AppResponse};
use mock_red_bank::msg::QueryMsg::UserAssetDebt;
use mock_red_bank::msg::UserAssetDebtResponse;
use rover::health::Health;

use rover::msg::query::{ConfigResponse, PositionResponse, QueryMsg};

pub fn get_token_id(res: AppResponse) -> String {
    let attr: Vec<&String> = res
        .events
        .iter()
        .flat_map(|event| &event.attributes)
        .filter(|attr| attr.key == "token_id")
        .map(|attr| &attr.value)
        .collect();

    assert_eq!(attr.len(), 1);
    attr.first().unwrap().to_string()
}

pub fn query_position(
    app: &App,
    manager_contract_addr: &Addr,
    token_id: &String,
) -> PositionResponse {
    app.wrap()
        .query_wasm_smart(
            manager_contract_addr.clone(),
            &QueryMsg::Position {
                token_id: token_id.clone(),
            },
        )
        .unwrap()
}

pub fn query_health(app: &App, manager_contract_addr: &Addr, token_id: &String) -> Health {
    app.wrap()
        .query_wasm_smart(
            manager_contract_addr.clone(),
            &QueryMsg::Health {
                token_id: token_id.clone(),
            },
        )
        .unwrap()
}

pub fn query_config(app: &mut App, contract_addr: &Addr) -> ConfigResponse {
    app.wrap()
        .query_wasm_smart(contract_addr.clone(), &QueryMsg::Config {})
        .unwrap()
}

pub fn query_red_bank_debt(
    app: &mut App,
    credit_manager_addr: &Addr,
    red_bank_addr: &str,
    denom: &str,
) -> UserAssetDebtResponse {
    app.wrap()
        .query_wasm_smart(
            red_bank_addr,
            &UserAssetDebt {
                user_address: credit_manager_addr.into(),
                denom: denom.into(),
            },
        )
        .unwrap()
}
