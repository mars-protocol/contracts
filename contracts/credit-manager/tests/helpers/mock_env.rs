use std::{default::Default, mem::take, str::FromStr};

use anyhow::Result as AnyResult;
use cosmwasm_std::{coins, testing::MockApi, Addr, Coin, Decimal, Empty, StdResult, Uint128};
use cw721::TokensResponse;
use cw721_base::{Action::TransferOwnership, Ownership};
use cw_multi_test::{App, AppResponse, BankSudo, BasicApp, Executor, SudoMsg};
use cw_vault_standard::{
    extensions::lockup::{LockupQueryMsg, UnlockingPosition},
    msg::{ExtensionQueryMsg, VaultStandardQueryMsg::VaultExtension},
};
use mars_account_nft::{
    msg::{
        ExecuteMsg as NftExecuteMsg, InstantiateMsg as NftInstantiateMsg, QueryMsg as NftQueryMsg,
    },
    nft_config::{NftConfigUpdates, UncheckedNftConfig},
};
use mars_mock_oracle::msg::{
    CoinPrice, ExecuteMsg as OracleExecuteMsg, InstantiateMsg as OracleInstantiateMsg,
};
use mars_mock_vault::{
    contract::DEFAULT_VAULT_TOKEN_PREFUND, msg::InstantiateMsg as VaultInstantiateMsg,
};
use mars_owner::OwnerUpdate;
use mars_params::{
    msg::{
        AssetParamsUpdate,
        AssetParamsUpdate::AddOrUpdate,
        ExecuteMsg::{UpdateAssetParams, UpdateVaultConfig},
        InstantiateMsg as ParamsInstantiateMsg, QueryMsg as ParamsQueryMsg, VaultConfigUpdate,
    },
    types::{
        asset::AssetParams,
        vault::{VaultConfig, VaultConfigUnchecked},
    },
};
use mars_red_bank_types::{
    oracle::ActionKind,
    red_bank::{
        QueryMsg::{UserCollateral, UserDebt},
        UserCollateralResponse, UserDebtResponse,
    },
};
use mars_rover::{
    adapters::{
        account_nft::AccountNftUnchecked,
        health::HealthContract,
        oracle::{Oracle, OracleBase, OracleUnchecked},
        params::Params,
        red_bank::RedBankBase,
        swap::{
            EstimateExactInSwapResponse, InstantiateMsg as SwapperInstantiateMsg,
            QueryMsg::EstimateExactInSwap, Swapper, SwapperBase,
        },
        vault::{Vault, VaultPosition, VaultPositionValue as VPositionValue, VaultUnchecked},
        zapper::{Zapper, ZapperBase},
    },
    msg::{
        execute::{Action, CallbackMsg},
        instantiate::ConfigUpdates,
        query::{
            CoinBalanceResponseItem, ConfigResponse, DebtShares, LentShares, Positions,
            SharesResponseItem, VaultPositionResponseItem, VaultUtilizationResponse,
        },
        ExecuteMsg, InstantiateMsg, QueryMsg,
        QueryMsg::{EstimateProvideLiquidity, VaultPositionValue},
    },
};
use mars_rover_health_types::{
    AccountKind, ExecuteMsg::UpdateConfig, HealthValuesResponse,
    InstantiateMsg as HealthInstantiateMsg, QueryMsg::HealthValues,
};
use mars_v2_zapper_mock::msg::{InstantiateMsg as ZapperInstantiateMsg, LpConfig};

use crate::helpers::{
    lp_token_info, mock_account_nft_contract, mock_health_contract, mock_oracle_contract,
    mock_params_contract, mock_red_bank_contract, mock_rover_contract, mock_swapper_contract,
    mock_v2_zapper_contract, mock_vault_contract, AccountToFund, CoinInfo, VaultTestInfo,
};

pub const DEFAULT_RED_BANK_COIN_BALANCE: Uint128 = Uint128::new(1_000_000);

