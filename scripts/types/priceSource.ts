export type PriceSource =
  | {
      fixed: {
        price: Decimal
      }
    }
  | {
      spot: {
        pool_id: number
      }
    }
  | {
      arithmetic_twap: {
        downtime_detector?: DowntimeDetector | null
        pool_id: number
        window_size: number
      }
    }
  | {
      geometric_twap: {
        downtime_detector?: DowntimeDetector | null
        pool_id: number
        window_size: number
      }
    }
  | {
      xyk_liquidity_token: {
        pool_id: number
      }
    }

export type Decimal = string

export type Downtime =
  | 'duration30s'
  | 'duration1m'
  | 'duration2m'
  | 'duration3m'
  | 'duration4m'
  | 'duration5m'
  | 'duration10m'
  | 'duration20m'
  | 'duration30m'
  | 'duration40m'
  | 'duration50m'
  | 'duration1h'
  | 'duration15h'
  | 'duration2h'
  | 'duration25h'
  | 'duration3h'
  | 'duration4h'
  | 'duration5h'
  | 'duration6h'
  | 'duration9h'
  | 'duration12h'
  | 'duration18h'
  | 'duration24h'
  | 'duration36h'
  | 'duration48h'

export interface DowntimeDetector {
  downtime: Downtime
  recovery: number
}
