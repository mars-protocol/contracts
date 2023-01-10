use cosmwasm_std::testing::MockApi;

use crate::helpers::{assert_contents_equal, build_mock_vaults, MockEnv};

pub mod helpers;

#[test]
fn test_pagination_on_vault_configs_query_works() {
    let vault_configs = build_mock_vaults(32);
    let mock = MockEnv::new().vault_configs(&vault_configs).build().unwrap();

    let vaults_res = mock.query_vault_configs(None, Some(58_u32));

    // Assert maximum is observed
    assert_eq!(vaults_res.len(), 30);

    let vaults_res = mock.query_vault_configs(None, Some(2_u32));

    // Assert limit request is observed
    assert_eq!(vaults_res.len(), 2);

    let vaults_res_a = mock.query_vault_configs(None, None);
    let vaults_res_b =
        mock.query_vault_configs(Some(vaults_res_a.last().unwrap().vault.clone()), None);
    let vaults_res_c =
        mock.query_vault_configs(Some(vaults_res_b.last().unwrap().vault.clone()), None);
    let vaults_res_d =
        mock.query_vault_configs(Some(vaults_res_c.last().unwrap().vault.clone()), None);

    // Assert default is observed
    assert_eq!(vaults_res_a.len(), 10);
    assert_eq!(vaults_res_b.len(), 10);
    assert_eq!(vaults_res_c.len(), 10);

    assert_eq!(vaults_res_d.len(), 2);

    let combined = vaults_res_a
        .iter()
        .cloned()
        .chain(vaults_res_b.iter().cloned())
        .chain(vaults_res_c.iter().cloned())
        .chain(vaults_res_d.iter().cloned())
        .map(|v| v.vault.check(&MockApi::default()).unwrap())
        .map(|v| v.query_info(&mock.app.wrap()).unwrap())
        .map(|info| info.vault_token)
        .collect::<Vec<_>>();

    assert_eq!(combined.len(), vault_configs.len());

    assert_contents_equal(
        &vault_configs.iter().map(|v| v.vault_token_denom.clone()).collect::<Vec<_>>(),
        &combined,
    )
}
