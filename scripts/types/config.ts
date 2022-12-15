export interface DeploymentConfig {
  chainName: string
  rewardCollectorTimeoutSeconds: number
  marsDenom: string
  baseAssetDenom: string
  atomDenom: string
  chainPrefix: string
  safetyFundFeeShare: string
  channelId: string
  timeoutRevision: number
  rewardCollectorTimeoutBlocks: number
  chainId: string
  rpcEndpoint: string
  deployerMnemonic: string
  slippage_tolerance: string
  base_asset_symbol: string
  second_asset_symbol: string
  multisigAddr?: string
  runTests: boolean
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
  price?: string
  pool_id?: number
  window_size?: number
}

export interface MultisigConfig {
  address: string
  useMultisig: boolean
}
