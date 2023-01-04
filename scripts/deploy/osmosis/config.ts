import { DeploymentConfig, AssetConfig, OracleConfig } from '../../types/config'

export const osmosisTestnetConfig: DeploymentConfig = {
  chainName: 'osmosis',
  atomDenom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  baseAssetDenom: 'uosmo',
  chainId: 'osmo-test-4',
  chainPrefix: 'osmo',
  channelId: 'channel-1753',
  marsDenom: 'umars',
  rewardCollectorTimeoutBlocks: 10,
  rewardCollectorTimeoutSeconds: 60,
  rpcEndpoint: 'https://rpc-test.osmosis.zone',
  // permissioned testnet:
  // rpcEndpoint: 'http://137.184.6.241:26657/',
  safetyFundFeeShare: '0.2',
  timeoutRevision: 1,
  deployerMnemonic:
    'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process',
  slippage_tolerance: '0.05',
  base_asset_symbol: 'OSMO',
  second_asset_symbol: 'ATOM',
  runTests: true,
}
export const osmosisTestMultisig: DeploymentConfig = {
  chainName: 'osmosis',
  atomDenom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  baseAssetDenom: 'uosmo',
  chainId: 'osmo-test-4',
  chainPrefix: 'osmo',
  channelId: 'channel-1',
  marsDenom: 'umars',
  rewardCollectorTimeoutBlocks: 10,
  rewardCollectorTimeoutSeconds: 60,
  rpcEndpoint: 'https://rpc-test.osmosis.zone',
  // permissioned testnet:
  // rpcEndpoint: 'http://137.184.6.241:26657/',
  safetyFundFeeShare: '0.2',
  timeoutRevision: 1,
  deployerMnemonic:
    'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process',
  slippage_tolerance: '0.5',
  base_asset_symbol: 'OSMO',
  second_asset_symbol: 'ATOM',
  multisigAddr: 'osmo1jklpvl3446z5qw58cvq8hqvthzjtsfvs9j65tq',
  runTests: true,
}

export const osmosisMainnet: DeploymentConfig = {
  chainName: 'osmosis',
  atomDenom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  baseAssetDenom: 'uosmo',
  chainId: 'osmosis-1',
  chainPrefix: 'osmo',
  channelId: 'TBD',
  marsDenom: 'umars',
  rewardCollectorTimeoutBlocks: 10,
  rewardCollectorTimeoutSeconds: 60,
  rpcEndpoint: 'https://rpc.osmosis.zone',
  safetyFundFeeShare: '0.2',
  timeoutRevision: 1,
  deployerMnemonic: 'TO BE INSERTED AT TIME OF DEPLOYMENT',
  slippage_tolerance: '0.1',
  base_asset_symbol: 'OSMO',
  second_asset_symbol: 'ATOM',
  multisigAddr: 'osmo1jklpvl3446z5qw58cvq8hqvthzjtsfvs9j65tq',
  runTests: false,
}

export const osmosisLocalConfig: DeploymentConfig = {
  chainName: 'osmosis',
  atomDenom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  baseAssetDenom: 'uosmo',
  chainId: 'localosmosis',
  chainPrefix: 'osmo',
  channelId: 'channel-1',
  marsDenom: 'umars',
  rewardCollectorTimeoutBlocks: 10,
  rewardCollectorTimeoutSeconds: 60,
  rpcEndpoint: 'http://localhost:26657',
  safetyFundFeeShare: '0.2',
  timeoutRevision: 1,
  deployerMnemonic:
    'notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius',
  slippage_tolerance: '0.05',
  base_asset_symbol: 'OSMO',
  second_asset_symbol: 'ATOM',
  runTests: false,
}

export const osmoAsset: AssetConfig = {
  denom: 'uosmo',
  max_loan_to_value: '0.59',
  reserve_factor: '0.2',
  liquidation_threshold: '0.61',
  liquidation_bonus: '0.15',
  interest_rate_model: {
    optimal_utilization_rate: '0.5',
    base: '0',
    slope_1: '0.25',
    slope_2: '3',
  },
  deposit_cap: '2000000000000',
  deposit_enabled: true,
  borrow_enabled: true,
  symbol: 'OSMO',
  emission: '560000',
}

export const atomAsset: AssetConfig = {
  denom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  max_loan_to_value: '0.68',
  reserve_factor: '0.2',
  liquidation_threshold: '0.7',
  liquidation_bonus: '0.15',
  interest_rate_model: {
    optimal_utilization_rate: '0.5',
    base: '0',
    slope_1: '0.25',
    slope_2: '3',
  },
  deposit_cap: '100000000000',
  deposit_enabled: true,
  borrow_enabled: true,
  symbol: 'ATOM',
  emission: '220000',
}

export const axlUSDCAsset: AssetConfig = {
  denom: 'ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858',
  max_loan_to_value: '0.795',
  reserve_factor: '0.2',
  liquidation_threshold: '0.8',
  liquidation_bonus: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.8',
    base: '0',
    slope_1: '0.2',
    slope_2: '2',
  },
  deposit_cap: '500000000000',
  deposit_enabled: true,
  borrow_enabled: true,
  symbol: 'axlUSDC',
  emission: '110000',
}

export const atomOracle: OracleConfig = {
  denom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  pool_id: 1,
  window_size: 1800,
}

export const osmoOracle: OracleConfig = {
  denom: 'uosmo',
  price: '1.0',
}

export const axlUSDCOracle: OracleConfig = {
  denom: 'ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858',
  pool_id: 678,
  window_size: 1800,
}
