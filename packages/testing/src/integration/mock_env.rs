#![allow(dead_code)]

use std::{collections::HashMap, default::Default, mem::take, str::FromStr};

use anyhow::Result as AnyResult;
use astroport_v5::incentives::InputSchedule;
use cosmwasm_std::{coin, Addr, Coin, Decimal, Empty, StdResult, Uint128};
use cw_multi_test::{App, AppResponse, BankSudo, BasicApp, Executor, SudoMsg};
use cw_paginate::PaginationResponse;
use mars_oracle_osmosis::OsmosisPriceSourceUnchecked;
use mars_types::{
    address_provider::{self, MarsAddressType},
    credit_manager::ActionCoin,
    incentives,
    oracle::{
        self,
        ActionKind::{Default as ActionDefault, Liquidation},
        PriceResponse,
    },
    params::{AssetParams, AssetParamsUpdate, TotalDepositResponse},
    red_bank::{
        self, CreateOrUpdateConfig, InitOrUpdateAssetParams, Market, MarketV2Response,
        UserCollateralResponse, UserDebtResponse, UserPositionResponse,
    },
    rewards_collector,
};
use pyth_sdk_cw::PriceIdentifier;

use super::mock_contracts::mock_astroport_incentives;
use crate::integration::mock_contracts::{
    mock_address_provider_contract, mock_incentives_contract, mock_oracle_osmosis_contract,
    mock_params_osmosis_contract, mock_pyth_contract, mock_red_bank_contract,
    mock_rewards_collector_osmosis_contract,
};

pub struct MockEnv {
    pub app: App,
    pub owner: Addr,
    pub address_provider: AddressProvider,
    pub astro_incentives: AstroIncentives,
    pub incentives: Incentives,
    pub oracle: Oracle,
    pub red_bank: RedBank,
    pub rewards_collector: RewardsCollector,
    pub params: Params,
    pub credit_manager: Addr,
    pub pyth: Addr,
}

#[derive(Clone)]
pub struct AddressProvider {
    pub contract_addr: Addr,
}

#[derive(Clone)]
pub struct AstroIncentives {
    pub contract_addr: Addr,
}

#[derive(Clone)]
pub struct Incentives {
    pub contract_addr: Addr,
}

#[derive(Clone)]
pub struct Oracle {
    pub contract_addr: Addr,
}

#[derive(Clone)]
pub struct RedBank {
    pub contract_addr: Addr,
}

#[derive(Clone)]
pub struct RewardsCollector {
    pub contract_addr: Addr,
}

#[derive(Clone)]
pub struct Params {
    pub contract_addr: Addr,
}

impl MockEnv {
    pub fn increment_by_blocks(&mut self, num_of_blocks: u64) {
        self.app.update_block(|block| {
            block.height += num_of_blocks;
            // assume block time = 6 sec
            block.time = block.time.plus_seconds(num_of_blocks * 6);
        })
    }

    pub fn increment_by_time(&mut self, seconds: u64) {
        self.app.update_block(|block| {
            block.height += seconds / 6;
            // assume block time = 6 sec
            block.time = block.time.plus_seconds(seconds);
        })
    }

    pub fn fund_accounts(&mut self, addrs: &[&Addr], amount: u128, denoms: &[&str]) {
        for addr in addrs {
            let coins: Vec<_> = denoms.iter().map(|&d| coin(amount, d)).collect();
            self.fund_account(addr, &coins);
        }
    }

    pub fn fund_account(&mut self, addr: &Addr, coins: &[Coin]) {
        self.app
            .sudo(SudoMsg::Bank(BankSudo::Mint {
                to_address: addr.to_string(),
                amount: coins.to_vec(),
            }))
            .unwrap();
    }

    pub fn query_balance(&self, addr: &Addr, denom: &str) -> StdResult<Coin> {
        self.app.wrap().query_balance(addr, denom)
    }

