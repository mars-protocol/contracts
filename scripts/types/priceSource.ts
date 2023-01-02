export type PriceSource =
  | {
      fixed: {
        price: string
      }
    }
  | {
      spot: {
        pool_id: number
      }
    }
  | {
      twap: {
        pool_id: number
        window_size: number
      }
    }
  | {
      xyk_liquidity_token: {
        pool_id: number
      }
    }
