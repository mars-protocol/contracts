// @ts-nocheck
/**
 * This file was automatically generated by @cosmwasm/ts-codegen@0.16.5.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run the @cosmwasm/ts-codegen generate command to regenerate this file.
 */

export interface InstantiateMsg {
  base_denom: string
  owner: string
}
export type ExecuteMsg =
  | {
      set_price_source: {
        denom: string
        price_source: OsmosisPriceSource
      }
    }
  | {
      remove_price_source: {
        denom: string
      }
    }
  | {
      update_owner: AdminUpdate
    }
export type OsmosisPriceSource =
  | {
      fixed: {
        price: Decimal
        [k: string]: unknown
      }
    }
  | {
      spot: {
        pool_id: number
        [k: string]: unknown
      }
    }
  | {
      twap: {
        pool_id: number
        window_size: number
        [k: string]: unknown
      }
    }
  | {
      xyk_liquidity_token: {
        pool_id: number
        [k: string]: unknown
      }
    }
export type Decimal = string
export type AdminUpdate =
  | {
      propose_new_admin: {
        proposed: string
      }
    }
  | 'clear_proposed'
  | 'accept_proposed'
  | 'abolish_admin_role'
export type QueryMsg =
  | {
      config: {}
    }
  | {
      price_source: {
        denom: string
      }
    }
  | {
      price_sources: {
        limit?: number | null
        start_after?: string | null
      }
    }
  | {
      price: {
        denom: string
      }
    }
  | {
      prices: {
        limit?: number | null
        start_after?: string | null
      }
    }
export interface ConfigResponse {
  base_denom: string
  owner?: string | null
  proposed_new_owner?: string | null
}
export interface PriceResponse {
  denom: string
  price: Decimal
}
export interface PriceSourceResponseForString {
  denom: string
  price_source: string
}
export type ArrayOfPriceSourceResponseForString = PriceSourceResponseForString[]
export type ArrayOfPriceResponse = PriceResponse[]