    pub fn query_all_balances(&self, addr: &Addr) -> HashMap<String, Uint128> {
        let res: Vec<Coin> = self.app.wrap().query_all_balances(addr).unwrap();
        res.into_iter().map(|r| (r.denom, r.amount)).collect()
    }
}

impl AstroIncentives {
    pub fn set_incentive_schedule(
        &self,
        env: &mut MockEnv,
        lp_denom: &str,
        incentive_schedule: &InputSchedule,
        funds: Vec<Coin>,
    ) {
        env.app
            .execute_contract(
                env.owner.clone(),
                self.contract_addr.clone(),
                &astroport_v5::incentives::ExecuteMsg::Incentivize {
                    lp_token: lp_denom.to_string(),
                    schedule: incentive_schedule.clone(),
                },
                &funds,
            )
            .unwrap();
    }
}

impl Incentives {
    pub fn whitelist_incentive_denoms(&self, env: &mut MockEnv, incentive_denoms: &[(&str, u128)]) {
        env.app
            .execute_contract(
                env.owner.clone(),
                self.contract_addr.clone(),
                &incentives::ExecuteMsg::UpdateWhitelist {
                    add_denoms: incentive_denoms.iter().map(|x| x.into()).collect(),
                    remove_denoms: vec![],
                },
                &[],
            )
            .unwrap();
    }

    pub fn init_asset_incentive_from_current_block(
        &self,
        env: &mut MockEnv,
        collateral_denom: &str,
        incentive_denom: &str,
        emission_per_second: u128,
        duration: u64,
    ) {
        let current_block_time = env.app.block_info().time.seconds();
        let funds = [coin(emission_per_second * duration as u128, incentive_denom)];
        env.fund_account(&env.owner.clone(), &funds);
        env.app
            .execute_contract(
                env.owner.clone(),
                self.contract_addr.clone(),
                &incentives::ExecuteMsg::SetAssetIncentive {
                    collateral_denom: collateral_denom.to_string(),
                    incentive_denom: incentive_denom.to_string(),
                    emission_per_second: emission_per_second.into(),
                    start_time: current_block_time,
                    duration,
                },
                &funds,
            )
            .unwrap();
    }

    pub fn init_asset_incentive(
        &self,
        env: &mut MockEnv,
        collateral_denom: &str,
        incentive_denom: &str,
        emission_per_second: u128,
        start_time: u64,
        duration: u64,
    ) {
        env.app
            .execute_contract(
                env.owner.clone(),
                self.contract_addr.clone(),
                &incentives::ExecuteMsg::SetAssetIncentive {
                    collateral_denom: collateral_denom.to_string(),
                    incentive_denom: incentive_denom.to_string(),
                    emission_per_second: emission_per_second.into(),
                    start_time,
                    duration,
                },
                &[],
            )
            .unwrap();
    }

    pub fn claim_rewards(&self, env: &mut MockEnv, sender: &Addr) -> AnyResult<AppResponse> {
        self.claim_rewards_with_acc_id(env, sender, None)
    }

    pub fn claim_rewards_with_acc_id(
        &self,
        env: &mut MockEnv,
        sender: &Addr,
        account_id: Option<String>,
    ) -> AnyResult<AppResponse> {
        env.app.execute_contract(
            sender.clone(),
            self.contract_addr.clone(),
            &incentives::ExecuteMsg::ClaimRewards {
                account_id,
                start_after_collateral_denom: None,
                start_after_incentive_denom: None,
                limit: None,
            },
            &[],
        )
    }

    pub fn claim_astro_rewards(
        &self,
        env: &mut MockEnv,
        sender: &Addr,
        account_id: String,
        lp_denom: &str,
    ) -> AnyResult<AppResponse> {
        env.app.execute_contract(
            sender.clone(),
            self.contract_addr.clone(),
            &incentives::ExecuteMsg::ClaimStakedAstroLpRewards {
                account_id,
                lp_denom: lp_denom.to_string(),
            },
            &[],
        )
    }

