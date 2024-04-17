pub mod account_nft;
pub mod adapters;
pub mod address_provider;
pub mod credit_manager;
pub mod error;
pub mod health;
pub mod incentives;
pub mod keys;
pub mod oracle;
pub mod params;
pub mod red_bank;
pub mod rewards_collector;
pub mod swapper;
pub mod traits;
pub mod zapper;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{StdError, Storage};
use cw_paginate::paginate_map;
use cw_storage_plus::{Bound, KeyDeserialize, Map, PrimaryKey};
use serde::{de::DeserializeOwned, Serialize};

#[cw_serde]
pub struct PaginationResponse<T> {
    pub data: Vec<T>,
    pub metadata: Metadata,
}

#[cw_serde]
pub struct Metadata {
    pub has_more: bool,
}

pub fn paginate_query<'a, K, T, R, E, F>(
    map: &Map<'a, K, T>,
    store: &dyn Storage,
    start: Option<Bound<'a, K>>,
    limit: usize,
    map_fn: F,
) -> Result<PaginationResponse<R>, E>
where
    K: PrimaryKey<'a> + KeyDeserialize,
    K::Output: 'static,
    T: Serialize + DeserializeOwned,
    F: Fn(K::Output, T) -> Result<R, E>,
    E: From<StdError>,
{
    let limit_plus_one = Some((limit + 1) as u32);
    let mut data = paginate_map(map, store, start, limit_plus_one, map_fn)?;

    let has_more = data.len() > limit;
    if has_more {
        data.pop();
    }

    Ok(PaginationResponse {
        data,
        metadata: Metadata {
            has_more,
        },
    })
}
