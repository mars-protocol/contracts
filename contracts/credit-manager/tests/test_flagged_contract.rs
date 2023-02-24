use cosmwasm_std::{coin, Addr};
use helpers::assert_err;
use mars_rover::{error::ContractError::Unauthorized, msg::execute::Action};

use crate::helpers::MockEnv;

pub mod helpers;

#[test]
fn addresses_in_config_cannot_execute_msgs() {
    let mut mock = MockEnv::new().build().unwrap();
    let config = mock.query_config();
    let vault_addrs = mock
        .query_vault_configs(None, None)
        .iter()
        .map(|v| v.vault.address.clone())
        .collect::<Vec<_>>();

    let banned = vec![
        config.account_nft.unwrap(),
        config.red_bank,
        config.oracle,
        config.swapper,
        config.zapper,
        config.health_contract,
    ]
    .into_iter()
    .chain(vault_addrs)
    .collect::<Vec<String>>();

    for addr_str in banned {
        let user = Addr::unchecked(addr_str);
        let account_id = mock.create_credit_account(&user).unwrap();
        let res = mock.update_credit_account(
            &account_id,
            &user,
            vec![Action::Deposit(coin(0, "uosmo"))],
            &[],
        );
        assert_err(
            res,
            Unauthorized {
                user: user.into(),
                action: "execute actions on rover".to_string(),
            },
        )
    }
}
