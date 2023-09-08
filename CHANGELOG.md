# Changelog

All notable changes to this project will be documented in this file.

## v1.2.0

- Allow Credit account to lend/reclaim to the Red Bank (calls Deposit/Withdraw in Red Bank), claim incentive rewards from lending to the Red Bank (pass account_id to track Credit Manager users in `red-bank` and `incentives` contract).
- Pyth oracle price sourcing support for EMA, confidence. Different price query for liquidation and other actions (Deposit / Borrow / Repay etc.).
- New liquidation mechanism on Red Bank and Credit Manager to allow variable liquidation bonus and close factors (similar to Euler model).
- Previously Red Bank and Credit Manager had separate deposit caps to protect against holding to many assets in relation to chain liquidity - consolidated these deposit caps into one `params` contract.
- Common `swapper` contract for Red Bank and Credit Manager.
- Credit Manager and Red Bank use `params` contract for common asset params (see new [types](/contracts/params/src/types)). Previous `Market` / `Markets` queries has changed - part of the params are moved to `params` contract (see [diff](./files/types_diff_v1_0_0__mars_v2.txt)).

## v1.0.0-rc0

This section documents the API changes compared to the Terra Classic deployment, found in the [`mars-core`](https://github.com/mars-protocol/mars-core) repository. This section is **not comprehensive**, as the changes are numerous. Changelog for later version start here should be made comprehensive.

- ([#81](https://github.com/mars-protocol/red-bank/pull/79/files)) Red Bank: The `total_debt_value` field in `UserPositionResponse` is removed, as previously the calculation of this value was erroneous. Additionally, `total_collateral_value` is renamed to `total_enabled_collateral`.

```diff
struct UserPositionResponse {
-   pub total_collateral_value: Decimal,
+   pub total_enabled_collateral: Decimal,
-   pub total_debt_value: Decimal,
    pub total_collateralized_debt: Decimal,
    pub weighted_max_ltv_collateral: Decimal,
    pub weighted_liquidation_threshold_collateral: Decimal,
    pub health_status: UserHealthStatus,
}
```

- ([#79](https://github.com/mars-protocol/red-bank/pull/79/files)) Incentives: The `user_address` parameter in `QueryMsg::UserUnclaimedRewards` is renamed to just `user` in accordance with the [coding guildelines](./CODING_GUIDELINES.md):

```diff
enum QueryMsg {
    UserUnclaimedRewards {
-       user_address: String,
+       user: String,
    },
}
```

- ([#79](https://github.com/mars-protocol/red-bank/pull/79/files)) Incentives: "maToken address" is replaced with the asset denom in the execute and query messages related to asset incentive:

```diff
enum ExecuteMsg {
    SetAssetIncentive {
-       ma_token_address: String,
+       denom: String,
        emission_per_second: Uint128,
    },
}

enum QueryMsg {
    AssetIncentive {
-       ma_token_address: String,
+       denom: String,
    },
}
```

- ([#79](https://github.com/mars-protocol/red-bank/pull/79/files)) Incentives: A new `address_provider` field is added to the instantiate message and config response:

```diff
struct InstantiateMsg {
    pub owner: String,
+   pub address_provider: String,
    pub mars_denom: String,
}

struct Config {
    pub owner: Addr,
+   pub address_provider: Addr,
    pub mars_denom: String,
}
```

- ([#76](https://github.com/mars-protocol/red-bank/pull/76/files)) Red Bank: Execute messages for creating and updating markets have been simplified:

```diff
enum ExecuteMsg {
    InitAsset {
        denom: String,
-       asset_params: InitOrUpdateAssetParams,
-       asset_symbol: Option<String>,
+       params: InitOrUpdateAssetParams,
    },
    UpdateAsset {
        denom: String,
-       asset_params: InitOrUpdateAssetParams,
+       params: InitOrUpdateAssetParams,
    },
}
```

- ([#76](https://github.com/mars-protocol/red-bank/pull/76/files)) Red Bank: `underlying_liquidity_amount` query now takes the asset's denom instead of the maToken contract address:

```diff
enum QueryMsg {
    UnderlyingLiquidityAmount {
-       ma_token_address: String,
+       denom: String,
        amount_scaled: Uint128,
    },
}
```

- ([#76](https://github.com/mars-protocol/red-bank/pull/76/files)) Red Bank: `market` query now no longer returns the maToken address. Additionally, it now returns the total scaled amount of collateral. The frontend should now query this method instead of the maToken's total supply:

```diff
struct Market {
    pub denom: String,
-   pub ma_token_address: Addr,
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
    pub liquidation_bonus: Decimal,
    pub reserve_factor: Decimal,
    pub interest_rate_model: InterestRateModel,
    pub borrow_index: Decimal,
    pub liquidity_index: Decimal,
    pub borrow_rate: Decimal,
    pub liquidity_rate: Decimal,
    pub indexes_last_updated: u64,
+   pub collateral_total_scaled: Uint128,
    pub debt_total_scaled: Uint128,
    pub deposit_enabled: bool,
    pub borrow_enabled: bool,
    pub deposit_cap: Uint128,
}
```

- ([#76](https://github.com/mars-protocol/red-bank/pull/76/files)) Red Bank: `user_collateral` query now returns the collateral scaled amount and amount. The frontend should now query this method instead of the maToken contracts:

```diff
struct UserCollateralResponse {
    pub denom: String,
+   pub amount_scaled: Uint128,
+   pub amount: Uint128,
    pub enabled: bool,
}
```

- ([#76](https://github.com/mars-protocol/red-bank/pull/76/files)) Red Bank: `user_debt` query now returns whether the debt is being borrowed as uncollateralized loan:

```diff
struct UserDebtResponse {
    pub denom: String,
    pub amount_scaled: Uint128,
    pub amount: Uint128,
+   pub uncollateralized: bool,
}
```

- ([#76](https://github.com/mars-protocol/red-bank/pull/76/files)) Red Bank: Parameters related to maToken have been removed from instantiate message, the `update_config` execute message, and the response type for config query:

```diff
struct CreateOrUpdateConfig {
    pub owner: Option<String>,
    pub address_provider: Option<String>,
-   pub ma_token_code_id: Option<u64>,
    pub close_factor: Option<Decimal>,
}

struct Config {
    pub owner: Addr,
    pub address_provider: Addr,
-   pub ma_token_code_id: u64,
    pub close_factor: Decimal,
}
```

- ([#69](https://github.com/mars-protocol/red-bank/pull/69/files)) Red Bank: `Market` no longer includes an index:

```diff
struct Market {
-   pub index: u64,
    pub denom: String,
    pub ma_token_address: Addr,
    // ....
}
```

- ([#69](https://github.com/mars-protocol/red-bank/pull/69/files)) Red Bank: The total number of markets is no longer returned in `ConfigResponse`:

```diff
struct ConfigResponse {
    pub owner: String,
    pub address_provider: String,
    pub ma_token_code_id: u64,
    pub close_factor: Decimal,
-   pub market_count: u64,
}
```

- ([#69](https://github.com/mars-protocol/red-bank/pull/69/files)) Red Bank: Previously, the enumerative queries `user_debts` and `user_collaterals` return the user's debt or collateral amounts in **every single asset that Red Bank supports**. Now they will only return the assets **for which the user has non-zero amounts deposited or borrowed**.

Additionally, the two queries now support pagination:

```diff
enum QueryMsg {
    UserDebts {
        user: String,
+       start_after: Option<String>,
+       limit: Option<u32>,
    },
    UserCollaterals {
        user: String,
+       start_after: Option<String>,
+       limit: Option<u32>,
    }
}
```

- ([#63](https://github.com/mars-protocol/red-bank/pull/63)) Red Bank: Rename a few query functions:

| old query        | new query         | description                                    |
| ---------------- | ----------------- | ---------------------------------------------- |
| `MarketList`     | `Markets`         | list all markets                               |
| `UserAssetDebt`  | `UserDebt`        | a user's debt position in a single asset       |
| `UserDebt`       | `UserDebts`       | a user's debt positions in all assets          |
| -                | `UserCollateral`  | a user's collateral position in a single asset |
| `UserCollateral` | `UserCollaterals` | a user's collateral positions in all assets    |

- ([#63](https://github.com/mars-protocol/red-bank/pull/63)) Red Bank: Changes to a few query response types:

Response to `Markets`:

```typescript
// old
type MarketsResponse = {
  market_list: Market[];
};

// new
type MarketResponse = Market[];
```

Response to `UserDebts`:

```typescript
// old
type UserDebtsResponse = {
  debts: UserDebtResponse[];
};

// new
type UserDebtsResponse = UserDebtResponse[];
```

Response to `UserCollaterals`:

```typescript
// old
type UserCollateralsResponse = {
  collaterals: UserCollateralResponse[];
};

// new
type UserCollateralsResponse = UserCollateralResponse[];
```

- ([#61](https://github.com/mars-protocol/red-bank/pull/61)) Red Bank: Implement variable naming convension.

```diff
pub struct CreateOrUpdateConfig {
    pub owner: Option<String>,
-   pub address_provider_address: Option<String>,
+   pub address_provider: Option<String>,
    pub ma_token_code_id: Option<u64>,
    pub close_factor: Option<Decimal>,
}

pub struct ConfigResponse {
    pub owner: String,
-   pub address_provider_address: Addr,
+   pub address_provider: String,
    pub ma_token_code_id: u64,
    pub market_count: u32,
    pub close_factor: Decimal,
}

pub struct ExecuteMsg {
    UpdateUncollateralizedLoanLimit {
-       user_address: String,
+       user: String,
        denom: String,
        new_limit: Uint128,
    },
    Liquidate {
-       user_address: String,
+       user: String,
        collateral_denom: String,
    }
}

pub struct QueryMsg {
    UncollateralizedLoanLimit {
-       user_address: String,
+       user: String,
        denom: String,
    },
    UserDebt {
-       user_address: String,
+       user: String,
    },
    UserAssetDebt {
-       user_address: String,
+       user: String,
        denom: String,
    },
    UserCollateral {
-       user_address: String,
+       user: String,
    },
    UserPosition {
-       user_address: String,
+       user: String,
    },
}
```

- ([#55](https://github.com/mars-protocol/red-bank/pull/55)) Red Bank: the option for the liquidator to request receiving the underlying asset is removed. Now the liquidator always receives collateral shares. To withdraw the underlying asset, dispatch another `ExecuteMsg::Withdraw`.

```diff
pub struct ExecuteMsg {
    Liquidate {
        collateral_denom: String,
        user_address: String,
-       receive_ma_token: bool,
    },
}
```

- ([#53](https://github.com/mars-protocol/red-bank/pull/53)) Red Bank: Several unnecessary parameters in the execute message are removed:

```diff
pub struct ExecuteMsg {
    Deposit {
-       denom: String,
        on_behalf_of: Option<String>,
    },
    Repay {
-       denom: String,
        on_behalf_of: Option<String>,
    },
    Liquidate {
        collateral_denom: String,
-       debt_denom: String,
        user_address: String,
        receive_ma_token: bool, // NOTE: this params is removed as well in PR #55
    },
}
```

- ([#46](https://github.com/mars-protocol/red-bank/pull/46)) Red Bank: the dynamic interest rate model is removed. The `InterestRateModel` struct is simplified:

```diff
- pub enum InterestRateModel {
-     Dynamic {
-         params: DynamicInterestRateModelParams,
-         state: DynamicInterestRateModelState,
-     },
-     Linear {
-         params: LinearInterestRateModelParams,
-     },
- }
+ pub struct InterestRateModel {
+     pub optimal_utilization_rate: Decimal,
+     pub base: Decimal,
+     pub slope_1: Decimal,
+     pub slope_2: Decimal,
+ }
```

The `Market` struct is updated accordingly. Note that this struct is the response type is the response type for the `market` and `markets` queries methods:

```diff
  pub struct Market {
-     pub interest_rate_model: red_bank::InterestRateModel, # old
+     pub interest_rate_model: InterestRateModel,           # new
  }
```

The `InitOrUpdateAssetParams` struct, which is used in `init_asset` and `update_asset` execute messages, is updated accordingly:

```diff
  pub struct InitOrUpdateAssetParams {
-     pub interest_rate_model_params: Option<InterestRateModelParams>,
+     pub interest_rate_model: Option<InterestRateModel>,
  }
```
