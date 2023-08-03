import { OsmosisPriceSourceForString } from './generated/mars-oracle-osmosis/MarsOracleOsmosis.types'
import { OsmosisRoute } from './generated/mars-swapper-osmosis/MarsSwapperOsmosis.types'
import { AstroportRoute } from './generated/mars-swapper-astroport/MarsSwapperAstroport.types'
import {
  WasmOracleCustomInitParams,
  WasmPriceSourceForString,
} from './generated/mars-oracle-wasm/MarsOracleWasm.types'
import {
  CmSettingsForString,
  Coin,
  Decimal,
  HlsParamsBaseForString,
  LiquidationBonus,
  RedBankSettings,
} from './generated/mars-params/MarsParams.types'
import { NeutronIbcConfig } from './generated/mars-rewards-collector-base/MarsRewardsCollectorBase.types'
import { Uint128 } from './generated/mars-red-bank/MarsRedBank.types'

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
  oracleBaseDenom: string
  rewardsCollectorName: string
  rewardsCollectorTimeoutSeconds: number
  rewardsCollectorNeutronIbcConfig?: NeutronIbcConfig | null
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
  multisigAddr?: string
  runTests: boolean
  mainnet: boolean
  swapRoutes: SwapRoute[]
  safetyFundAddr: string
  protocolAdminAddr: string
  feeCollectorAddr: string
  swapperDexName: string
  assets: AssetConfig[]
  vaults: VaultConfig[]
  oracleConfigs: OracleConfig[]
  oracleCustomInitParams?: WasmOracleCustomInitParams
  incentiveEpochDuration: number
  maxWhitelistedIncentiveDenoms: number
  targetHealthFactor: string
}

export interface AssetConfig {
  symbol: string
  credit_manager: CmSettingsForString
  denom: string
  liquidation_bonus: LiquidationBonus
  liquidation_threshold: Decimal
  max_loan_to_value: Decimal
  protocol_liquidation_fee: Decimal
  red_bank: RedBankSettings
  deposit_cap: Uint128
}
export interface VaultConfig {
  addr: string
  symbol: string
  deposit_cap: Coin
  hls?: HlsParamsBaseForString | null
  liquidation_threshold: Decimal
  max_loan_to_value: Decimal
  whitelisted: boolean
}

export interface OracleConfig {
  denom: string
  price_source: OsmosisPriceSourceForString | WasmPriceSourceForString
}
