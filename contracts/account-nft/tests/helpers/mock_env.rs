use anyhow::Result as AnyResult;
use cosmwasm_std::Addr;
use cw721::OwnerOfResponse;
use cw_multi_test::{App, AppResponse, BasicApp, Executor};

use mars_account_nft::config::{ConfigUpdates, UncheckedConfig};
use mars_account_nft::msg::ExecuteMsg::{AcceptMinterRole, UpdateConfig};
use mars_account_nft::msg::{ExecuteMsg as ExtendedExecuteMsg, QueryMsg};
use mars_mock_credit_manager::msg::ExecuteMsg::SetHealthResponse;
use mars_rover::msg::query::HealthResponse;

use crate::helpers::MockEnvBuilder;

pub struct MockEnv {
    pub app: BasicApp,
    pub minter: Addr,
    pub nft_contract: Addr,
    pub deployer: Addr,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            minter: None,
            deployer: Addr::unchecked("deployer"),
            nft_contract: None,
        }
    }

    pub fn query_config(&mut self) -> UncheckedConfig {
        self.app
            .wrap()
            .query_wasm_smart(self.nft_contract.clone(), &QueryMsg::Config {})
            .unwrap()
    }

    pub fn query_next_id(&mut self) -> u64 {
        self.app
            .wrap()
            .query_wasm_smart(self.nft_contract.clone(), &QueryMsg::NextId {})
            .unwrap()
    }

    // Double checking ownership by querying NFT account-nft for correct owner
    pub fn assert_owner_is_correct(&mut self, user: &Addr, token_id: &str) {
        let owner_res: OwnerOfResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.nft_contract.clone(),
                &QueryMsg::OwnerOf {
                    token_id: token_id.to_string(),
                    include_expired: None,
                },
            )
            .unwrap();
        assert_eq!(user.to_string(), owner_res.owner)
    }

    pub fn set_health_response(
        &mut self,
        sender: &Addr,
        account_id: &str,
        response: &HealthResponse,
    ) -> AppResponse {
        self.app
            .execute_contract(
                sender.clone(),
                self.minter.clone(),
                &SetHealthResponse {
                    account_id: account_id.to_string(),
                    response: response.clone(),
                },
                &[],
            )
            .unwrap()
    }

    pub fn mint(&mut self, token_owner: &Addr) -> AnyResult<String> {
        let res = self.app.execute_contract(
            self.minter.clone(),
            self.nft_contract.clone(),
            &ExtendedExecuteMsg::Mint {
                user: token_owner.into(),
            },
            &[],
        )?;

        let attr: Vec<&str> = res
            .events
            .iter()
            .flat_map(|event| &event.attributes)
            .filter(|attr| attr.key == "token_id")
            .map(|attr| attr.value.as_str())
            .collect();

        assert_eq!(attr.len(), 1);
        Ok(attr.first().unwrap().to_string())
    }

    pub fn burn(&mut self, sender: &Addr, token_id: &str) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.nft_contract.clone(),
            &ExtendedExecuteMsg::Burn {
                token_id: token_id.to_string(),
            },
            &[],
        )
    }

    pub fn propose_new_minter(
        &mut self,
        sender: &Addr,
        proposed_new_minter: &Addr,
    ) -> AnyResult<AppResponse> {
        self.update_config(
            sender,
            &ConfigUpdates {
                max_value_for_burn: None,
                proposed_new_minter: Some(proposed_new_minter.to_string()),
            },
        )
    }

    pub fn accept_proposed_minter(&mut self, sender: &Addr) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.nft_contract.clone(),
            &AcceptMinterRole {},
            &[],
        )
    }

    pub fn update_config(
        &mut self,
        sender: &Addr,
        updates: &ConfigUpdates,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.nft_contract.clone(),
            &UpdateConfig {
                updates: updates.clone(),
            },
            &[],
        )
    }
}