    pub fn stake_astro_lp(
        &self,
        env: &mut MockEnv,
        sender: &Addr,
        account_id: String,
        lp_coin: Coin,
    ) {
        env.app
            .execute_contract(
                sender.clone(),
                self.contract_addr.clone(),
                &incentives::ExecuteMsg::StakeAstroLp {
                    account_id,
                    lp_coin: lp_coin.clone(),
                },
                &[lp_coin],
            )
            .unwrap();
    }

    pub fn unstake_astro_lp(
        &self,
        env: &mut MockEnv,
        sender: &Addr,
        account_id: String,
        lp_coin: Coin,
    ) {
        env.app
            .execute_contract(
                sender.clone(),
                self.contract_addr.clone(),
                &incentives::ExecuteMsg::UnstakeAstroLp {
                    account_id,
                    lp_coin: ActionCoin::from(&lp_coin.clone()),
                },
                &[],
            )
            .unwrap();
    }

    pub fn query_unclaimed_rewards(&self, env: &mut MockEnv, user: &Addr) -> StdResult<Vec<Coin>> {
        self.query_unclaimed_rewards_with_acc_id(env, user, None)
    }

    pub fn query_unclaimed_rewards_with_acc_id(
        &self,
        env: &mut MockEnv,
        user: &Addr,
        account_id: Option<String>,
    ) -> StdResult<Vec<Coin>> {
        env.app.wrap().query_wasm_smart(
            self.contract_addr.clone(),
            &incentives::QueryMsg::UserUnclaimedRewards {
                account_id,
                user: user.to_string(),
                start_after_collateral_denom: None,
                start_after_incentive_denom: None,
                limit: None,
            },
        )
    }

    pub fn query_unclaimed_astroport_rewards(
        &self,
        env: &MockEnv,
        account_id: String,
        lp_denom: &str,
    ) -> StdResult<Vec<Coin>> {
        env.app.wrap().query_wasm_smart(
            self.contract_addr.clone(),
            &incentives::QueryMsg::StakedAstroLpRewards {
                account_id,
                lp_denom: lp_denom.to_string(),
            },
        )
    }
}

impl Oracle {
    pub fn set_price_source_fixed(&self, env: &mut MockEnv, denom: &str, price: Decimal) {
        env.app
            .execute_contract(
                env.owner.clone(),
                self.contract_addr.clone(),
                &oracle::ExecuteMsg::<_, Empty>::SetPriceSource {
                    denom: denom.to_string(),
                    price_source: OsmosisPriceSourceUnchecked::Fixed {
                        price,
                    },
                },
                &[],
            )
            .unwrap();
    }

    pub fn set_price_source_pyth(
        &self,
        env: &mut MockEnv,
        denom: &str,
        pyth_addr: String,
        max_confidence: Decimal,
        max_deviation: Decimal,
    ) {
        env.app
            .execute_contract(
                env.owner.clone(),
                self.contract_addr.clone(),
                &oracle::ExecuteMsg::<_, Empty>::SetPriceSource {
                    denom: denom.to_string(),
                    price_source: OsmosisPriceSourceUnchecked::Pyth {
                        contract_addr: pyth_addr,
                        price_feed_id: PriceIdentifier::from_hex(
                            "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
                        )
                        .unwrap(),
                        max_staleness: 30u64,
                        max_confidence,
                        max_deviation,
                        denom_decimals: 6u8,
                    },
                },
                &[],
            )
            .unwrap();
    }

    pub fn query_price(&self, env: &mut MockEnv, denom: &str) -> PriceResponse {
        env.app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &oracle::QueryMsg::Price {
                    denom: denom.to_string(),
                    kind: Some(ActionDefault),
                },
            )
            .unwrap()
    }

    pub fn query_price_for_liquidate(&self, env: &mut MockEnv, denom: &str) -> PriceResponse {
        env.app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &oracle::QueryMsg::Price {
                    denom: denom.to_string(),
                    kind: Some(Liquidation),
                },
            )
            .unwrap()
    }
}

