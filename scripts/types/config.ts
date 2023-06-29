import { OsmosisPriceSourceForString } from './generated/mars-oracle-osmosis/MarsOracleOsmosis.types'
import { OsmosisRoute } from './generated/mars-swapper-osmosis/MarsSwapperOsmosis.types'
import { AstroportRoute } from './generated/mars-swapper-astroport/MarsSwapperAstroport.types'
import {
  WasmOracleCustomInitParams,
  WasmPriceSourceForString,
} from './generated/mars-oracle-wasm/MarsOracleWasm.types'
import {
  Coin,
  Decimal,
  HlsAssetTypeForString,
  HlsParamsBaseForString,
  Uint128,
} from './generated/mars-params/MarsParams.types'

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
  targetHealthFactor: string
  swapperDexName: string
  assets: AssetConfig[]
  vaults: VaultConfig[]
  oracleConfigs: OracleConfig[]
  oracleCustomInitParams?: WasmOracleCustomInitParams
  incentiveEpochDuration: number
  maxWhitelistedIncentiveDenoms: number
}

export interface AssetConfig {
  credit_manager: {
    whitelisted: boolean
  }
  symbol: string
  denom: string
  liquidation_bonus: string
  liquidation_threshold: string
  max_loan_to_value: string
  protocol_liquidation_fee: Decimal
  red_bank: {
    borrow_enabled: boolean
    deposit_cap: Uint128
    deposit_enabled: boolean
  }
}

export interface VaultConfig {
  addr: string
  symbol: string
  deposit_cap: Coin
  hls?: {
    correlations: HlsAssetTypeForString
    liquidation_threshold: Decimal
    max_loan_to_value: Decimal
  }
  liquidation_threshold: Decimal
  max_loan_to_value: Decimal
  whitelisted: boolean
}

export type HlsAssetTypeForString =
  | {
      coin: {
        denom: string
      }
    }
  | {
      vault: {
        addr: string
      }
    }

export interface OracleConfig {
  denom: string
  price_source: OsmosisPriceSourceForString | WasmPriceSourceForString
}
