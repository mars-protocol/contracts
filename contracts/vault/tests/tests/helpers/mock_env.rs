use std::{fmt::Debug, mem::take, str::FromStr};

use anyhow::{bail, Result as AnyResult};
use cosmwasm_std::{
    from_json, testing::MockApi, Addr, Api, BankMsg, BankQuery, Binary, BlockInfo, Coin, CustomMsg,
    CustomQuery, Decimal, Empty, Event, GovMsg, IbcMsg, IbcQuery, MemoryStorage, Querier,
    QueryRequest, Storage, SupplyResponse, Uint128,
};
use cw_multi_test::{
    no_init, App, AppResponse, BankKeeper, BankSudo, BasicApp, BasicAppBuilder, CosmosRouter,
    DistributionKeeper, Executor, FailingModule, StakeKeeper, Stargate, SudoMsg, WasmKeeper,
};
use cw_vault_standard::VaultInfoResponse;
use mars_testing::multitest::modules::token_factory::{CustomApp, TokenFactory};
use mars_vault::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use osmosis_std::types::osmosis::tokenfactory::v1beta1::{
    MsgBurn, MsgBurnResponse, MsgCreateDenom, MsgCreateDenomResponse, MsgMint, MsgMintResponse,
};

use super::contracts::mock_vault_contract;

pub struct AccountToFund {
    pub addr: Addr,
    pub funds: Vec<Coin>,
}

pub struct MockEnv {
    pub app: CustomApp,
    pub vault_contract: Addr,
}

pub struct MockEnvBuilder {
    pub app: CustomApp,
    pub accounts_to_fund: Vec<AccountToFund>,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        let tf_default = TokenFactory::default();
        let app = BasicAppBuilder::new().with_stargate(tf_default).build(no_init);
        MockEnvBuilder {
            app,
            accounts_to_fund: vec![],
        }
    }

    //--------------------------------------------------------------------------------------------------
    // Execute Msgs
    //--------------------------------------------------------------------------------------------------

    pub fn deposit(
        &mut self,
        sender: &Addr,
        amount: Uint128,
        send_funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.vault_contract.clone(),
            &ExecuteMsg::Deposit {
                amount,
                recipient: None,
            },
            send_funds,
        )
    }

    //--------------------------------------------------------------------------------------------------
    // Queries
    //--------------------------------------------------------------------------------------------------

    pub fn query_vault_info(&self, denom: &str) -> VaultInfoResponse {
        self.app.wrap().query_wasm_smart(self.vault_contract.clone(), &QueryMsg::Info {}).unwrap()
    }
}

impl MockEnvBuilder {
    pub fn build(mut self) -> AnyResult<MockEnv> {
        let code_id = self.app.store_code(mock_vault_contract());

        let params_contract = self.app.instantiate_contract(
            code_id,
            Addr::unchecked("owner"),
            &InstantiateMsg {
                base_token: "uusdc".to_string(),
                vault_token_subdenom: "vault".to_string(),
                fund_manager_account_id: "40".to_string(),
                title: None,
                subtitle: None,
                description: None,
            },
            &[],
            "mock-vault-contract",
            None,
        )?;

        self.fund_users();

        Ok(MockEnv {
            app: self.app,
            vault_contract: params_contract,
        })
    }

    fn fund_users(&mut self) {
        for account in &self.accounts_to_fund {
            self.app
                .sudo(SudoMsg::Bank(BankSudo::Mint {
                    to_address: account.addr.to_string(),
                    amount: account.funds.clone(),
                }))
                .unwrap();
        }
    }

    pub fn fund_account(mut self, account: AccountToFund) -> Self {
        self.accounts_to_fund.push(account);
        self
    }
}
