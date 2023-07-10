use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Empty};
use cw721::OwnerOfResponse;
use cw721_base::{
    Action::{AcceptOwnership, TransferOwnership},
    ExecuteMsg::UpdateOwnership,
    Ownership,
};
use cw_multi_test::{App, AppResponse, BasicApp, Executor};
use mars_account_nft::{
    msg::{ExecuteMsg, ExecuteMsg::UpdateConfig, QueryMsg},
    nft_config::{NftConfigUpdates, UncheckedNftConfig},
};
use mars_mock_rover_health::msg::ExecuteMsg::SetHealthResponse;
use mars_rover_health_types::{AccountKind, HealthValuesResponse};

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
            health_contract: None,
            set_health_contract: true,
        }
    }

    pub fn query_config(&mut self) -> UncheckedNftConfig {
        self.app.wrap().query_wasm_smart(self.nft_contract.clone(), &QueryMsg::Config {}).unwrap()
    }

    pub fn query_ownership(&mut self) -> Ownership<Addr> {
        self.app
            .wrap()
            .query_wasm_smart(self.nft_contract.clone(), &QueryMsg::Ownership {})
            .unwrap()
    }

    pub fn query_next_id(&mut self) -> String {
        self.app.wrap().query_wasm_smart(self.nft_contract.clone(), &QueryMsg::NextId {}).unwrap()
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

    pub fn assert_next_id(&mut self, expected_next_id: &str) {
        let actual_next_id = self.query_next_id();
        assert_eq!(expected_next_id, &actual_next_id)
    }

    pub fn set_health_response(
        &mut self,
        sender: &Addr,
        account_id: &str,
        response: &HealthValuesResponse,
    ) -> AppResponse {
        let config = self.query_config();

        self.app
            .execute_contract(
                sender.clone(),
                Addr::unchecked(config.health_contract_addr.unwrap()),
                &SetHealthResponse {
                    account_id: account_id.to_string(),
                    kind: AccountKind::Default,
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
            &ExecuteMsg::Mint {
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
            &ExecuteMsg::Burn {
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
        self.app.execute_contract(
            sender.clone(),
            self.nft_contract.clone(),
            &UpdateOwnership::<Empty, Empty>(TransferOwnership {
                new_owner: proposed_new_minter.to_string(),
                expiry: None,
            }),
            &[],
        )
    }

    pub fn accept_proposed_minter(&mut self, sender: &Addr) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.nft_contract.clone(),
            &UpdateOwnership::<Empty, Empty>(AcceptOwnership),
            &[],
        )
    }

    pub fn update_config(
        &mut self,
        sender: &Addr,
        updates: &NftConfigUpdates,
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