pub struct MockEnv {
    pub app: BasicApp,
    pub rover: Addr,
    pub mars_oracle: Addr,
    pub health_contract: HealthContract,
    pub params: Params,
}

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub owner: Option<Addr>,
    pub emergency_owner: Option<Addr>,
    pub vault_configs: Option<Vec<VaultTestInfo>>,
    pub coin_params: Option<Vec<CoinInfo>>,
    pub oracle: Option<Oracle>,
    pub params: Option<Params>,
    pub red_bank: Option<RedBankBase<Addr>>,
    pub deploy_nft_contract: bool,
    pub set_nft_contract_minter: bool,
    pub accounts_to_fund: Vec<AccountToFund>,
    pub target_health_factor: Option<Decimal>,
    pub max_unlocking_positions: Option<Uint128>,
    pub health_contract: Option<HealthContract>,
    pub evil_vault: Option<String>,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            owner: None,
            emergency_owner: None,
            vault_configs: None,
            coin_params: None,
            oracle: None,
            params: None,
            red_bank: None,
            deploy_nft_contract: true,
            set_nft_contract_minter: true,
            accounts_to_fund: vec![],
            target_health_factor: None,
            max_unlocking_positions: None,
            health_contract: None,
            evil_vault: None,
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

    pub fn repay_from_wallet(
        &mut self,
        sender: &Addr,
        account_id: &str,
        funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::RepayFromWallet {
                account_id: account_id.to_string(),
            },
            funds,
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

    pub fn update_asset_params(&mut self, update: AssetParamsUpdate) {
        let config = self.query_config();
        self.app
            .execute_contract(
                Addr::unchecked(config.ownership.owner.unwrap()),
                Addr::unchecked(config.params),
                &UpdateAssetParams(update),
                &[],
            )
            .unwrap();
    }

    pub fn update_vault_params(&mut self, update: VaultConfigUpdate) {
        let config = self.query_config();
        self.app
            .execute_contract(
                Addr::unchecked(config.ownership.owner.unwrap()),
                Addr::unchecked(config.params),
                &UpdateVaultConfig(update),
                &[],
            )
            .unwrap();
    }

    pub fn update_nft_config(
        &mut self,
        sender: &Addr,
        config: Option<NftConfigUpdates>,
        ownership: Option<cw721_base::Action>,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateNftConfig {
                config,
                ownership,
            },
            &[],
        )
    }

    pub fn deploy_new_nft_contract(&mut self) -> AnyResult<AccountNftUnchecked> {
        let nft_minter = Addr::unchecked("original_nft_minter");
        let nft_contract = deploy_nft_contract(&mut self.app, &nft_minter);
        propose_new_nft_minter(
            &mut self.app,
            nft_contract.clone(),
            &nft_minter.clone(),
            &self.rover.clone(),
        );
        Ok(AccountNftUnchecked::new(nft_contract.to_string()))
    }

    pub fn create_credit_account(&mut self, sender: &Addr) -> AnyResult<String> {
        self._create_credit_account(sender, AccountKind::Default)
    }

    pub fn create_hls_account(&mut self, sender: &Addr) -> String {
        self._create_credit_account(sender, AccountKind::HighLeveredStrategy).unwrap()
    }

    fn _create_credit_account(&mut self, sender: &Addr, kind: AccountKind) -> AnyResult<String> {
        let res = self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::CreateCreditAccount(kind),
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

    pub fn query_health(
        &self,
        account_id: &str,
        kind: AccountKind,
        action: ActionKind,
    ) -> HealthValuesResponse {
        self.app
            .wrap()
            .query_wasm_smart(
                self.health_contract.clone().address(),
                &HealthValues {
                    account_id: account_id.to_string(),
                    kind,
                    action,
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

    pub fn query_account_kind(&self, account_id: &str) -> AccountKind {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::AccountKind {
                    account_id: account_id.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_nft_config(&self) -> UncheckedNftConfig {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(config.account_nft.unwrap(), &NftQueryMsg::Config {})
            .unwrap()
    }

    pub fn query_nft_ownership(&self) -> Ownership<Addr> {
        let config = self.query_config();
        self.app
            .wrap()
            .query_wasm_smart(config.account_nft.unwrap(), &NftQueryMsg::Ownership {})
            .unwrap()
    }

    pub fn query_rewards_collector_account(&self) -> String {
        let config = self.query_config();
        let response: TokensResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                config.account_nft.unwrap(),
                &NftQueryMsg::Tokens {
                    owner: config.rewards_collector.unwrap(),
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        response.tokens.first().unwrap().to_string()
    }

    pub fn query_vault_params(&self, vault_addr: &str) -> VaultConfig {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params.address(),
                &ParamsQueryMsg::VaultConfig {
                    address: vault_addr.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_asset_params(&self, denom: &str) -> AssetParams {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params.address(),
                &ParamsQueryMsg::AssetParams {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_all_vault_params(&self) -> Vec<VaultConfig> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.params.address(),
                &ParamsQueryMsg::AllVaultConfigs {
                    start_after: None,
                    limit: Some(30), // Max limit
                },
            )
            .unwrap()
    }

    pub fn get_vault(&self, vault: &VaultTestInfo) -> VaultUnchecked {
        let vault_params = self.query_all_vault_params();
        let matched_vault = vault_params
            .iter()
            .find(|v| {
                let info = Vault::new(v.addr.clone()).query_info(&self.app.wrap()).unwrap();
                vault.vault_token_denom == info.vault_token
            })
            .unwrap();
        VaultUnchecked::new(matched_vault.addr.to_string())
    }

    pub fn query_vault_utilization(
        &self,
        vault: &VaultUnchecked,
    ) -> StdResult<VaultUtilizationResponse> {
        self.app.wrap().query_wasm_smart(
            self.rover.clone(),
            &QueryMsg::VaultUtilization {
                vault: vault.clone(),
            },
        )
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
        let info = Vault::new(Addr::unchecked(vault.address.clone()))
            .query_info(&self.app.wrap())
            .unwrap();
        self.app.wrap().query_balance(self.rover.clone(), info.vault_token).unwrap().amount
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
        self.set_emergency_owner(&rover);

        let mars_oracle = self.get_oracle();

        let params = self.get_params_contract();
        self.add_params_to_contract();

        let health_contract = self.get_health_contract();
        self.update_health_contract_config(&rover);

        self.deploy_nft_contract(&rover);

        if self.deploy_nft_contract && self.set_nft_contract_minter {
            self.update_config(
                &rover,
                ConfigUpdates {
                    rewards_collector: Some("rewards_collector_contract".to_string()),
                    ..Default::default()
                },
            );
        }

        self.fund_users();

        self.deploy_vaults();

        Ok(MockEnv {
            app: take(&mut self.app),
            rover,
            mars_oracle: mars_oracle.address().clone(),
            health_contract,
            params,
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
                        account_nft: Some(AccountNftUnchecked::new(nft_contract.to_string())),
                        ..Default::default()
                    },
                )
            }
        }
    }

    fn add_params_to_contract(&mut self) {
        let params_to_set = self.get_coin_params();
        let params_contract = self.get_params_contract();

        for coin_info in params_to_set {
            self.app
                .execute_contract(
                    self.get_owner(),
                    params_contract.address().clone(),
                    &UpdateAssetParams(AddOrUpdate {
                        params: coin_info.into(),
                    }),
                    &[],
                )
                .unwrap();
        }
    }

    pub fn set_emergency_owner(&mut self, rover: &Addr) {
        if let Some(eo) = self.emergency_owner.clone() {
            self.app
                .execute_contract(
                    self.get_owner(),
                    rover.clone(),
                    &ExecuteMsg::UpdateOwner(OwnerUpdate::SetEmergencyOwner {
                        emergency_owner: eo.to_string(),
                    }),
                    &[],
                )
                .unwrap();
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
        let max_unlocking_positions = self.get_max_unlocking_positions();

        let oracle = self.get_oracle().into();
        let zapper = self.deploy_zapper(&oracle)?.into();
        let health_contract = self.get_health_contract().into();
        let params = self.get_params_contract().into();

        self.app.instantiate_contract(
            code_id,
            self.get_owner(),
            &InstantiateMsg {
                owner: self.get_owner().to_string(),
                red_bank,
                oracle,
                max_unlocking_positions,
                swapper,
                zapper,
                health_contract,
                params,
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
            .get_coin_params()
            .iter()
            .map(|item| CoinPrice {
                pricing: ActionKind::Default,
                denom: item.denom.clone(),
                price: item.price,
            })
            .collect();
        prices.push(CoinPrice {
            pricing: ActionKind::Default,
            denom: "uusdc".to_string(),
            price: Decimal::from_atomics(12345u128, 4).unwrap(),
        });

        // Ensures vault base token denoms are pricable in the oracle
        // even if they are not whitelisted in Rover
        let price_denoms = prices.iter().map(|c| c.denom.clone()).collect::<Vec<_>>();
        self.vault_configs.clone().unwrap_or_default().iter().for_each(|v| {
            if !price_denoms.contains(&v.base_token_denom) {
                prices.push(CoinPrice {
                    pricing: ActionKind::Default,
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

    fn get_params_contract(&mut self) -> Params {
        if self.params.is_none() {
            let hc = self.deploy_params_contract();
            self.params = Some(hc);
        }
        self.params.clone().unwrap()
    }

    pub fn deploy_params_contract(&mut self) -> Params {
        let contract_code_id = self.app.store_code(mock_params_contract());
        let owner = self.get_owner();

        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                owner.clone(),
                &ParamsInstantiateMsg {
                    owner: owner.to_string(),
                    target_health_factor: self
                        .target_health_factor
                        .unwrap_or(Decimal::from_str("1.2").unwrap()),
                },
                &[],
                "mock-params-contract",
                Some(owner.to_string()),
            )
            .unwrap();

        Params::new(addr)
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
        let owner = self.get_owner();

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

    fn update_health_contract_config(&mut self, cm_addr: &Addr) {
        let health_contract = self.get_health_contract();

        self.app
            .execute_contract(
                self.get_owner(),
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
                &Empty {},
                &[],
                "mock-red-bank",
                None,
            )
            .unwrap();

        // fund red bank with whitelisted coins
        if !self.get_coin_params().is_empty() {
            self.app
                .sudo(SudoMsg::Bank(BankSudo::Mint {
                    to_address: addr.to_string(),
                    amount: self
                        .get_coin_params()
                        .iter()
                        .map(|info| info.to_coin(DEFAULT_RED_BANK_COIN_BALANCE.u128()))
                        .collect(),
                }))
                .unwrap();
        }

        RedBankBase::new(addr)
    }

    fn deploy_vault(&mut self, vault: &VaultTestInfo) -> Addr {
        let code_id = self.app.store_code(mock_vault_contract());
        let oracle = self.get_oracle().into();
        let vault_addr = self
            .app
            .instantiate_contract(
                code_id,
                Addr::unchecked("vault-instantiator"),
                &VaultInstantiateMsg {
                    vault_token_denom: vault.clone().vault_token_denom,
                    lockup: vault.lockup,
                    base_token_denom: vault.clone().base_token_denom,
                    oracle,
                    is_evil: self.evil_vault.clone(),
                },
                &[],
                "mock-vault",
                None,
            )
            .unwrap();
        self.fund_vault(&vault_addr, &vault.vault_token_denom);

        let params = self.get_params_contract();

        self.app
            .execute_contract(
                self.get_owner(),
                params.address().clone(),
                &UpdateVaultConfig(VaultConfigUpdate::AddOrUpdate {
                    config: VaultConfigUnchecked {
                        addr: vault_addr.to_string(),
                        deposit_cap: vault.deposit_cap.clone(),
                        max_loan_to_value: vault.max_ltv,
                        liquidation_threshold: vault.liquidation_threshold,
                        whitelisted: vault.whitelisted,
                        hls: vault.hls.clone(),
                    },
                }),
                &[],
            )
            .unwrap();

        vault_addr
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
        let code_id = self.app.store_code(mock_v2_zapper_contract());
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

    fn deploy_vaults(&mut self) -> Vec<Addr> {
        self.vault_configs
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|v| self.deploy_vault(v))
            .collect()
    }

    fn get_coin_params(&self) -> Vec<CoinInfo> {
        self.coin_params.clone().unwrap_or_default()
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

    pub fn emergency_owner(&mut self, eo: &Addr) -> &mut Self {
        self.emergency_owner = Some(eo.clone());
        self
    }

    pub fn vault_configs(&mut self, vault_configs: &[VaultTestInfo]) -> &mut Self {
        self.vault_configs = Some(vault_configs.to_vec());
        self
    }

    pub fn set_params(&mut self, coins: &[CoinInfo]) -> &mut Self {
        self.coin_params = Some(coins.to_vec());
        self
    }

    pub fn params_contract(&mut self, params: &str) -> &mut Self {
        self.params = Some(Params::new(Addr::unchecked(params)));
        self
    }

    pub fn health_contract(&mut self, health: &str) -> &mut Self {
        self.health_contract = Some(HealthContract::new(Addr::unchecked(health)));
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

    pub fn params(&mut self, addr: &str) -> &mut Self {
        self.params = Some(Params::new(Addr::unchecked(addr)));
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

    pub fn target_health_factor(&mut self, thf: Decimal) -> &mut Self {
        self.target_health_factor = Some(thf);
        self
    }

    pub fn max_unlocking_positions(&mut self, max: u128) -> &mut Self {
        self.max_unlocking_positions = Some(Uint128::new(max));
        self
    }

    pub fn evil_vault(&mut self, credit_account: &str) -> &mut Self {
        self.evil_vault = Some(credit_account.to_string());
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
    let proposal_msg: NftExecuteMsg = NftExecuteMsg::UpdateOwnership(TransferOwnership {
        new_owner: new_minter.into(),
        expiry: None,
    });
    app.execute_contract(old_minter.clone(), nft_contract, &proposal_msg, &[]).unwrap();
}
