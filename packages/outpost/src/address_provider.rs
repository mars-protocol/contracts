use std::any::type_name;
use std::fmt;
use std::str::FromStr;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, StdError};

/// Addresses of the current chain
#[cw_serde]
#[derive(Copy, Eq, Hash)]
pub enum MarsLocal {
    Incentives,
    Oracle,
    RedBank,
    RewardsCollector,

    /// Protocol admin is an ICS-27 interchain account controlled by Mars Hub's x/gov module.
    /// This account will take the owner and admin roles of outpost contracts.
    ///
    /// Owner means the account who can invoke certain priviliged execute methods on a contract,
    /// such as updating the config.
    /// Admin means the account who can migrate a contract.
    ProtocolAdmin,
}

/// Addresses that aren't on the current chain
#[cw_serde]
#[derive(Copy, Eq, Hash)]
pub enum MarsRemote {
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
}

impl fmt::Display for MarsLocal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MarsLocal::Incentives => "incentives",
            MarsLocal::Oracle => "oracle",
            MarsLocal::RedBank => "red_bank",
            MarsLocal::RewardsCollector => "rewards_collector",
            MarsLocal::ProtocolAdmin => "protocol_admin",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for MarsRemote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            MarsRemote::FeeCollector => "fee_collector",
            MarsRemote::SafetyFund => "safety_fund",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for MarsLocal {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "incentives" => Ok(MarsLocal::Incentives),
            "oracle" => Ok(MarsLocal::Oracle),
            "red_bank" => Ok(MarsLocal::RedBank),
            "rewards_collector" => Ok(MarsLocal::RewardsCollector),
            "protocol_admin" => Ok(MarsLocal::ProtocolAdmin),
            _ => Err(StdError::parse_err(type_name::<Self>(), s)),
        }
    }
}

impl FromStr for MarsRemote {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fee_collector" => Ok(MarsRemote::FeeCollector),
            "safety_fund" => Ok(MarsRemote::SafetyFund),
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
pub enum ExecuteMsg {
    /// Set local address
    SetLocalAddress {
        local: MarsLocal,
        address: String,
    },
    /// Set remote address
    SetRemoteAddress {
        remote: MarsRemote,
        address: String,
    },
    /// Propose to transfer the contract's ownership to another account
    TransferOwnership {
        new_owner: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Get config
    #[returns(Config)]
    Config {},

    /// Get a single local address
    #[returns(LocalAddressResponse)]
    LocalAddress(MarsLocal),
    /// Get a list of local addresses
    #[returns(Vec<LocalAddressResponse>)]
    LocalAddresses(Vec<MarsLocal>),
    /// Query all stored local addresses with pagination
    #[returns(Vec<LocalAddressResponse>)]
    AllLocalAddresses {
        start_after: Option<MarsLocal>,
        limit: Option<u32>,
    },

    /// Get a single remote address
    #[returns(RemoteAddressResponse)]
    RemoteAddress(MarsRemote),
    /// Get a list of remote addresses
    #[returns(Vec<RemoteAddressResponse>)]
    RemoteAddresses(Vec<MarsRemote>),
    /// Query all stored remote addresses with pagination
    #[returns(Vec<RemoteAddressResponse>)]
    AllRemoteAddresses {
        start_after: Option<MarsRemote>,
        limit: Option<u32>,
    },
}

pub type Config = InstantiateMsg;

#[cw_serde]
pub struct LocalAddressResponse {
    /// Local address
    pub local: MarsLocal,
    /// Validated address on the current chain
    pub address: Addr,
}

#[cw_serde]
pub struct RemoteAddressResponse {
    /// Remote address
    pub remote: MarsRemote,
    /// Address not on the current chain
    pub address: String,
}

pub mod helpers {
    use std::collections::HashMap;

    use super::{LocalAddressResponse, MarsLocal, QueryMsg};
    use crate::address_provider::{MarsRemote, RemoteAddressResponse};
    use cosmwasm_std::{Addr, Deps, StdResult};

    pub fn query_local_address(
        deps: Deps<impl cosmwasm_std::CustomQuery>,
        address_provider_addr: &Addr,
        contract: MarsLocal,
    ) -> StdResult<Addr> {
        let res: LocalAddressResponse = deps
            .querier
            .query_wasm_smart(address_provider_addr, &QueryMsg::LocalAddress(contract))?;

        Ok(res.address)
    }

    pub fn query_local_addresses(
        deps: Deps<impl cosmwasm_std::CustomQuery>,
        address_provider_addr: &Addr,
        contracts: Vec<MarsLocal>,
    ) -> StdResult<HashMap<MarsLocal, Addr>> {
        let res: Vec<LocalAddressResponse> = deps
            .querier
            .query_wasm_smart(address_provider_addr, &QueryMsg::LocalAddresses(contracts))?;

        Ok(res.iter().map(|item| (item.local, item.address.clone())).collect())
    }

    pub fn query_remote_address(
        deps: Deps<impl cosmwasm_std::CustomQuery>,
        address_provider_addr: &Addr,
        gov: MarsRemote,
    ) -> StdResult<String> {
        let res: RemoteAddressResponse =
            deps.querier.query_wasm_smart(address_provider_addr, &QueryMsg::RemoteAddress(gov))?;

        Ok(res.address)
    }
}
