use cosmwasm_std::Addr;
use cw_multi_test::Executor;
use mars_testing::multitest::modules::token_factory::CustomApp;
use mars_vault::msg::InstantiateMsg;

use super::helpers::{mock_managed_vault_contract, MockEnv};

#[test]
fn sample_test() {
    let mut mock = MockEnv::new().build().unwrap();
    let _original_config = mock.query_config();

    let _managed_vault_addr = deploy_managed_vault(&mut mock.app);
}

fn deploy_managed_vault(app: &mut CustomApp) -> Addr {
    let contract_code_id = app.store_code(mock_managed_vault_contract());
    app.instantiate_contract(
        contract_code_id,
        Addr::unchecked("vault_contract_owner"),
        &InstantiateMsg {
            base_token: "uusdc".to_string(),
            vault_token_subdenom: "vault".to_string(),
            fund_manager_account_id: "40".to_string(),
            title: None,
            subtitle: None,
            description: None,
        },
        &[],
        "mock-managed-vault",
        None,
    )
    .unwrap()
}
