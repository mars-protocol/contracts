use cosmwasm_std::{
    to_json_binary, Addr, CosmosMsg, Order, Response, StdResult, Storage, Uint128, WasmMsg,
};
use mars_types::{
    incentives,
    keys::{UserId, UserIdKey},
    red_bank::{Collateral, Debt, Market},
};

use crate::state::{COLLATERALS, DEBTS, UNCOLLATERALIZED_LOAN_LIMITS};

/// A helper class providing an intuitive API for managing user positions in the contract store.
///
/// For example, to increase a user's debt shares, instead of:
///
/// ```rust
/// DEBTS.update(deps.storage, &user_addr, |opt| -> StdResult<_> {
///     let mut debt = opt.unwrap_or_default();
///     debt.amount_scaled = debt.amount_scaled.checked_add(new_debt)?;
///     Ok(debt)
/// })?;
/// ```
///
/// The `User` struct allows you simply do
///
/// ```rust
/// let user = User(&user_addr);
/// user.increase_debt(deps.storage, new_debt)?;
/// ```
#[derive(Clone, Copy)]
pub struct User<'a>(pub &'a Addr);

// Implement Into<String> for User so that it can be easily used in event attributes, e.g.
//
// ```rust
// let user = User(&user_addr);
// let res = Response::new().add_attribute("user", user);
// ```
impl<'a> From<User<'a>> for String {
    fn from(user: User) -> String {
        user.0.to_string()
    }
}

impl<'a> User<'a> {
    /// Returns a reference to the user's address
    pub fn address(&self) -> &Addr {
        self.0
    }

    /// Load the user's collateral
    pub fn collateral(
        &self,
        store: &dyn Storage,
        denom: &str,
        account_id: &str,
    ) -> StdResult<Collateral> {
        let user_id = UserId::credit_manager(self.0.clone(), account_id.to_string());
        let user_id_key: UserIdKey = user_id.try_into()?;
        COLLATERALS.load(store, (&user_id_key, denom))
    }

    /// Load the user's debt
    pub fn debt(&self, store: &dyn Storage, denom: &str) -> StdResult<Debt> {
        DEBTS.load(store, (self.0, denom))
    }

    /// Load the user's scaled debt amount; default to zero if not borrowing.
    pub fn debt_amount_scaled(&self, store: &dyn Storage, denom: &str) -> StdResult<Uint128> {
        let amount_scaled = DEBTS
            .may_load(store, (self.0, denom))?
            .map(|debt| debt.amount_scaled)
            .unwrap_or_else(Uint128::zero);
        Ok(amount_scaled)
    }

    /// Load the user's uncollateralized loan limit. Return zero if the user has not been given an
    /// uncollateralized loan limit.
    pub fn uncollateralized_loan_limit(
        &self,
        store: &dyn Storage,
        denom: &str,
    ) -> StdResult<Uint128> {
        let limit = UNCOLLATERALIZED_LOAN_LIMITS
            .may_load(store, (self.0, denom))?
            .unwrap_or_else(Uint128::zero);
        Ok(limit)
    }

    /// Return `true` if the user is borrowing a non-zero amount in _any_ asset; return `false` if
    /// the user is not borrowing any asset.
    ///
    /// The user is borrowing if, in the `DEBTS` map, there is at least one denom stored under the
    /// user address prefix.
    pub fn is_borrowing(&self, store: &dyn Storage) -> bool {
        DEBTS.prefix(self.0).range(store, None, None, Order::Ascending).next().is_some()
    }

    /// Increase a user's collateral shares by the specified amount. Returns a message to inform the
    /// incentives contract to update the user's index.
    ///
    /// If the user does not already have a collateral amount, the asset is enabled as collateral by
    /// default. To disable, send a separate `update_asset_collateral_status` execute message.
    ///
    /// This may be invoked if a user makes a deposit, or when a liquidator liquidates a position.
    pub fn increase_collateral(
        &self,
        store: &mut dyn Storage,
        market: &Market,
        amount_scaled: Uint128,
        incentives_addr: &Addr,
        response: Response,
        account_id: Option<String>,
    ) -> StdResult<Response> {
        let acc_id = account_id.clone().unwrap_or("".to_string());

        let user_id = UserId::credit_manager(self.0.clone(), acc_id);
        let user_id_key: UserIdKey = user_id.try_into()?;

        let mut amount_scaled_before = Uint128::zero();

        COLLATERALS.update(store, (&user_id_key, &market.denom), |opt| -> StdResult<_> {
            match opt {
                Some(mut col) => {
                    amount_scaled_before = col.amount_scaled;
                    col.amount_scaled = col.amount_scaled.checked_add(amount_scaled)?;
                    Ok(col)
                }
                None => Ok(Collateral {
                    amount_scaled,
                    enabled: true, // enable by default
                }),
            }
        })?;

        let msg = self.build_incentives_balance_changed_msg(
            incentives_addr,
            market,
            amount_scaled_before,
            account_id,
        )?;

        Ok(response.add_message(msg))
    }

