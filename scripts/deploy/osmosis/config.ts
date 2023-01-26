import { DeploymentConfig, AssetConfig, OracleConfig } from '../../types/config'

export const osmosisTestnetConfig: DeploymentConfig = {
  chainName: 'osmosis',
  atomDenom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  baseAssetDenom: 'uosmo',
  chainId: 'osmo-test-4',
  chainPrefix: 'osmo',
  channelId: 'channel-2083',
  marsDenom: 'ibc/ACA4C8A815A053CC027DB90D15915ADA31939FA331CE745862CDD00A2904FA17',
  rewardCollectorTimeoutBlocks: 100,
  rewardCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'https://rpc-test.osmosis.zone',
  safetyFundFeeShare: '0.5',
  timeoutRevision: 1,
  deployerMnemonic:
    'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process',
  slippage_tolerance: '0.01',
  base_asset_symbol: 'OSMO',
  second_asset_symbol: 'ATOM',
  runTests: false,
  mainnet: false,
}
export const osmosisTestMultisig: DeploymentConfig = {
  chainName: 'osmosis',
  atomDenom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  baseAssetDenom: 'uosmo',
  chainId: 'osmo-test-4',
  chainPrefix: 'osmo',
  channelId: 'channel-2083',
  marsDenom: 'ibc/ACA4C8A815A053CC027DB90D15915ADA31939FA331CE745862CDD00A2904FA17',
  rewardCollectorTimeoutBlocks: 100,
  rewardCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'https://rpc-test.osmosis.zone',
  safetyFundFeeShare: '0.5',
  timeoutRevision: 1,
  deployerMnemonic:
    'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process',
  slippage_tolerance: '0.01',
  base_asset_symbol: 'OSMO',
  second_asset_symbol: 'ATOM',
  multisigAddr: 'osmo1jklpvl3446z5qw58cvq8hqvthzjtsfvs9j65tq',
  runTests: false,
  mainnet: false,
}
/// FIXME:: TBD fields must be updated after mars hub launch
export const osmosisMainnet: DeploymentConfig = {
  chainName: 'osmosis',
  atomDenom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  baseAssetDenom: 'uosmo',
  chainId: 'osmosis-1',
  chainPrefix: 'osmo',
  channelId: 'TBD',
  marsDenom: 'TBD',
  rewardCollectorTimeoutBlocks: 100,
  rewardCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'https://rpc.osmosis.zone',
  safetyFundFeeShare: '0.5',
  timeoutRevision: 1,
  deployerMnemonic: 'TO BE INSERTED AT TIME OF DEPLOYMENT',
  slippage_tolerance: '0.01',
  base_asset_symbol: 'OSMO',
  second_asset_symbol: 'ATOM',
  multisigAddr: 'osmo1jklpvl3446z5qw58cvq8hqvthzjtsfvs9j65tq',
  runTests: false,
  mainnet: true,
}

export const osmosisLocalConfig: DeploymentConfig = {
  chainName: 'osmosis',
  atomDenom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  baseAssetDenom: 'uosmo',
  chainId: 'localosmosis',
  chainPrefix: 'osmo',
  channelId: 'channel-1',
  marsDenom: 'umars',
  rewardCollectorTimeoutBlocks: 100,
  rewardCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'http://localhost:26657',
  safetyFundFeeShare: '0.2',
  timeoutRevision: 1,
  deployerMnemonic:
    'notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius',
  slippage_tolerance: '0.05',
  base_asset_symbol: 'OSMO',
  second_asset_symbol: 'ATOM',
  runTests: false,
  mainnet: false,
}

export const osmoAsset: AssetConfig = {
  denom: 'uosmo',
  max_loan_to_value: '0.59',
  reserve_factor: '0.2',
  liquidation_threshold: '0.61',
  liquidation_bonus: '0.15',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
  deposit_cap: '2500000000000',
  deposit_enabled: true,
  borrow_enabled: true,
  symbol: 'OSMO',
}

export const atomAsset: AssetConfig = {
  denom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  max_loan_to_value: '0.68',
  reserve_factor: '0.2',
  liquidation_threshold: '0.7',
  liquidation_bonus: '0.15',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
  deposit_cap: '100000000000',
  deposit_enabled: true,
  borrow_enabled: true,
  symbol: 'ATOM',
}

export const axlUSDCAsset: AssetConfig = {
  denom: 'ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858',
  max_loan_to_value: '0.79',
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
}

export const atomOracle: OracleConfig = {
  denom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  pool_id: 1,
  window_size: 1800,
  downtime_detector: { downtime: 'duration10m', recovery: 120 },
}

export const osmoOracle: OracleConfig = {
  denom: 'uosmo',
  price: '1.0',
}

export const axlUSDCOracle: OracleConfig = {
  denom: 'ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858',
  pool_id: 678,
  window_size: 1800,
  downtime_detector: { downtime: 'duration10m', recovery: 7200 },
}
