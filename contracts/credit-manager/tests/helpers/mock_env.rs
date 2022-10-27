use std::mem::take;

use anyhow::Result as AnyResult;
use cosmwasm_std::testing::MockApi;
use cosmwasm_std::{coins, Addr, Coin, Decimal, Uint128};
use cw721_base::InstantiateMsg as NftInstantiateMsg;
use cw_multi_test::{App, AppResponse, BankSudo, BasicApp, Executor, SudoMsg};
use mars_outpost::red_bank::QueryMsg::UserDebt;
use mars_outpost::red_bank::UserDebtResponse;

use account_nft::msg::ExecuteMsg as NftExecuteMsg;
use mars_oracle_adapter::msg::{
    InstantiateMsg as OracleAdapterInstantiateMsg, PricingMethod, VaultPricingInfo,
};
use mock_oracle::msg::{
    CoinPrice, ExecuteMsg as OracleExecuteMsg, InstantiateMsg as OracleInstantiateMsg,
};
use mock_red_bank::msg::{CoinMarketInfo, InstantiateMsg as RedBankInstantiateMsg};
use mock_vault::contract::DEFAULT_VAULT_TOKEN_PREFUND;
use mock_vault::msg::InstantiateMsg as VaultInstantiateMsg;
use rover::adapters::swap::QueryMsg::EstimateExactInSwap;
use rover::adapters::swap::{
    EstimateExactInSwapResponse, InstantiateMsg as SwapperInstantiateMsg, Swapper, SwapperBase,
};
use rover::adapters::vault::{VaultBase, VaultConfig, VaultUnchecked};
use rover::adapters::{OracleBase, RedBankBase};
use rover::msg::execute::{Action, CallbackMsg};
use rover::msg::instantiate::{ConfigUpdates, VaultInstantiateConfig};
use rover::msg::query::{
    CoinBalanceResponseItem, ConfigResponse, DebtShares, HealthResponse, Positions,
    SharesResponseItem, VaultPositionResponseItem, VaultWithBalance,
};
use rover::msg::vault::QueryMsg::{Info as VaultInfoMsg, UnlockingPositionsForAddr};
use rover::msg::vault::{UnlockingPosition, VaultInfo};
use rover::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

use crate::helpers::{
    mock_account_nft_contract, mock_oracle_adapter_contract, mock_oracle_contract,
    mock_red_bank_contract, mock_rover_contract, mock_swapper_contract, mock_vault_contract,
    AccountToFund, CoinInfo, VaultTestInfo,
};

pub const DEFAULT_RED_BANK_COIN_BALANCE: Uint128 = Uint128::new(1_000_000);

pub struct MockEnv {
    pub app: BasicApp,
    pub rover: Addr,
    pub mars_oracle: Addr,
}

pub struct MockEnvBuilder {
    pub app: BasicApp,
    pub owner: Option<Addr>,
    pub allowed_vaults: Option<Vec<VaultTestInfo>>,
    pub pre_deployed_vaults: Option<Vec<VaultInstantiateConfig>>,
    pub allowed_coins: Option<Vec<CoinInfo>>,
    pub oracle: Option<OracleBase<Addr>>,
    pub oracle_adapter: Option<OracleBase<Addr>>,
    pub red_bank: Option<RedBankBase<Addr>>,
    pub deploy_nft_contract: bool,
    pub set_nft_contract_owner: bool,
    pub accounts_to_fund: Vec<AccountToFund>,
    pub max_liquidation_bonus: Option<Decimal>,
    pub max_close_factor: Option<Decimal>,
}

