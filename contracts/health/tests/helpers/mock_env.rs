use anyhow::Result as AnyResult;
use cosmwasm_std::{Addr, Coin, Decimal, Empty, StdResult, Uint128};
use cosmwasm_vault_standard::{
    VaultInfoResponse, VaultStandardExecuteMsg::Deposit, VaultStandardQueryMsg::Info,
};
use cw_multi_test::{App, AppResponse, BankSudo, BasicApp, Executor, SudoMsg};
use mars_mock_credit_manager::msg::ExecuteMsg::{
    SetAllowedCoins, SetPositionsResponse, SetVaultConfig,
};
use mars_mock_oracle::msg::{CoinPrice, ExecuteMsg::ChangePrice};
use mars_mock_red_bank::msg::CoinMarketInfo;
use mars_mock_vault::contract::STARTING_VAULT_SHARES;
use mars_red_bank_types::red_bank::{ExecuteMsg::UpdateAsset, InitOrUpdateAssetParams};
use mars_rover::{
    adapters::vault::VaultUnchecked,
    msg::{
        query::{Positions, VaultInfoResponse as CmVaultConfig},
        QueryMsg::VaultInfo,
    },
};
use mars_rover_health_types::{ConfigResponse, ExecuteMsg::UpdateConfig, HealthResponse, QueryMsg};

use crate::helpers::MockEnvBuilder;

pub struct MockEnv {
    pub app: BasicApp,
    pub deployer: Addr,
    pub health_contract: Addr,
    pub cm_contract: Addr,
    pub vault_contract: Addr,
    pub oracle: Addr,
    pub red_bank: Addr,
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
            red_bank: None,
        }
    }

    pub fn query_health(&self, account_id: &str) -> StdResult<HealthResponse> {
        self.app.wrap().query_wasm_smart(
            self.health_contract.clone(),
            &QueryMsg::Health {
                account_id: account_id.to_string(),
            },
        )
    }

    pub fn query_config(&self) -> ConfigResponse {
        self.app
            .wrap()
            .query_wasm_smart(self.health_contract.clone(), &QueryMsg::Config {})
            .unwrap()
    }

    pub fn query_vault_config(&self, vault: &VaultUnchecked) -> CmVaultConfig {
        self.app
            .wrap()
            .query_wasm_smart(
                self.cm_contract.clone(),
                &VaultInfo {
                    vault: vault.clone(),
                },
            )
            .unwrap()
    }

    pub fn update_config(
        &mut self,
        sender: &Addr,
        credit_manager_addr: &Addr,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.health_contract.clone(),
            &UpdateConfig {
                credit_manager: credit_manager_addr.to_string(),
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

    pub fn set_market(&mut self, denom: &str, market: &CoinMarketInfo) {
        self.app
            .execute_contract(
                self.deployer.clone(),
                self.red_bank.clone(),
                &UpdateAsset {
                    denom: denom.to_string(),
                    params: InitOrUpdateAssetParams {
                        max_loan_to_value: Some(market.max_ltv),
                        liquidation_threshold: Some(market.liquidation_threshold),
                        liquidation_bonus: Some(market.liquidation_bonus),
                        reserve_factor: None,
                        interest_rate_model: None,
                        deposit_enabled: None,
                        borrow_enabled: None,
                        deposit_cap: None,
                    },
                },
                &[],
            )
            .unwrap();
    }

    pub fn set_allowed_coins(&mut self, allowed_coins: &[String]) {
        self.app
            .execute_contract(
                self.deployer.clone(),
                self.cm_contract.clone(),
                &SetAllowedCoins(allowed_coins.to_vec()),
                &[],
            )
            .unwrap();
    }

    pub fn vault_allowed(&mut self, vault: &VaultUnchecked, allowed: bool) {
        let mut config = self.query_vault_config(vault).config;
        config.whitelisted = allowed;

        self.app
            .execute_contract(
                self.deployer.clone(),
                self.cm_contract.clone(),
                &SetVaultConfig {
                    address: self.vault_contract.to_string(),
                    config,
                },
                &[],
            )
            .unwrap();
    }
}
