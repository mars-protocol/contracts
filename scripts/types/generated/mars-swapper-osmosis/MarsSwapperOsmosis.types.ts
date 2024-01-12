// @ts-nocheck
/**
 * This file was automatically generated by @cosmwasm/ts-codegen@0.35.3.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run the @cosmwasm/ts-codegen generate command to regenerate this file.
 */

export interface InstantiateMsg {
  owner: string
}
export type ExecuteMsg =
  | {
      update_owner: OwnerUpdate
    }
  | {
      set_route: {
        denom_in: string
        denom_out: string
        route: OsmosisRoute
      }
    }
  | {
      swap_exact_in: {
        coin_in: Coin
        denom_out: string
        route?: SwapperRoute | null
        slippage: Decimal
      }
    }
  | {
      transfer_result: {
        denom_in: string
        denom_out: string
        recipient: Addr
      }
    }
export type OwnerUpdate =
  | {
      propose_new_owner: {
        proposed: string
      }
    }
  | 'clear_proposed'
  | 'accept_proposed'
  | 'abolish_owner_role'
  | {
      set_emergency_owner: {
        emergency_owner: string
      }
    }
  | 'clear_emergency_owner'
export type OsmosisRoute = SwapAmountInRoute[]
export type Uint128 = string
export type SwapperRoute =
  | {
      astro: AstroportRoute
    }
  | {
      osmo: OsmosisRoute
    }
export type OsmosisRoute2 = SwapAmountInRoute2[]
export type Decimal = string
export type Addr = string
export interface SwapAmountInRoute {
  pool_id: number
  token_out_denom: string
}
export interface Coin {
  amount: Uint128
  denom: string
  [k: string]: unknown
}
export interface AstroportRoute {
  factory: string
  operations: SwapOperation[]
  oracle: string
  router: string
}
export interface SwapOperation {
  from: string
  to: string
}
export interface SwapAmountInRoute2 {
  pool_id: number
  token_out_denom: string
}
export type QueryMsg =
  | {
      owner: {}
    }
  | {
      route: {
        denom_in: string
        denom_out: string
      }
    }
  | {
      routes: {
        limit?: number | null
        start_after?: [string, string] | null
      }
    }
  | {
      estimate_exact_in_swap: {
        coin_in: Coin
        denom_out: string
        route?: SwapperRoute | null
      }
    }
export interface EstimateExactInSwapResponse {
  amount: Uint128
}
export interface OwnerResponse {
  abolished: boolean
  emergency_owner?: string | null
  initialized: boolean
  owner?: string | null
  proposed?: string | null
}
export interface RouteResponseForEmpty {
  denom_in: string
  denom_out: string
  route: Empty
}
export interface Empty {
  [k: string]: unknown
}
export type ArrayOfRouteResponseForEmpty = RouteResponseForEmpty[]
