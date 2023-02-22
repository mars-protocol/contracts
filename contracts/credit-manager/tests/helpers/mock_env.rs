use std::mem::take;

use anyhow::Result as AnyResult;
use cosmwasm_std::{coins, testing::MockApi, Addr, Coin, Decimal, StdResult, Uint128};
use cosmwasm_vault_standard::{
    extensions::lockup::{LockupQueryMsg, UnlockingPosition},
    msg::{ExtensionQueryMsg, VaultStandardQueryMsg::VaultExtension},
};
use cw_multi_test::{App, AppResponse, BankSudo, BasicApp, Executor, SudoMsg};
use mars_account_nft::{
    msg::{
        ExecuteMsg as NftExecuteMsg, InstantiateMsg as NftInstantiateMsg, QueryMsg as NftQueryMsg,
    },
    nft_config::{NftConfigUpdates, UncheckedNftConfig},
};
use mars_mock_oracle::msg::{
    CoinPrice, ExecuteMsg as OracleExecuteMsg, InstantiateMsg as OracleInstantiateMsg,
};
use mars_mock_red_bank::msg::{CoinMarketInfo, InstantiateMsg as RedBankInstantiateMsg};
use mars_mock_vault::{
    contract::DEFAULT_VAULT_TOKEN_PREFUND, msg::InstantiateMsg as VaultInstantiateMsg,
};
use mars_owner::OwnerUpdate;
use mars_red_bank_types::red_bank::{
    QueryMsg::{UserCollateral, UserDebt},
    UserCollateralResponse, UserDebtResponse,
};
use mars_rover::{
    adapters::{
        health::HealthContract,
        oracle::{Oracle, OracleBase, OracleUnchecked},
        red_bank::RedBankBase,
        swap::{
            EstimateExactInSwapResponse, InstantiateMsg as SwapperInstantiateMsg,
            QueryMsg::EstimateExactInSwap, Swapper, SwapperBase,
        },
        vault::{
            VaultBase, VaultConfig, VaultPosition, VaultPositionValue as VPositionValue,
            VaultUnchecked,
        },
        zapper::{Zapper, ZapperBase},
    },
    msg::{
        execute::{Action, CallbackMsg},
        instantiate::{ConfigUpdates, VaultInstantiateConfig},
        query::{
            CoinBalanceResponseItem, ConfigResponse, DebtShares, LentShares, Positions,
            SharesResponseItem, VaultInfoResponse as RoverVaultInfoResponse,
            VaultPositionResponseItem, VaultWithBalance,
        },
        zapper::{
            InstantiateMsg as ZapperInstantiateMsg, LpConfig, QueryMsg::EstimateProvideLiquidity,
        },
        ExecuteMsg, InstantiateMsg, QueryMsg,
        QueryMsg::VaultPositionValue,
    },
};
use mars_rover_health_types::{
    ExecuteMsg::UpdateConfig, HealthResponse, InstantiateMsg as HealthInstantiateMsg,
    QueryMsg::Health,
};

use crate::helpers::{
    lp_token_info, mock_account_nft_contract, mock_health_contract, mock_oracle_contract,
    mock_red_bank_contract, mock_rover_contract, mock_swapper_contract, mock_vault_contract,
    mock_zapper_contract, AccountToFund, CoinInfo, VaultTestInfo,
};

pub const DEFAULT_RED_BANK_COIN_BALANCE: Uint128 = Uint128::new(1_000_000);

