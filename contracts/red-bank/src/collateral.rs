use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};
use mars_types::{
    self,
    address_provider::{self, MarsAddressType},
    keys::{UserId, UserIdKey},
};

use crate::{
    error::ContractError,
    health::get_health_and_positions,
    state::{COLLATERALS, CONFIG},
    user::User,
};

/// Update (enable / disable) collateral asset for specific user
pub fn update_asset_collateral_status(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    denom: String,
    enable: bool,
) -> Result<Response, ContractError> {
    let user = User(&info.sender);

    let user_id = UserId::credit_manager(info.sender.clone(), "".to_string());
    let user_id_key: UserIdKey = user_id.try_into()?;

    let mut collateral =
        COLLATERALS.may_load(deps.storage, (&user_id_key, &denom))?.ok_or_else(|| {
            ContractError::UserNoCollateralBalance {
                user: user.into(),
                denom: denom.clone(),
            }
        })?;

    let previously_enabled = collateral.enabled;

    collateral.enabled = enable;
    COLLATERALS.save(deps.storage, (&user_id_key, &denom), &collateral)?;

    // if the collateral was previously enabled, but is not disabled, it is necessary to ensure the
    // user is not liquidatable after disabling
    if previously_enabled && !enable {
        let config = CONFIG.load(deps.storage)?;

        let addresses = address_provider::helpers::query_contract_addrs(
            deps.as_ref(),
            &config.address_provider,
            vec![MarsAddressType::Oracle, MarsAddressType::Params],
        )?;
        let oracle_addr = &addresses[&MarsAddressType::Oracle];
        let params_addr = &addresses[&MarsAddressType::Params];

        let (health, _) = get_health_and_positions(
            &deps.as_ref(),
            &env,
            user.address(),
            "",
            oracle_addr,
            params_addr,
            false,
        )?;

        if health.is_liquidatable() {
            return Err(ContractError::InvalidHealthFactorAfterDisablingCollateral {});
        }
    }

    Ok(Response::new()
        .add_attribute("action", "update_asset_collateral_status")
        .add_attribute("user", user)
        .add_attribute("denom", denom)
        .add_attribute("enable", enable.to_string()))
}
