import { DeploymentConfig, AssetConfig, MultisigConfig } from '../../types/config'

export const osmosisTestnetConfig: DeploymentConfig = {
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
  slippage_tolerance: '0.5',
  base_asset_symbol: 'OSMO',
  second_asset_symbol: 'ATOM',
}

export const osmoAsset: AssetConfig = {
  denom: 'uosmo',
  initial_borrow_rate: '0.1',
  max_loan_to_value: '0.55',
  reserve_factor: '0.2',
  liquidation_threshold: '0.65',
  liquidation_bonus: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.7',
    base: '0.3',
    slope_1: '0.25',
    slope_2: '0.3',
  },
  deposit_cap: '1000000000',
  deposit_enabled: true,
  borrow_enabled: true,
  symbol: 'OSMO',
}

export const atomAsset: AssetConfig = {
  denom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  initial_borrow_rate: '0.1',
  max_loan_to_value: '0.65',
  reserve_factor: '0.2',
  liquidation_threshold: '0.7',
  liquidation_bonus: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.1',
    base: '0.3',
    slope_1: '0.25',
    slope_2: '0.3',
  },
  deposit_cap: '1000000000',
  deposit_enabled: true,
  borrow_enabled: true,
  symbol: 'ATOM',
}

export const osmosisMultisig: MultisigConfig = {
  address: 'osmo1zwt8al0cev8gfs8esxq5h340m6edjanwmvt7wy',
}