#[allow(clippy::new_ret_no_self)]
impl MockEnv {
    pub fn new() -> MockEnvBuilder {
        MockEnvBuilder {
            app: App::default(),
            owner: None,
            allowed_vaults: None,
            pre_deployed_vaults: None,
            allowed_coins: None,
            oracle: None,
            oracle_adapter: None,
            red_bank: None,
            deploy_nft_contract: true,
            set_nft_contract_owner: true,
            accounts_to_fund: vec![],
            max_liquidation_bonus: None,
            max_close_factor: None,
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
        new_config: ConfigUpdates,
    ) -> AnyResult<AppResponse> {
        self.app.execute_contract(
            sender.clone(),
            self.rover.clone(),
            &ExecuteMsg::UpdateConfig { new_config },
            &[],
        )
    }

    pub fn deploy_nft_contract(&mut self) -> AnyResult<Addr> {
        let nft_contract = deploy_nft_contract(&mut self.app, &self.rover.clone());
        propose_new_nft_contract_owner(
            &mut self.app,
            nft_contract.clone(),
            &self.rover.clone(),
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
                self.rover.clone(),
                &QueryMsg::Health {
                    account_id: account_id.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_balance(&self, addr: &Addr, denom: &str) -> Coin {
        self.app.wrap().query_balance(addr.clone(), denom).unwrap()
    }

    pub fn query_config(&self) -> ConfigResponse {
        self.app
            .wrap()
            .query_wasm_smart(self.rover.clone(), &QueryMsg::Config {})
            .unwrap()
    }

    pub fn query_vault_configs(
        &self,
        start_after: Option<VaultUnchecked>,
        limit: Option<u32>,
    ) -> Vec<VaultInstantiateConfig> {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::VaultConfigs { start_after, limit },
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
                vault.denom == info.token_denom
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
                &QueryMsg::AllowedCoins { start_after, limit },
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
                &QueryMsg::AllCoinBalances { start_after, limit },
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
                &QueryMsg::AllDebtShares { start_after, limit },
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
                &QueryMsg::AllTotalDebtShares { start_after, limit },
            )
            .unwrap()
    }

    pub fn query_total_debt_shares(&self, denom: &str) -> DebtShares {
        self.app
            .wrap()
            .query_wasm_smart(
                self.rover.clone(),
                &QueryMsg::TotalDebtShares(denom.to_string()),
            )
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

    pub fn query_preview_redeem(&self, vault: &VaultUnchecked, shares: Uint128) -> Vec<Coin> {
        vault
            .check(&MockApi::default())
            .unwrap()
            .query_preview_redeem(&self.app.wrap(), shares)
            .unwrap()
    }

    pub fn query_unlocking_position_info(
        &self,
        vault: &VaultUnchecked,
        id: u64,
    ) -> UnlockingPosition {
        vault
            .check(&MockApi::default())
            .unwrap()
            .query_unlocking_position_info(&self.app.wrap(), id)
            .unwrap()
    }

    pub fn query_unlocking_positions(
        &self,
        vault: &VaultUnchecked,
        manager_contract_addr: &Addr,
    ) -> Vec<UnlockingPosition> {
        self.app
            .wrap()
            .query_wasm_smart(
                vault.address.to_string(),
                &UnlockingPositionsForAddr {
                    addr: manager_contract_addr.to_string(),
                },
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
                &QueryMsg::AllVaultPositions { start_after, limit },
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
                &QueryMsg::AllTotalVaultCoinBalances { start_after, limit },
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
}

impl MockEnvBuilder {
    pub fn build(&mut self) -> AnyResult<MockEnv> {
        let rover = self.get_rover()?;
        let mars_oracle = self.get_oracle();
        self.deploy_nft_contract(&rover);
        self.fund_users();

        Ok(MockEnv {
            app: take(&mut self.app),
            rover,
            mars_oracle: mars_oracle.address().clone(),
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
        let nft_contract_owner = Addr::unchecked("original_nft_contract_owner");

        if self.deploy_nft_contract {
            let nft_contract = deploy_nft_contract(&mut self.app, &nft_contract_owner);
            if self.set_nft_contract_owner {
                propose_new_nft_contract_owner(
                    &mut self.app,
                    nft_contract.clone(),
                    &nft_contract_owner,
                    rover,
                );
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

    pub fn update_config(&mut self, rover: &Addr, new_config: ConfigUpdates) {
        self.app
            .execute_contract(
                self.get_owner(),
                rover.clone(),
                &ExecuteMsg::UpdateConfig { new_config },
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
        let allowed_coins = self
            .get_allowed_coins()
            .iter()
            .map(|info| info.denom.clone())
            .collect();
        let max_liquidation_bonus = self.get_max_liquidation_bonus();
        let max_close_factor = self.get_max_close_factor();

        let mut allowed_vaults = vec![];
        allowed_vaults.extend(self.deploy_vaults());
        allowed_vaults.extend(self.pre_deployed_vaults.clone().unwrap_or_default());

        let oracle = self.get_oracle_adapter(allowed_vaults.clone()).into();

        self.app.instantiate_contract(
            code_id,
            self.get_owner(),
            &InstantiateMsg {
                owner: self.get_owner().to_string(),
                allowed_coins,
                allowed_vaults,
                red_bank,
                oracle,
                max_liquidation_bonus,
                max_close_factor,
                swapper,
            },
            &[],
            "mock-rover-contract",
            None,
        )
    }

    fn get_owner(&self) -> Addr {
        self.owner
            .clone()
            .unwrap_or_else(|| Addr::unchecked("owner"))
    }

    fn get_oracle(&mut self) -> OracleBase<Addr> {
        if self.oracle.is_none() {
            let addr = self.deploy_oracle();
            self.oracle = Some(addr);
        }
        self.oracle.clone().unwrap()
    }

    fn deploy_oracle(&mut self) -> OracleBase<Addr> {
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
        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                Addr::unchecked("oracle_contract_owner"),
                &OracleInstantiateMsg { prices },
                &[],
                "mock-oracle",
                None,
            )
            .unwrap();
        OracleBase::new(addr)
    }

    fn get_oracle_adapter(&mut self, vaults: Vec<VaultInstantiateConfig>) -> OracleBase<Addr> {
        if self.oracle_adapter.is_none() {
            let addr = self.deploy_oracle_adapter(vaults);
            self.oracle_adapter = Some(addr);
        }
        self.oracle_adapter.clone().unwrap()
    }

    fn deploy_oracle_adapter(&mut self, vaults: Vec<VaultInstantiateConfig>) -> OracleBase<Addr> {
        let owner = Addr::unchecked("oracle_adapter_contract_owner");
        let contract_code_id = self.app.store_code(mock_oracle_adapter_contract());
        let oracle = self.get_oracle().into();
        let vault_pricing = if self.pre_deployed_vaults.is_some() {
            vec![]
        } else {
            vaults
                .into_iter()
                .map(|config| {
                    let info: VaultInfo = self
                        .app
                        .wrap()
                        .query_wasm_smart(config.vault.address.clone(), &VaultInfoMsg {})
                        .unwrap();
                    VaultPricingInfo {
                        denom: info.token_denom,
                        addr: Addr::unchecked(config.vault.address),
                        method: PricingMethod::PreviewRedeem,
                    }
                })
                .collect()
        };
        let addr = self
            .app
            .instantiate_contract(
                contract_code_id,
                owner.clone(),
                &OracleAdapterInstantiateMsg {
                    oracle,
                    vault_pricing,
                    owner: owner.to_string(),
                },
                &[],
                "mars-oracle-adapter",
                None,
            )
            .unwrap();
        OracleBase::new(addr)
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
                    lp_token_denom: vault.clone().denom,
                    lockup: vault.lockup,
                    asset_denoms: vault.clone().underlying_denoms,
                    oracle,
                },
                &[],
                "mock-vault",
                None,
            )
            .unwrap();
        self.fund_vault(&addr, &vault.denom);
        VaultInstantiateConfig {
            vault: VaultBase::new(addr.to_string()),
            config: VaultConfig {
                deposit_cap: vault.deposit_cap.clone(),
                max_ltv: vault.max_ltv,
                liquidation_threshold: vault.liquidation_threshold,
                whitelisted: true,
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
        self.allowed_vaults
            .clone()
            .unwrap_or_default()
            .iter()
            .map(|v| self.deploy_vault(v))
            .collect()
    }

    fn get_allowed_coins(&self) -> Vec<CoinInfo> {
        self.allowed_coins.clone().unwrap_or_default()
    }

    fn get_max_liquidation_bonus(&self) -> Decimal {
        self.max_liquidation_bonus
            .unwrap_or_else(|| Decimal::from_atomics(5u128, 2).unwrap()) // 5%
    }

    fn get_max_close_factor(&self) -> Decimal {
        self.max_close_factor
            .unwrap_or_else(|| Decimal::from_atomics(5u128, 1).unwrap()) // 50%
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

    pub fn allowed_vaults(&mut self, allowed_vaults: &[VaultTestInfo]) -> &mut Self {
        self.allowed_vaults = Some(allowed_vaults.to_vec());
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

    pub fn oracle_adapter(&mut self, addr: &str) -> &mut Self {
        self.oracle_adapter = Some(OracleBase::new(Addr::unchecked(addr)));
        self
    }

    pub fn no_nft_contract(&mut self) -> &mut Self {
        self.deploy_nft_contract = false;
        self
    }

    pub fn no_nft_contract_owner(&mut self) -> &mut Self {
        self.set_nft_contract_owner = false;
        self
    }

    pub fn pre_deployed_vault(&mut self, address: &str, info: &VaultTestInfo) -> &mut Self {
        let config = VaultInstantiateConfig {
            vault: VaultBase::new(address.to_string()),
            config: VaultConfig {
                deposit_cap: info.deposit_cap.clone(),
                max_ltv: info.max_ltv,
                liquidation_threshold: info.liquidation_threshold,
                whitelisted: true,
            },
        };
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

    pub fn max_liquidation_bonus(&mut self, bonus: Decimal) -> &mut Self {
        self.max_liquidation_bonus = Some(bonus);
        self
    }

    pub fn max_close_factor(&mut self, cf: Decimal) -> &mut Self {
        self.max_close_factor = Some(cf);
        self
    }
}

//--------------------------------------------------------------------------------------------------
// Shared utils between MockBuilder & MockEnv
//--------------------------------------------------------------------------------------------------

fn deploy_nft_contract(app: &mut App, owner: &Addr) -> Addr {
    let nft_contract_code_id = app.store_code(mock_account_nft_contract());
    app.instantiate_contract(
        nft_contract_code_id,
        owner.clone(),
        &NftInstantiateMsg {
            name: "Rover Credit Account".to_string(),
            symbol: "RCA".to_string(),
            minter: owner.to_string(),
        },
        &[],
        "manager-mock-account-nft",
        None,
    )
    .unwrap()
}

fn propose_new_nft_contract_owner(
    app: &mut App,
    nft_contract: Addr,
    nft_contract_owner: &Addr,
    rover: &Addr,
) {
    let proposal_msg: NftExecuteMsg = NftExecuteMsg::ProposeNewOwner {
        new_owner: rover.to_string(),
    };
    app.execute_contract(nft_contract_owner.clone(), nft_contract, &proposal_msg, &[])
        .unwrap();
}