impl RedBank {
    pub fn init_asset(&self, env: &mut MockEnv, denom: &str, params: InitOrUpdateAssetParams) {
        env.app
            .execute_contract(
                env.owner.clone(),
                self.contract_addr.clone(),
                &red_bank::ExecuteMsg::InitAsset {
                    denom: denom.to_string(),
                    params,
                },
                &[],
            )
            .unwrap();
    }

    pub fn update_user_collateral_status(
        &self,
        env: &mut MockEnv,
        sender: &Addr,
        denom: &str,
        enabled: bool,
    ) {
        env.app
            .execute_contract(
                sender.clone(),
                self.contract_addr.clone(),
                &red_bank::ExecuteMsg::UpdateAssetCollateralStatus {
                    denom: denom.to_string(),
                    enable: enabled,
                },
                &[],
            )
            .unwrap();
    }

    pub fn deposit(&self, env: &mut MockEnv, sender: &Addr, coin: Coin) -> AnyResult<AppResponse> {
        self.deposit_with_acc_id(env, sender, coin, None)
    }

    pub fn deposit_with_acc_id(
        &self,
        env: &mut MockEnv,
        sender: &Addr,
        coin: Coin,
        account_id: Option<String>,
    ) -> AnyResult<AppResponse> {
        env.app.execute_contract(
            sender.clone(),
            self.contract_addr.clone(),
            &red_bank::ExecuteMsg::Deposit {
                account_id,
                on_behalf_of: None,
            },
            &[coin],
        )
    }

    pub fn borrow(
        &self,
        env: &mut MockEnv,
        sender: &Addr,
        denom: &str,
        amount: u128,
    ) -> AnyResult<AppResponse> {
        env.app.execute_contract(
            sender.clone(),
            self.contract_addr.clone(),
            &red_bank::ExecuteMsg::Borrow {
                denom: denom.to_string(),
                amount: amount.into(),
                recipient: None,
            },
            &[],
        )
    }

    pub fn repay(&self, env: &mut MockEnv, sender: &Addr, coin: Coin) -> AnyResult<AppResponse> {
        env.app.execute_contract(
            sender.clone(),
            self.contract_addr.clone(),
            &red_bank::ExecuteMsg::Repay {
                on_behalf_of: None,
            },
            &[coin],
        )
    }

    pub fn withdraw(
        &self,
        env: &mut MockEnv,
        sender: &Addr,
        denom: &str,
        amount: Option<Uint128>,
    ) -> AnyResult<AppResponse> {
        self.withdraw_with_acc_id(env, sender, denom, amount, None, None)
    }

    pub fn withdraw_with_acc_id(
        &self,
        env: &mut MockEnv,
        sender: &Addr,
        denom: &str,
        amount: Option<Uint128>,
        account_id: Option<String>,
        liquidation_related: Option<bool>,
    ) -> AnyResult<AppResponse> {
        env.app.execute_contract(
            sender.clone(),
            self.contract_addr.clone(),
            &red_bank::ExecuteMsg::Withdraw {
                denom: denom.to_string(),
                amount,
                recipient: None,
                account_id,
                liquidation_related,
            },
            &[],
        )
    }

    pub fn liquidate(
        &self,
        env: &mut MockEnv,
        liquidator: &Addr,
        user: &Addr,
        collateral_denom: &str,
        send_funds: &[Coin],
    ) -> AnyResult<AppResponse> {
        self.liquidate_with_different_recipient(
            env,
            liquidator,
            user,
            collateral_denom,
            send_funds,
            None,
        )
    }

    pub fn liquidate_with_different_recipient(
        &self,
        env: &mut MockEnv,
        liquidator: &Addr,
        user: &Addr,
        collateral_denom: &str,
        send_funds: &[Coin],
        recipient: Option<String>,
    ) -> AnyResult<AppResponse> {
        env.app.execute_contract(
            liquidator.clone(),
            self.contract_addr.clone(),
            &red_bank::ExecuteMsg::Liquidate {
                user: user.to_string(),
                collateral_denom: collateral_denom.to_string(),
                recipient,
            },
            send_funds,
        )
    }

