import { OsmosisPriceSourceForString } from './generated/mars-oracle-osmosis/MarsOracleOsmosis.types'
import { OsmosisRoute } from './generated/mars-swapper-osmosis/MarsSwapperOsmosis.types'
import { AstroportRoute } from './generated/mars-swapper-astroport/MarsSwapperAstroport.types'
import { WasmPriceSourceForString } from './generated/mars-oracle-wasm/MarsOracleWasm.types'

type SwapRoute = {
  denom_in: string
  denom_out: string
  route: OsmosisRoute | AstroportRoute
}

export type SwapperExecuteMsg = {
  set_route: SwapRoute
}

export function isOsmosisRoute(route: OsmosisRoute | AstroportRoute): route is OsmosisRoute {
  return Array.isArray(route)
}

export function isAstroportRoute(route: OsmosisRoute | AstroportRoute): route is AstroportRoute {
  return !isOsmosisRoute(route)
}

export interface DeploymentConfig {
  chainName: string
  oracleName: string
  rewardCollectorTimeoutSeconds: number
  marsDenom: string
  baseAssetDenom: string
  gasPrice: string
  atomDenom: string
  chainPrefix: string
  safetyFundFeeShare: string
  channelId: string
  feeCollectorDenom: string
  safetyFundDenom: string
  chainId: string
  rpcEndpoint: string
  deployerMnemonic: string
  slippage_tolerance: string
  base_asset_symbol: string
  second_asset_symbol: string
  multisigAddr?: string
  runTests: boolean
  mainnet: boolean
  swapRoutes: SwapRoute[]
  safetyFundAddr: string
  protocolAdminAddr: string
  feeCollectorAddr: string
  maxCloseFactor: string
  swapperDexName: string
  assets: AssetConfig[]
  oracleConfigs: OracleConfig[]
}

export interface AssetConfig {
  denom: string
  max_loan_to_value: string
  reserve_factor: string
  liquidation_threshold: string
  liquidation_bonus: string
  interest_rate_model: {
    optimal_utilization_rate: string
    base: string
    slope_1: string
    slope_2: string
  }
  deposit_cap: string
  deposit_enabled: boolean
  borrow_enabled: boolean
  symbol: string
}

export interface OracleConfig {
  denom: string
  price_source: OsmosisPriceSourceForString | WasmPriceSourceForString
}
