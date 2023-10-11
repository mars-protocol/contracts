use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Decimal};
use mars_utils::helpers::validate_native_denom;

use crate::error::MarsError;

#[cw_serde]
pub enum HlsAssetType<T> {
    Coin {
        denom: String,
    },
    Vault {
        addr: T,
    },
}

impl From<HlsAssetType<Addr>> for HlsAssetType<String> {
    fn from(t: HlsAssetType<Addr>) -> Self {
        match t {
            HlsAssetType::Coin {
                denom,
            } => HlsAssetType::Coin {
                denom,
            },
            HlsAssetType::Vault {
                addr,
            } => HlsAssetType::Vault {
                addr: addr.to_string(),
            },
        }
    }
}

#[cw_serde]
pub struct HlsParamsBase<T> {
    pub max_loan_to_value: Decimal,
    pub liquidation_threshold: Decimal,
    /// Given this asset is debt, correlations are the only allowed collateral
    /// which are permitted to fulfill the HLS strategy
    pub correlations: Vec<HlsAssetType<T>>,
}

pub type HlsParams = HlsParamsBase<Addr>;
pub type HlsParamsUnchecked = HlsParamsBase<String>;

impl From<HlsParams> for HlsParamsUnchecked {
    fn from(hls: HlsParams) -> Self {
        Self {
            max_loan_to_value: hls.max_loan_to_value,
            liquidation_threshold: hls.liquidation_threshold,
            correlations: hls.correlations.into_iter().map(Into::into).collect(),
        }
    }
}

impl HlsParamsUnchecked {
    pub fn check(&self, api: &dyn Api) -> Result<HlsParams, MarsError> {
        Ok(HlsParamsBase {
            max_loan_to_value: self.max_loan_to_value,
            liquidation_threshold: self.liquidation_threshold,
            correlations: self
                .correlations
                .iter()
                .map(|c| match c {
                    HlsAssetType::Coin {
                        denom,
                    } => {
                        validate_native_denom(denom)?;
                        Ok(HlsAssetType::Coin {
                            denom: denom.clone(),
                        })
                    }
                    HlsAssetType::Vault {
                        addr,
                    } => Ok(HlsAssetType::Vault {
                        addr: api.addr_validate(addr)?,
                    }),
                })
                .collect::<Result<Vec<_>, MarsError>>()?,
        })
    }
}