    /// Decrease a user's collateral shares by the specified amount. Returns a message to inform the
    /// incentives contract to update the user's index.
    ///
    /// If reduced to zero, delete the collateral position from contract storage.
    ///
    /// This may be invoked if a user makes a withdrawal, or gets liquidated.
    pub fn decrease_collateral(
        &self,
        store: &mut dyn Storage,
        market: &Market,
        amount_scaled: Uint128,
        incentives_addr: &Addr,
        response: Response,
        account_id: Option<String>,
    ) -> StdResult<Response> {
        let acc_id = account_id.clone().unwrap_or("".to_string());

        let user_id = UserId::credit_manager(self.0.clone(), acc_id);
        let user_id_key: UserIdKey = user_id.try_into()?;

        let mut collateral = COLLATERALS.load(store, (&user_id_key, &market.denom))?;

        let amount_scaled_before = collateral.amount_scaled;
        collateral.amount_scaled = collateral.amount_scaled.checked_sub(amount_scaled)?;

        if collateral.amount_scaled.is_zero() {
            COLLATERALS.remove(store, (&user_id_key, &market.denom));
        } else {
            COLLATERALS.save(store, (&user_id_key, &market.denom), &collateral)?;
        }

        let msg = self.build_incentives_balance_changed_msg(
            incentives_addr,
            market,
            amount_scaled_before,
            account_id,
        )?;

        Ok(response.add_message(msg))
    }

    /// For internal use by the struct only.
    ///
    /// Create an execute message to inform the incentive contract to update the user's index upon a
    /// change in the user's scaled collateral amount.
    fn build_incentives_balance_changed_msg(
        &self,
        incentives_addr: &Addr,
        market: &Market,
        user_amount_scaled_before: Uint128,
        account_id: Option<String>,
    ) -> StdResult<CosmosMsg> {
        Ok(WasmMsg::Execute {
            contract_addr: incentives_addr.into(),
            msg: to_json_binary(&incentives::ExecuteMsg::BalanceChange {
                user_addr: self.address().clone(),
                account_id,
                denom: market.denom.clone(),
                user_amount_scaled_before,
                total_amount_scaled_before: market.collateral_total_scaled,
            })?,
            funds: vec![],
        }
        .into())
    }

    /// Increase a user's debt shares by the specified amount.
    ///
    /// This may be invoked if a user makes a new borrowing.
    pub fn increase_debt(
        &self,
        store: &mut dyn Storage,
        denom: &str,
        amount_scaled: Uint128,
        uncollateralized: bool,
    ) -> StdResult<()> {
        DEBTS.update(store, (self.0, denom), |opt| -> StdResult<_> {
            match opt {
                Some(debt) => Ok(Debt {
                    amount_scaled: debt.amount_scaled.checked_add(amount_scaled)?,
                    uncollateralized,
                }),
                None => Ok(Debt {
                    amount_scaled,
                    uncollateralized,
                }),
            }
        })?;
        Ok(())
    }

    /// Decrease a user's debt shares by the specified amount. If reduced to zero, delete the debt
    /// position from contract storage.
    ///
    /// This may be invoked if a user makes a repayment, or gets liquidated.
    pub fn decrease_debt(
        &self,
        store: &mut dyn Storage,
        denom: &str,
        amount_scaled: Uint128,
    ) -> StdResult<()> {
        let mut debt = DEBTS.load(store, (self.0, denom))?;

        debt.amount_scaled = debt.amount_scaled.checked_sub(amount_scaled)?;

        if debt.amount_scaled.is_zero() {
            DEBTS.remove(store, (self.0, denom));
        } else {
            DEBTS.save(store, (self.0, denom), &debt)?;
        }

        Ok(())
    }
}