pub struct MockEnv {
    pub app: BasicApp,
    pub rover: Addr,
    pub mars_oracle: Addr,
    pub health_contract: HealthContract,
}

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub owner: Option<Addr>,
    pub vault_configs: Option<Vec<VaultTestInfo>>,
    pub pre_deployed_vaults: Option<Vec<VaultInstantiateConfig>>,
    pub allowed_coins: Option<Vec<CoinInfo>>,
    pub oracle: Option<Oracle>,
    pub red_bank: Option<RedBankBase<Addr>>,
    pub deploy_nft_contract: bool,
    pub set_nft_contract_minter: bool,
    pub accounts_to_fund: Vec<AccountToFund>,
    pub max_close_factor: Option<Decimal>,
    pub max_unlocking_positions: Option<Uint128>,
    pub health_contract: Option<HealthContract>,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            owner: None,
            vault_configs: None,
            pre_deployed_vaults: None,
            allowed_coins: None,
            oracle: None,
            red_bank: None,
            deploy_nft_contract: true,
            set_nft_contract_minter: true,
            accounts_to_fund: vec![],
            max_close_factor: None,
            max_unlocking_positions: None,
            health_contract: None,
        }
    }

    //--------------------------------------------------------------------------------------------------
    // Execute Msgs
    //--------------------------------------------------------------------------------------------------

    pub fn update_credit_account(
        &mut self,
        account_id: &str,
        sender: &Addr,
        actions: Vec<Action>,
        send_funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateCreditAccount {
                account_id: account_id.to_string(),
                actions,
            },
            send_funds,
        )
    }

    pub fn invoke_callback(
        &mut self,
        sender: &Addr,
        callback: CallbackMsg,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::Callback(callback),
            &[],
        )
    }

    pub fn update_config(
        &mut self,
        sender: &Addr,
        updates: ConfigUpdates,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateConfig {
                updates,
            },
            &[],
        )
    }

    pub fn update_nft_config(
        &mut self,
        sender: &Addr,
        updates: NftConfigUpdates,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateNftConfig {
                updates,
            },
            &[],
        )
    }

    pub fn deploy_new_nft_contract(&mut self) -> AnyResult<Addr> {
        let nft_minter = Addr::unchecked("original_nft_minter");
        let nft_contract = deploy_nft_contract(&mut self.app, &nft_minter);
        propose_new_nft_minter(
            &mut self.app,
            nft_contract.clone(),
            &nft_minter.clone(),
            &self.rover.clone(),
        );
        Ok(nft_contract)
    }

    pub fn create_credit_account(&mut self, sender: &Addr) -> AnyResult<String> {
        let res = self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::CreateCreditAccount {},
            &[],
        )?;
        Ok(self.get_account_id(res))
    }

    fn get_account_id(&mut self, res: AppResponse) -> String {
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

    pub fn price_change(&mut self, coin: CoinPrice) {
        self.app
            .execute_contract(
                Addr::unchecked("anyone"),
                self.mars_oracle.clone(),
                &OracleExecuteMsg::ChangePrice(coin),
                &[],
            )
            .unwrap();
    }

    pub fn execute_callback(&mut self, sender: &Addr, msg: CallbackMsg) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::Callback(msg),
            &[],
        )
    }

    pub fn update_owner(&mut self, sender: &Addr, update: OwnerUpdate) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateOwner(update),
            &[],
        )
    }

    //--------------------------------------------------------------------------------------------------
    // Queries
    //--------------------------------------------------------------------------------------------------

    pub fn query_positions(&self, account_id: &str) -> Positions {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::Positions {
                    account_id: account_id.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_health(&self, account_id: &str) -> HealthResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.health_contract.clone().address(),
                &Health {
                    account_id: account_id.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_balance(&self, addr: &Addr, denom: &str) -> Coin {
        self.app.wrap().query_balance(addr.clone(), denom).unwrap()
    }

    pub fn query_config(&self) -> ConfigResponse {
        self.app.wrap().query_wasm_smart(self.rover.clone(), &QueryMsg::Config {}).unwrap()
    }

    pub fn query_nft_config(&self) -> UncheckedNftConfig {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(config.account_nft.unwrap(), &NftQueryMsg::Config {})
            .unwrap()
    }

    pub fn query_vault_config(&self, vault: &VaultUnchecked) -> StdResult<RoverVaultInfoResponse> {
        self.app.wrap().query_wasm_smart(
            self.rover.clone(),
            &QueryMsg::VaultInfo {
                vault: vault.clone(),
            },
        )
    }

    pub fn query_vault_configs(
        &self,
        start_after: Option<VaultUnchecked>,
        limit: Option<u32>,
    ) -> Vec<RoverVaultInfoResponse> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::VaultsInfo {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn get_vault(&self, vault: &VaultTestInfo) -> VaultUnchecked {
        self.query_vault_configs(None, Some(30)) // Max limit
            .iter()
            .find(|v| {
                let info = v
                    .vault
                    .check(&MockApi::default())
                    .unwrap()
                    .query_info(&self.app.wrap())
                    .unwrap();
                vault.vault_token_denom == info.vault_token
            })
            .unwrap()
            .vault
            .clone()
    }

    pub fn query_allowed_coins(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<String> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllowedCoins {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_coin_balances(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> Vec<CoinBalanceResponseItem> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllCoinBalances {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_debt_shares(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> Vec<SharesResponseItem> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllDebtShares {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_total_debt_shares(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<DebtShares> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllTotalDebtShares {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_lent_shares(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> Vec<SharesResponseItem> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllLentShares {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_total_lent_shares(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Vec<LentShares> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllTotalLentShares {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_total_debt_shares(&self, denom: &str) -> DebtShares {
        self.app
            .wrap()
            .query_wasm_smart(self.rover.clone(), &QueryMsg::TotalDebtShares(denom.to_string()))
            .unwrap()
    }

    pub fn query_total_lent_shares(&self, denom: &str) -> LentShares {
        self.app
            .wrap()
            .query_wasm_smart(self.rover.clone(), &QueryMsg::TotalLentShares(denom.to_string()))
            .unwrap()
    }

    pub fn query_red_bank_debt(&self, denom: &str) -> UserDebtResponse {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(
                config.red_bank,
                &UserDebt {
                    user: self.rover.to_string(),
                    denom: denom.into(),
                },
            )
            .unwrap()
    }

    pub fn query_red_bank_collateral(&self, denom: &str) -> UserCollateralResponse {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(
                config.red_bank,
                &UserCollateral {
                    user: self.rover.to_string(),
                    denom: denom.into(),
                },
            )
            .unwrap()
    }

    pub fn query_preview_redeem(&self, vault: &VaultUnchecked, shares: Uint128) -> Uint128 {
        vault
            .check(&MockApi::default())
            .unwrap()
            .query_preview_redeem(&self.app.wrap(), shares)
            .unwrap()
    }

    pub fn query_unlocking_position(&self, vault: &VaultUnchecked, id: u64) -> UnlockingPosition {
        vault
            .check(&MockApi::default())
            .unwrap()
            .query_unlocking_position(&self.app.wrap(), id)
            .unwrap()
    }

    pub fn query_unlocking_positions(
        &self,
        vault: &VaultUnchecked,
        addr: &Addr,
    ) -> Vec<UnlockingPosition> {
        self.app
            .wrap()
            .query_wasm_smart(
                vault.address.to_string(),
                &VaultExtension(ExtensionQueryMsg::Lockup(LockupQueryMsg::UnlockingPositions {
                    owner: addr.to_string(),
                    start_after: None,
                    limit: None,
                })),
            )
            .unwrap()
    }

    pub fn query_total_vault_coin_balance(&self, vault: &VaultUnchecked) -> Uint128 {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::TotalVaultCoinBalance {
                    vault: vault.clone(),
                },
            )
            .unwrap()
    }

    pub fn query_all_vault_positions(
        &self,
        start_after: Option<(String, String)>,
        limit: Option<u32>,
    ) -> Vec<VaultPositionResponseItem> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllVaultPositions {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_all_total_vault_coin_balances(
        &self,
        start_after: Option<VaultUnchecked>,
        limit: Option<u32>,
    ) -> Vec<VaultWithBalance> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AllTotalVaultCoinBalances {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }

    pub fn query_swap_estimate(
        &self,
        coin_in: &Coin,
        denom_out: &str,
    ) -> EstimateExactInSwapResponse {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(
                config.swapper,
                &EstimateExactInSwap {
                    coin_in: coin_in.clone(),
                    denom_out: denom_out.to_string(),
                },
            )
            .unwrap()
    }

    pub fn estimate_provide_liquidity(&self, lp_token_out: &str, coins_in: &[Coin]) -> Uint128 {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(
                config.zapper,
                &EstimateProvideLiquidity {
                    lp_token_out: lp_token_out.to_string(),
                    coins_in: coins_in.to_vec(),
                },
            )
            .unwrap()
    }

    pub fn query_vault_position_value(
        &self,
        position: &VaultPosition,
    ) -> StdResult<VPositionValue> {
        self.app.wrap().query_wasm_smart(
            self.rover.clone(),
            &VaultPositionValue {
                vault_position: position.clone(),
            },
        )
    }
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        let rover = self.get_rover()?;
        let mars_oracle = self.get_oracle();

        let health_contract = self.get_health_contract();
        self.update_health_contract_config(rover.clone());

        self.deploy_nft_contract(&rover);
        self.fund_users();

        Ok(MockEnv {
            app: take(&mut self.app),
            rover,
            mars_oracle: mars_oracle.address().clone(),
            health_contract,
        })
    }

    //--------------------------------------------------------------------------------------------------
    // Execute Msgs
    //--------------------------------------------------------------------------------------------------

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

    fn deploy_nft_contract(&mut self, rover: &Addr) {
        let nft_minter = Addr::unchecked("original_nft_minter");

        if self.deploy_nft_contract {
            let nft_contract = deploy_nft_contract(&mut self.app, &nft_minter);
            if self.set_nft_contract_minter {
                propose_new_nft_minter(&mut self.app, nft_contract.clone(), &nft_minter, rover);
                self.update_config(
                    rover,
                    ConfigUpdates {
                        account_nft: Some(nft_contract.to_string()),
                        ..Default::default()
                    },
                )
            }
        }
    }

    pub fn update_config(&mut self, rover: &Addr, updates: ConfigUpdates) {
        self.app
            .execute_contract(
                self.get_owner(),
                rover.clone(),
                &ExecuteMsg::UpdateConfig {
                    updates,
                },
                &[],
            )
            .unwrap();
    }

    //--------------------------------------------------------------------------------------------------
    // Get or defaults
    //--------------------------------------------------------------------------------------------------

    fn get_rover(&mut self) -> AnyResult<Addr> {
        let code_id = self.app.store_code(mock_rover_contract());
        let red_bank = self.get_red_bank().into();
        let swapper = self.deploy_swapper().into();
        let allowed_coins =
            self.get_allowed_coins().iter().map(|info| info.denom.clone()).collect();
        let max_close_factor = self.get_max_close_factor();
        let max_unlocking_positions = self.get_max_unlocking_positions();

        let mut vault_configs = vec![];
        vault_configs.extend(self.deploy_vaults());
        vault_configs.extend(self.pre_deployed_vaults.clone().unwrap_or_default());

        let oracle = self.get_oracle().into();
        let zapper = self.deploy_zapper(&oracle)?.into();
        let health_contract = self.get_health_contract().into();

        self.app.instantiate_contract(
            code_id,
            self.get_owner(),
            &InstantiateMsg {
                owner: self.get_owner().to_string(),
                allowed_coins,
                vault_configs,
                red_bank,
                oracle,
                max_close_factor,
                max_unlocking_positions,
                swapper,
                zapper,
                health_contract,
            },
            &[],
            "mock-rover-contract",
            None,
        )
    }

    fn get_owner(&self) -> Addr {
        self.owner.clone().unwrap_or_else(|| Addr::unchecked("owner"))
    }

    fn get_oracle(&mut self) -> Oracle {
        if self.oracle.is_none() {
            let addr = self.deploy_oracle();
            self.oracle = Some(addr);
        }
        self.oracle.clone().unwrap()
    }

    fn deploy_oracle(&mut self) -> Oracle {
        let contract_code_id = self.app.store_code(mock_oracle_contract());
        let mut prices: Vec<CoinPrice> = self
            .get_allowed_coins()
            .iter()
            .map(|item| CoinPrice {
                denom: item.denom.clone(),
                price: item.price,
            })
            .collect();
        prices.push(CoinPrice {
            denom: "uusdc".to_string(),
            price: Decimal::from_atomics(12345u128, 4).unwrap(),
        });

        // Ensures vault base token denoms are pricable in the oracle
        // even if they are not whitelisted in Rover
        let price_denoms = prices.iter().map(|c| c.denom.clone()).collect::<Vec<_>>();
        self.vault_configs.clone().unwrap_or_default().iter().for_each(|v| {
            if !price_denoms.contains(&v.base_token_denom) {
                prices.push(CoinPrice {
                    denom: v.base_token_denom.clone(),
                    price: Decimal::from_atomics(456u128, 5).unwrap(),
                });
            }
        });

        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                Addr::unchecked("oracle_contract_owner"),
                &OracleInstantiateMsg {
                    prices,
                },
                &[],
                "mock-oracle",
                None,
            )
            .unwrap();
        OracleBase::new(addr)
    }

    fn get_health_contract(&mut self) -> HealthContract {
        if self.health_contract.is_none() {
            let hc = self.deploy_health_contract();
            self.health_contract = Some(hc);
        }
        self.health_contract.clone().unwrap()
    }

    pub fn deploy_health_contract(&mut self) -> HealthContract {
        let contract_code_id = self.app.store_code(mock_health_contract());
        let owner = Addr::unchecked("health_contract_owner");

        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                owner.clone(),
                &HealthInstantiateMsg {
                    owner: owner.to_string(),
                },
                &[],
                "mock-health-contract",
                Some(owner.to_string()),
            )
            .unwrap();

        HealthContract::new(addr)
    }

    fn update_health_contract_config(&mut self, cm_addr: Addr) {
        let owner = Addr::unchecked("health_contract_owner");
        let health_contract = self.get_health_contract();

        self.app
            .execute_contract(
                owner,
                health_contract.address().clone(),
                &UpdateConfig {
                    credit_manager: cm_addr.to_string(),
                },
                &[],
            )
            .unwrap();
    }

    fn get_red_bank(&mut self) -> RedBankBase<Addr> {
        if self.red_bank.is_none() {
            let addr = self.deploy_red_bank();
            self.red_bank = Some(addr);
        }
        self.red_bank.clone().unwrap()
    }

    pub fn deploy_red_bank(&mut self) -> RedBankBase<Addr> {
        let contract_code_id = self.app.store_code(mock_red_bank_contract());
        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                Addr::unchecked("red_bank_contract_owner"),
                &RedBankInstantiateMsg {
                    coins: self
                        .get_allowed_coins()
                        .iter()
                        .map(|item| CoinMarketInfo {
                            denom: item.denom.to_string(),
                            max_ltv: item.max_ltv,
                            liquidation_threshold: item.liquidation_threshold,
                            liquidation_bonus: item.liquidation_bonus,
                        })
                        .collect(),
                },
                &[],
                "mock-red-bank",
                None,
            )
            .unwrap();

        // fund red bank with whitelisted coins
        if !self.get_allowed_coins().is_empty() {
            self.app
                .sudo(SudoMsg::Bank(BankSudo::Mint {
                    to_address: addr.to_string(),
                    amount: self
                        .get_allowed_coins()
                        .iter()
                        .map(|info| info.to_coin(DEFAULT_RED_BANK_COIN_BALANCE.u128()))
                        .collect(),
                }))
                .unwrap();
        }

        RedBankBase::new(addr)
    }

    fn deploy_vault(&mut self, vault: &VaultTestInfo) -> VaultInstantiateConfig {
        let code_id = self.app.store_code(mock_vault_contract());
        let oracle = self.get_oracle().into();
        let addr = self
            .app
            .instantiate_contract(
                code_id,
                Addr::unchecked("vault-instantiator"),
                &VaultInstantiateMsg {
                    vault_token_denom: vault.clone().vault_token_denom,
                    lockup: vault.lockup,
                    base_token_denom: vault.clone().base_token_denom,
                    oracle,
                },
                &[],
                "mock-vault",
                None,
            )
            .unwrap();
        self.fund_vault(&addr, &vault.vault_token_denom);
        VaultInstantiateConfig {
            vault: VaultBase::new(addr.to_string()),
            config: VaultConfig {
                deposit_cap: vault.deposit_cap.clone(),
                max_ltv: vault.max_ltv,
                liquidation_threshold: vault.liquidation_threshold,
                whitelisted: vault.whitelisted,
            },
        }
    }

    fn deploy_swapper(&mut self) -> Swapper {
        let code_id = self.app.store_code(mock_swapper_contract());
        let addr = self
            .app
            .instantiate_contract(
                code_id,
                Addr::unchecked("swapper-instantiator"),
                &SwapperInstantiateMsg {
                    owner: self.get_owner().to_string(),
                },
                &[],
                "mock-vault",
                None,
            )
            .unwrap();
        // Fund with osmo to simulate swaps
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: addr.to_string(),
                amount: coins(1_000_000, "uosmo"),
            }))
            .unwrap();
        SwapperBase::new(addr)
    }

    fn deploy_zapper(&mut self, oracle: &OracleUnchecked) -> AnyResult<Zapper> {
        let code_id = self.app.store_code(mock_zapper_contract());
        let lp_token = lp_token_info();
        let addr = self.app.instantiate_contract(
            code_id,
            Addr::unchecked("zapper-instantiator"),
            &ZapperInstantiateMsg {
                oracle: oracle.clone(),
                lp_configs: vec![LpConfig {
                    lp_token_denom: lp_token.denom.to_string(),
                    lp_pair_denoms: ("uatom".to_string(), "uosmo".to_string()),
                }],
            },
            &[],
            "mock-vault",
            None,
        )?;
        // Fund with lp tokens to simulate mints
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: addr.to_string(),
                amount: coins(10_000_000, lp_token.denom),
            }))
            .unwrap();
        Ok(ZapperBase::new(addr))
    }

    /// cw-multi-test does not yet have the ability to mint sdk coins. For this reason,
    /// this contract expects to be pre-funded with vault tokens and it will simulate the mint.
    fn fund_vault(&mut self, vault_addr: &Addr, denom: &str) {
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: vault_addr.to_string(),
                amount: vec![Coin {
                    denom: denom.into(),
                    amount: DEFAULT_VAULT_TOKEN_PREFUND,
                }],
            }))
            .unwrap();
    }

    fn deploy_vaults(&mut self) -> Vec<VaultInstantiateConfig> {
        self.vault_configs
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|v| self.deploy_vault(v))
            .collect()
    }

    fn get_allowed_coins(&self) -> Vec<CoinInfo> {
        self.allowed_coins.clone().unwrap_or_default()
    }

    fn get_max_close_factor(&self) -> Decimal {
        self.max_close_factor.unwrap_or_else(|| Decimal::from_atomics(5u128, 1).unwrap())
        // 50%
    }

    fn get_max_unlocking_positions(&self) -> Uint128 {
        self.max_unlocking_positions.unwrap_or_else(|| Uint128::new(100))
    }

    //--------------------------------------------------------------------------------------------------
    // Setter functions
    //--------------------------------------------------------------------------------------------------

    pub fn fund_account(&mut self, account: AccountToFund) -> &mut Self {
        self.accounts_to_fund.push(account);
        self
    }

    pub fn owner(&mut self, owner: &str) -> &mut Self {
        self.owner = Some(Addr::unchecked(owner));
        self
    }

    pub fn vault_configs(&mut self, vault_configs: &[VaultTestInfo]) -> &mut Self {
        self.vault_configs = Some(vault_configs.to_vec());
        self
    }

    pub fn allowed_coins(&mut self, allowed_coins: &[CoinInfo]) -> &mut Self {
        self.allowed_coins = Some(allowed_coins.to_vec());
        self
    }

    pub fn red_bank(&mut self, red_bank: &str) -> &mut Self {
        self.red_bank = Some(RedBankBase::new(Addr::unchecked(red_bank)));
        self
    }

    pub fn oracle(&mut self, addr: &str) -> &mut Self {
        self.oracle = Some(OracleBase::new(Addr::unchecked(addr)));
        self
    }

    pub fn no_nft_contract(&mut self) -> &mut Self {
        self.deploy_nft_contract = false;
        self
    }

    pub fn no_nft_contract_minter(&mut self) -> &mut Self {
        self.set_nft_contract_minter = false;
        self
    }

    pub fn pre_deployed_vault(
        &mut self,
        info: &VaultTestInfo,
        config: Option<VaultInstantiateConfig>,
    ) -> &mut Self {
        let config = config.unwrap_or_else(|| self.deploy_vault(info));
        let new_list = match self.pre_deployed_vaults.clone() {
            None => Some(vec![config]),
            Some(mut curr) => {
                curr.push(config);
                Some(curr)
            }
        };
        self.pre_deployed_vaults = new_list;
        self
    }

    pub fn max_close_factor(&mut self, cf: Decimal) -> &mut Self {
        self.max_close_factor = Some(cf);
        self
    }

    pub fn max_unlocking_positions(&mut self, max: u128) -> &mut Self {
        self.max_unlocking_positions = Some(Uint128::new(max));
        self
    }
}

//--------------------------------------------------------------------------------------------------
// Shared utils between MockBuilder & MockEnv
//--------------------------------------------------------------------------------------------------

fn deploy_nft_contract(app: &mut App, minter: &Addr) -> Addr {
    let nft_contract_code_id = app.store_code(mock_account_nft_contract());
    app.instantiate_contract(
        nft_contract_code_id,
        minter.clone(),
        &NftInstantiateMsg {
            max_value_for_burn: Default::default(),
            health_contract: None,
            name: "Rover Credit Account".to_string(),
            symbol: "RCA".to_string(),
            minter: minter.to_string(),
        },
        &[],
        "manager-mock-account-nft",
        None,
    )
    .unwrap()
}

fn propose_new_nft_minter(app: &mut App, nft_contract: Addr, old_minter: &Addr, new_minter: &Addr) {
    let proposal_msg: NftExecuteMsg = NftExecuteMsg::UpdateConfig {
        updates: NftConfigUpdates {
            max_value_for_burn: None,
            proposed_new_minter: Some(new_minter.into()),
            health_contract_addr: None,
        },
    };
    app.execute_contract(old_minter.clone(), nft_contract, &proposal_msg, &[]).unwrap();
}
