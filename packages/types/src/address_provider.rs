use std::{any::type_name, fmt, str::FromStr};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::StdError;
use mars_owner::OwnerUpdate;
use strum::EnumIter;

#[cw_serde]
#[derive(Copy, Eq, Hash, EnumIter)]
pub enum MarsAddressType {
    Incentives,
    Oracle,
    RedBank,
    RewardsCollector,
    Params,
    CreditManager,
    /// Protocol admin is an ICS-27 interchain account controlled by Mars Hub's x/gov module.
    /// This account will take the owner and admin roles of red-bank contracts.
    ///
    /// Owner means the account who can invoke certain priviliged execute methods on a contract,
    /// such as updating the config.
    /// Admin means the account who can migrate a contract.
    ProtocolAdmin,
    /// The `fee_collector` module account controlled by Mars Hub's x/distribution module.
    /// Funds sent to this account will be distributed as staking rewards.
    ///
    /// NOTE: This is a Mars Hub address with the `mars` bech32 prefix, which may not be recognized
    /// by the `api.addr_validate` method.
    FeeCollector,
    /// The module account controlled by the by Mars Hub's x/safety module.
    /// Funds sent to this account will be deposited into the safety fund.
    ///
    /// NOTE: This is a Mars Hub address with the `mars` bech32 prefix, which may not be recognized
    /// by the `api.addr_validate` method.
    SafetyFund,
    /// The swapper contract on the chain
    Swapper,
    /// Astroport incentives contract
    AstroportIncentives,
    /// Perps contract
    Perps,
    /// The address that shall receive the revenue share given to neutron (10%)
    RevenueShare,
}

impl fmt::Display for MarsAddressType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MarsAddressType::CreditManager => "credit_manager",
            MarsAddressType::FeeCollector => "fee_collector",
            MarsAddressType::Incentives => "incentives",
            MarsAddressType::Oracle => "oracle",
            MarsAddressType::Params => "params",
            MarsAddressType::ProtocolAdmin => "protocol_admin",
            MarsAddressType::RedBank => "red_bank",
            MarsAddressType::RewardsCollector => "rewards_collector",
            MarsAddressType::SafetyFund => "safety_fund",
            MarsAddressType::Swapper => "swapper",
            MarsAddressType::AstroportIncentives => "astroport_incentives",
            MarsAddressType::Perps => "perps",
            MarsAddressType::RevenueShare => "revenue_share",
        };
        write!(f, "{s}")
    }
}

impl FromStr for MarsAddressType {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "credit_manager" => Ok(MarsAddressType::CreditManager),
            "fee_collector" => Ok(MarsAddressType::FeeCollector),
            "incentives" => Ok(MarsAddressType::Incentives),
            "oracle" => Ok(MarsAddressType::Oracle),
            "params" => Ok(MarsAddressType::Params),
            "protocol_admin" => Ok(MarsAddressType::ProtocolAdmin),
            "red_bank" => Ok(MarsAddressType::RedBank),
            "rewards_collector" => Ok(MarsAddressType::RewardsCollector),
            "safety_fund" => Ok(MarsAddressType::SafetyFund),
            "swapper" => Ok(MarsAddressType::Swapper),
            "astroport_incentives" => Ok(MarsAddressType::AstroportIncentives),
            "perps" => Ok(MarsAddressType::Perps),
            "revenue_share" => Ok(MarsAddressType::RevenueShare),
            _ => Err(StdError::parse_err(type_name::<Self>(), s)),
        }
    }
}

/// Essentially, mars-address-provider is a required init param for all other contracts, so it needs
/// to be initialised first (Only owner can be set on initialization). So the deployment looks like
/// this:
///
/// 1. Init the address provider
/// 2. Init all other contracts, passing in the address provider address (not ALL contracts need this
///    but many do)
/// 3. Update the address provider, with an update config call to contain all the other contract addresses
///    from step 2, this is why we need it to be owned by an EOA (externally owned account) - so we
///    can do this update as part of the deployment
/// 4. Update the owner of the address provider contract at the end of deployment to be either a. the
///    multisig or b. the gov/council contract
#[cw_serde]
pub struct InstantiateMsg {
    /// The contract's owner
    pub owner: String,
    /// The address prefix of the chain this contract is deployed on
    pub prefix: String,
}

#[cw_serde]
pub struct Config {
    /// The address prefix of the chain this contract is deployed on
    pub prefix: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Set address
    SetAddress {
        address_type: MarsAddressType,
        address: String,
    },
    /// Manages admin role state
    UpdateOwner(OwnerUpdate),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get config
    #[returns(ConfigResponse)]
    Config {},
    /// Get a single address
    #[returns(AddressResponseItem)]
    Address(MarsAddressType),
    /// Get a list of addresses
    #[returns(Vec<AddressResponseItem>)]
    Addresses(Vec<MarsAddressType>),
    /// Query all stored addresses with pagination
    #[returns(Vec<AddressResponseItem>)]
    AllAddresses {
        start_after: Option<MarsAddressType>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    /// The contract's owner
    pub owner: Option<String>,
    /// The contract's proposed owner
    pub proposed_new_owner: Option<String>,
    /// The address prefix of the chain this contract is deployed on
    pub prefix: String,
}

#[cw_serde]
pub struct AddressResponseItem {
    /// The type of address
    pub address_type: MarsAddressType,
    /// Address value
    pub address: String,
}

pub mod helpers {
    use std::collections::HashMap;

    use cosmwasm_std::{Addr, CustomQuery, Deps, StdResult};

    use super::{AddressResponseItem, MarsAddressType, QueryMsg};

    /// Query contract address.
    ///
    /// It fails if the provided address does not start with current chain prefix.
    pub fn query_contract_addr(
        deps: Deps<impl CustomQuery>,
        address_provider_addr: &Addr,
        contract: MarsAddressType,
    ) -> StdResult<Addr> {
        deps.querier
            .query_wasm_smart::<AddressResponseItem>(
                address_provider_addr,
                &QueryMsg::Address(contract),
            )
            .map(|res| deps.api.addr_validate(&res.address))?
    }

    /// Query contract addresses.
    ///
    /// It fails if the provided address does not start with current chain prefix.
    pub fn query_contract_addrs(
        deps: Deps<impl CustomQuery>,
        address_provider_addr: &Addr,
        contracts: Vec<MarsAddressType>,
    ) -> StdResult<HashMap<MarsAddressType, Addr>> {
        deps.querier
            .query_wasm_smart::<Vec<AddressResponseItem>>(
                address_provider_addr,
                &QueryMsg::Addresses(contracts),
            )?
            .into_iter()
            .map(|item| Ok((item.address_type, deps.api.addr_validate(&item.address)?)))
            .collect()
    }

    /// Query Mars Hub module address
    pub fn query_module_addr(
        deps: Deps<impl CustomQuery>,
        address_provider_addr: &Addr,
        module: MarsAddressType,
    ) -> StdResult<String> {
        deps.querier
            .query_wasm_smart::<AddressResponseItem>(
                address_provider_addr,
                &QueryMsg::Address(module),
            )
            .map(|res| res.address)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use strum::IntoEnumIterator;

    use super::MarsAddressType;

    #[test]
    fn mars_address_type_fmt_and_from_string() {
        for address_type in MarsAddressType::iter() {
            assert_eq!(MarsAddressType::from_str(&address_type.to_string()).unwrap(), address_type);
        }
    }

    #[test]
    #[should_panic]
    fn mars_address_type_from_str_invalid_string() {
        MarsAddressType::from_str("invalid_address_type").unwrap();
    }
}