    pub fn query_market(&self, env: &mut MockEnv, denom: &str) -> Market {
        env.app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &red_bank::QueryMsg::Market {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_market_v2(&self, env: &mut MockEnv, denom: &str) -> MarketV2Response {
        env.app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &red_bank::QueryMsg::MarketV2 {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_markets(&self, env: &mut MockEnv) -> HashMap<String, Market> {
        let res: Vec<Market> = env
            .app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &red_bank::QueryMsg::Markets {
                    start_after: None,
                    limit: Some(100),
                },
            )
            .unwrap();
        res.into_iter().map(|r| (r.denom.clone(), r)).collect()
    }

    pub fn query_user_debt(&self, env: &mut MockEnv, user: &Addr, denom: &str) -> UserDebtResponse {
        env.app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &red_bank::QueryMsg::UserDebt {
                    user: user.to_string(),
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_user_debts(
        &self,
        env: &mut MockEnv,
        user: &Addr,
    ) -> HashMap<String, UserDebtResponse> {
        let res: Vec<UserDebtResponse> = env
            .app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &red_bank::QueryMsg::UserDebts {
                    user: user.to_string(),
                    start_after: None,
                    limit: Some(100),
                },
            )
            .unwrap();
        res.into_iter().map(|r| (r.denom.clone(), r)).collect()
    }

    pub fn query_user_collateral(
        &self,
        env: &mut MockEnv,
        user: &Addr,
        denom: &str,
    ) -> UserCollateralResponse {
        self.query_user_collateral_with_acc_id(env, user, None, denom)
    }

    pub fn query_user_collateral_with_acc_id(
        &self,
        env: &mut MockEnv,
        user: &Addr,
        account_id: Option<String>,
        denom: &str,
    ) -> UserCollateralResponse {
        env.app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &red_bank::QueryMsg::UserCollateral {
                    user: user.to_string(),
                    account_id,
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn query_user_collaterals(
        &self,
        env: &mut MockEnv,
        user: &Addr,
    ) -> HashMap<String, UserCollateralResponse> {
        self.query_user_collaterals_with_acc_id(env, user, None)
    }

    pub fn query_user_collaterals_with_acc_id(
        &self,
        env: &mut MockEnv,
        user: &Addr,
        account_id: Option<String>,
    ) -> HashMap<String, UserCollateralResponse> {
        let res: Vec<UserCollateralResponse> = env
            .app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &red_bank::QueryMsg::UserCollaterals {
                    user: user.to_string(),
                    account_id,
                    start_after: None,
                    limit: Some(100),
                },
            )
            .unwrap();
        res.into_iter().map(|r| (r.denom.clone(), r)).collect()
    }

    pub fn query_user_position(&self, env: &mut MockEnv, user: &Addr) -> UserPositionResponse {
        self.query_user_position_with_acc_id(env, user, None)
    }

    pub fn query_user_position_with_acc_id(
        &self,
        env: &mut MockEnv,
        user: &Addr,
        account_id: Option<String>,
    ) -> UserPositionResponse {
        env.app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &red_bank::QueryMsg::UserPosition {
                    user: user.to_string(),
                    account_id,
                },
            )
            .unwrap()
    }

    pub fn query_scaled_liquidity_amount(&self, env: &mut MockEnv, coin: Coin) -> Uint128 {
        env.app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &red_bank::QueryMsg::ScaledLiquidityAmount {
                    denom: coin.denom,
                    amount: coin.amount,
                },
            )
            .unwrap()
    }

    pub fn query_scaled_debt_amount(&self, env: &mut MockEnv, coin: Coin) -> Uint128 {
        env.app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &red_bank::QueryMsg::ScaledDebtAmount {
                    denom: coin.denom,
                    amount: coin.amount,
                },
            )
            .unwrap()
    }
}

impl RewardsCollector {
    pub fn withdraw_from_red_bank(&self, env: &mut MockEnv, denom: &str, amount: Option<Uint128>) {
        env.app
            .execute_contract(
                Addr::unchecked("anyone"),
                self.contract_addr.clone(),
                &mars_types::rewards_collector::ExecuteMsg::WithdrawFromRedBank {
                    denom: denom.to_string(),
                    amount,
                },
                &[],
            )
            .unwrap();
    }

    pub fn claim_incentive_rewards(&self, env: &mut MockEnv) -> AnyResult<AppResponse> {
        env.app.execute_contract(
            Addr::unchecked("anyone"),
            self.contract_addr.clone(),
            &mars_types::rewards_collector::ExecuteMsg::ClaimIncentiveRewards {
                start_after_collateral_denom: None,
                start_after_incentive_denom: None,
                limit: None,
            },
            &[],
        )
    }
}

impl Params {
    pub fn init_params(&self, env: &mut MockEnv, params: AssetParams) {
        env.app
            .execute_contract(
                env.owner.clone(),
                self.contract_addr.clone(),
                &mars_types::params::ExecuteMsg::UpdateAssetParams(
                    AssetParamsUpdate::AddOrUpdate {
                        params: params.into(),
                    },
                ),
                &[],
            )
            .unwrap();
    }

    pub fn query_params(&self, env: &mut MockEnv, denom: &str) -> AssetParams {
        env.app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &mars_types::params::QueryMsg::AssetParams {
                    denom: denom.to_string(),
                },
            )
            .unwrap()
    }

    pub fn all_total_deposits_v2(
        &self,
        env: &mut MockEnv,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> PaginationResponse<TotalDepositResponse> {
        env.app
            .wrap()
            .query_wasm_smart(
                self.contract_addr.clone(),
                &mars_types::params::QueryMsg::AllTotalDepositsV2 {
                    start_after,
                    limit,
                },
            )
            .unwrap()
    }
}

pub struct MockEnvBuilder {
    app: BasicApp,
    admin: Option<String>,
    owner: Addr,

    chain_prefix: String,
    mars_denom: String,
    base_denom: String,
    base_denom_decimals: u8,
    target_health_factor: Decimal,

    // rewards-collector params
    safety_tax_rate: Decimal,
    safety_fund_denom: String,
    fee_collector_denom: String,
    slippage_tolerance: Decimal,

    pyth_contract_addr: String,

    credit_manager_contract_addr: String,
}

impl MockEnvBuilder {
    pub fn new(admin: Option<String>, owner: Addr) -> Self {
        Self {
            app: App::default(),
            admin,
            owner,
            chain_prefix: "".to_string(), // empty prefix for multitest because deployed contracts have addresses such as contract1, contract2 etc which are invalid in address-provider
            mars_denom: "umars".to_string(),
            base_denom: "uosmo".to_string(),
            base_denom_decimals: 6u8,
            target_health_factor: Decimal::from_str("1.05").unwrap(),
            safety_tax_rate: Decimal::percent(50),
            safety_fund_denom: "uusdc".to_string(),
            fee_collector_denom: "uusdc".to_string(),
            slippage_tolerance: Decimal::percent(5),
            pyth_contract_addr: "osmo1svg55quy7jjee6dn0qx85qxxvx5cafkkw4tmqpcjr9dx99l0zrhs4usft5"
                .to_string(), // correct bech32 addr to pass validation
            credit_manager_contract_addr:
                "osmo1q7khj532p2fyvmnu83tul6xddl6yl0d0kmrzdz2pfel3lkxem92sw6zqrl".to_string(),
        }
    }

    pub fn chain_prefix(&mut self, prefix: &str) -> &mut Self {
        self.chain_prefix = prefix.to_string();
        self
    }

    pub fn mars_denom(&mut self, denom: &str) -> &mut Self {
        self.mars_denom = denom.to_string();
        self
    }

    pub fn base_denom(&mut self, denom: &str) -> &mut Self {
        self.base_denom = denom.to_string();
        self
    }

    pub fn target_health_factor(&mut self, thf: Decimal) -> &mut Self {
        self.target_health_factor = thf;
        self
    }

    pub fn safety_tax_rate(&mut self, percentage: Decimal) -> &mut Self {
        self.safety_tax_rate = percentage;
        self
    }

    pub fn safety_fund_denom(&mut self, denom: &str) -> &mut Self {
        self.safety_fund_denom = denom.to_string();
        self
    }

    pub fn fee_collector_denom(&mut self, denom: &str) -> &mut Self {
        self.fee_collector_denom = denom.to_string();
        self
    }

    pub fn slippage_tolerance(&mut self, percentage: Decimal) -> &mut Self {
        self.slippage_tolerance = percentage;
        self
    }

    pub fn pyth_contract_addr(&mut self, pyth_contract_addr: Addr) -> &mut Self {
        self.pyth_contract_addr = pyth_contract_addr.to_string();
        self
    }

    pub fn build(&mut self) -> MockEnv {
        let address_provider_addr = self.deploy_address_provider();
        let incentives_addr = self.deploy_incentives(&address_provider_addr);
        let oracle_addr = self.deploy_oracle_osmosis();
        let red_bank_addr = self.deploy_red_bank(&address_provider_addr);
        let rewards_collector_addr = self.deploy_rewards_collector_osmosis(&address_provider_addr);
        let params_addr = self.deploy_params_osmosis(&address_provider_addr);
        let pyth_addr = self.deploy_mock_pyth();
        let astroport_incentives_addr = self.deploy_mock_astroport_incentives();

        self.update_address_provider(
            &address_provider_addr,
            MarsAddressType::Incentives,
            &incentives_addr,
        );
        self.update_address_provider(&address_provider_addr, MarsAddressType::Oracle, &oracle_addr);
        self.update_address_provider(
            &address_provider_addr,
            MarsAddressType::RedBank,
            &red_bank_addr,
        );
        self.update_address_provider(
            &address_provider_addr,
            MarsAddressType::RewardsCollector,
            &rewards_collector_addr,
        );
        self.update_address_provider(&address_provider_addr, MarsAddressType::Params, &params_addr);
        let cm_addr = Addr::unchecked(&self.credit_manager_contract_addr);
        self.update_address_provider(
            &address_provider_addr,
            MarsAddressType::CreditManager,
            &cm_addr,
        );
        self.update_address_provider(
            &address_provider_addr,
            MarsAddressType::AstroportIncentives,
            &astroport_incentives_addr,
        );

        MockEnv {
            app: take(&mut self.app),
            owner: self.owner.clone(),
            address_provider: AddressProvider {
                contract_addr: address_provider_addr,
            },
            astro_incentives: AstroIncentives {
                contract_addr: astroport_incentives_addr,
            },
            incentives: Incentives {
                contract_addr: incentives_addr,
            },
            oracle: Oracle {
                contract_addr: oracle_addr,
            },
            red_bank: RedBank {
                contract_addr: red_bank_addr,
            },
            rewards_collector: RewardsCollector {
                contract_addr: rewards_collector_addr,
            },
            params: Params {
                contract_addr: params_addr,
            },
            credit_manager: cm_addr,
            pyth: pyth_addr,
        }
    }

    fn deploy_address_provider(&mut self) -> Addr {
        let code_id = self.app.store_code(mock_address_provider_contract());

        self.app
            .instantiate_contract(
                code_id,
                self.owner.clone(),
                &address_provider::InstantiateMsg {
                    owner: self.owner.to_string(),
                    prefix: self.chain_prefix.clone(),
                },
                &[],
                "address-provider",
                None,
            )
            .unwrap()
    }

    fn deploy_incentives(&mut self, address_provider_addr: &Addr) -> Addr {
        let code_id = self.app.store_code(mock_incentives_contract());

        self.app
            .instantiate_contract(
                code_id,
                self.owner.clone(),
                &incentives::InstantiateMsg {
                    owner: self.owner.to_string(),
                    address_provider: address_provider_addr.to_string(),
                    epoch_duration: 604800,
                    max_whitelisted_denoms: 10,
                },
                &[],
                "incentives",
                None,
            )
            .unwrap()
    }

    fn deploy_oracle_osmosis(&mut self) -> Addr {
        let code_id = self.app.store_code(mock_oracle_osmosis_contract());

        self.app
            .instantiate_contract(
                code_id,
                self.owner.clone(),
                &oracle::InstantiateMsg::<Empty> {
                    owner: self.owner.to_string(),
                    base_denom: self.base_denom.clone(),
                    custom_init: None,
                },
                &[],
                "oracle",
                None,
            )
            .unwrap()
    }

    fn deploy_red_bank(&mut self, address_provider_addr: &Addr) -> Addr {
        let code_id = self.app.store_code(mock_red_bank_contract());

        self.app
            .instantiate_contract(
                code_id,
                self.owner.clone(),
                &red_bank::InstantiateMsg {
                    owner: self.owner.to_string(),
                    config: CreateOrUpdateConfig {
                        address_provider: Some(address_provider_addr.to_string()),
                    },
                },
                &[],
                "red-bank",
                None,
            )
            .unwrap()
    }

    fn deploy_rewards_collector_osmosis(&mut self, address_provider_addr: &Addr) -> Addr {
        let code_id = self.app.store_code(mock_rewards_collector_osmosis_contract());

        self.app
            .instantiate_contract(
                code_id,
                self.owner.clone(),
                &rewards_collector::InstantiateMsg {
                    owner: self.owner.to_string(),
                    address_provider: address_provider_addr.to_string(),
                    safety_tax_rate: self.safety_tax_rate,
                    safety_fund_denom: self.safety_fund_denom.clone(),
                    fee_collector_denom: self.fee_collector_denom.clone(),
                    channel_id: "0".to_string(),
                    timeout_seconds: 900,
                    slippage_tolerance: self.slippage_tolerance,
                    neutron_ibc_config: None,
                },
                &[],
                "rewards-collector",
                None,
            )
            .unwrap()
    }

    fn deploy_params_osmosis(&mut self, address_provider_addr: &Addr) -> Addr {
        let code_id = self.app.store_code(mock_params_osmosis_contract());

        self.app
            .instantiate_contract(
                code_id,
                self.owner.clone(),
                &mars_types::params::InstantiateMsg {
                    owner: self.owner.to_string(),
                    address_provider: address_provider_addr.to_string(),
                    target_health_factor: self.target_health_factor,
                },
                &[],
                "params",
                None,
            )
            .unwrap()
    }

    pub fn deploy_mock_pyth(&mut self) -> Addr {
        let code_id = self.app.store_code(mock_pyth_contract());

        self.app
            .instantiate_contract(code_id, self.owner.clone(), &Empty {}, &[], "mock-pyth", None)
            .unwrap()
    }

    pub fn deploy_mock_astroport_incentives(&mut self) -> Addr {
        let code_id = self.app.store_code(mock_astroport_incentives());

        self.app
            .instantiate_contract(
                code_id,
                self.owner.clone(),
                &Empty {},
                &[],
                "mock-astroport-incentives",
                None,
            )
            .unwrap()
    }

    fn update_address_provider(
        &mut self,
        address_provider_addr: &Addr,
        address_type: MarsAddressType,
        addr: &Addr,
    ) {
        self.app
            .execute_contract(
                self.owner.clone(),
                address_provider_addr.clone(),
                &address_provider::ExecuteMsg::SetAddress {
                    address_type,
                    address: addr.to_string(),
                },
                &[],
            )
            .unwrap();
    }
}
