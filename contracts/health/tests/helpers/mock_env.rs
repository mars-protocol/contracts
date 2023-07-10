use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Coin, Decimal, Empty, StdResult, Uint128};
use cw_multi_test::{App, AppResponse, BankSudo, BasicApp, Executor, SudoMsg};
use cw_vault_standard::{
    VaultInfoResponse, VaultStandardExecuteMsg::Deposit, VaultStandardQueryMsg::Info,
};
use mars_mock_credit_manager::msg::ExecuteMsg::SetPositionsResponse;
use mars_mock_oracle::msg::{CoinPrice, ExecuteMsg::ChangePrice};
use mars_mock_vault::contract::STARTING_VAULT_SHARES;
use mars_params::{
    msg::{
        AssetParamsUpdate,
        ExecuteMsg::{UpdateAssetParams, UpdateVaultConfig},
        QueryMsg as ParamsQueryMsg, VaultConfigUpdate,
    },
    types::vault::VaultConfig,
};
use mars_rover::{adapters::vault::VaultUnchecked, msg::query::Positions};
use mars_rover_health_types::{
    AccountKind, ConfigResponse, ExecuteMsg::UpdateConfig, HealthState, HealthValuesResponse,
    QueryMsg,
};

use crate::helpers::MockEnvBuilder;

pub struct MockEnv {
    pub app: BasicApp,
    pub deployer: Addr,
    pub health_contract: Addr,
    pub cm_contract: Addr,
    pub vault_contract: Addr,
    pub oracle: Addr,
    pub params: Addr,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            deployer: Addr::unchecked("deployer"),
            health_contract: None,
            set_cm_config: true,
            cm_contract: None,
            vault_contract: None,
            oracle: None,
            params: None,
        }
    }

    pub fn query_health_values(
        &self,
        account_id: &str,
        kind: AccountKind,
    ) -> StdResult<HealthValuesResponse> {
        self.app.wrap().query_wasm_smart(
            self.health_contract.clone(),
            &QueryMsg::HealthValues {
                account_id: account_id.to_string(),
                kind,
            },
        )
    }

    pub fn query_health_state(
        &self,
        account_id: &str,
        kind: AccountKind,
    ) -> StdResult<HealthState> {
        self.app.wrap().query_wasm_smart(
            self.health_contract.clone(),
            &QueryMsg::HealthState {
                account_id: account_id.to_string(),
                kind,
            },
        )
    }

    pub fn query_config(&self) -> ConfigResponse {
        self.app
            .wrap()
            .query_wasm_smart(self.health_contract.clone(), &QueryMsg::Config {})
            .unwrap()
    }

    pub fn query_vault_config(&self, vault: &VaultUnchecked) -> VaultConfig {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params.clone(),
                &ParamsQueryMsg::VaultConfig {
                    address: vault.address.to_string(),
                },
            )
            .unwrap()
    }

    pub fn update_config(
        &mut self,
        sender: &Addr,
        credit_manager: String,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.health_contract.clone(),
            &UpdateConfig {
                credit_manager,
            },
            &[],
        )
    }

    pub fn set_positions_response(&mut self, account_id: &str, positions: &Positions) {
        self.app
            .execute_contract(
                self.deployer.clone(),
                self.cm_contract.clone(),
                &SetPositionsResponse {
                    account_id: account_id.to_string(),
                    positions: positions.clone(),
                },
                &[],
            )
            .unwrap();
    }

    // Meant to ensure that the vault issues shares correctly to match the position response
    pub fn deposit_into_vault(&mut self, base_token_amount: Uint128) {
        let info: VaultInfoResponse = self
            .app
            .wrap()
            .query_wasm_smart(self.vault_contract.clone(), &Info::<Empty> {})
            .unwrap();

        let coin_to_deposit = Coin {
            denom: info.base_token.clone(),
            amount: base_token_amount,
        };

        // Seed deployer with vault tokens
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: self.deployer.clone().to_string(),
                amount: vec![coin_to_deposit.clone()],
            }))
            .unwrap();

        // Seed vault contract with vault tokens
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: self.vault_contract.clone().to_string(),
                amount: vec![Coin {
                    denom: info.vault_token,
                    amount: STARTING_VAULT_SHARES,
                }],
            }))
            .unwrap();

        self.app
            .execute_contract(
                self.deployer.clone(),
                self.vault_contract.clone(),
                &Deposit::<Empty> {
                    amount: base_token_amount,
                    recipient: None,
                },
                &[coin_to_deposit],
            )
            .unwrap();
    }

    pub fn set_price(&mut self, denom: &str, price: Decimal) {
        self.app
            .execute_contract(
                self.deployer.clone(),
                self.oracle.clone(),
                &ChangePrice(CoinPrice {
                    denom: denom.to_string(),
                    price,
                }),
                &[],
            )
            .unwrap();
    }

    pub fn update_asset_params(&mut self, update: AssetParamsUpdate) {
        self.app
            .execute_contract(
                self.deployer.clone(),
                self.params.clone(),
                &UpdateAssetParams(update),
                &[],
            )
            .unwrap();
    }

    pub fn update_vault_params(&mut self, update: VaultConfigUpdate) {
        self.app
            .execute_contract(
                self.deployer.clone(),
                self.params.clone(),
                &UpdateVaultConfig(update),
                &[],
            )
            .unwrap();
    }
}
