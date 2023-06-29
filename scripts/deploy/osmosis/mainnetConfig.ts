import { DeploymentConfig, AssetConfig, OracleConfig } from '../../types/config'

// Mainnet:
const osmo = 'uosmo'
const atom = 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2'
const axlUSDC = 'ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858'
const mars = 'ibc/573FCD90FACEE750F55A8864EF7D38265F07E5A9273FA0E8DAFD39951332B580'

const pythContractAddr = ''

// note the following three addresses are all 'mars' bech32 prefix
const safetyFundAddr = ''
const protocolAdminAddr = ''
const feeCollectorAddr = ''

export const osmoAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'OSMO',
  denom: osmo,
  liquidation_bonus: '0.15',
  liquidation_threshold: '0.61',
  max_loan_to_value: '0.59',
  red_bank: {
    borrow_enabled: true,
    deposit_cap: '2500000000000',
    deposit_enabled: true,
  },
}

export const atomAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'ATOM',
  denom: atom,
  liquidation_bonus: '0.15',
  liquidation_threshold: '0.7',
  max_loan_to_value: '0.68',
  red_bank: {
    borrow_enabled: true,
    deposit_cap: '100000000000',
    deposit_enabled: true,
  },
}

export const axlUSDCAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'axlUSDC',
  denom: axlUSDC,
  liquidation_bonus: '0.1',
  liquidation_threshold: '0.75',
  max_loan_to_value: '0.74',
  red_bank: {
    borrow_enabled: true,
    deposit_cap: '500000000000',
    deposit_enabled: true,
  },
}

export const atomOracle = {
  denom: atom,
  price_source: {
    pyth: {
      contract_addr: pythContractAddr,
      price_feed_id: '',
      max_staleness: 60,
      denom_decimals: 6,
      max_confidence: 5,
      max_deviation: 4,
    },
  },
}

export const axlUSDCOracle: OracleConfig = {
  denom: axlUSDC,
  price_source: {
    geometric_twap: {
      pool_id: 678,
      window_size: 1800,
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
    },
  },
}

export const osmosisMainnet = {
  chainName: 'osmosis',
  atomDenom: atom,
  baseAssetDenom: osmo,
  gasPrice: '0.1uosmo',
  chainId: 'osmosis-1',
  chainPrefix: 'osmo',
  channelId: 'channel-557',
  marsDenom: mars,
  rewardCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'https://rpc.osmosis.zone',
  safetyFundFeeShare: '0.5',
  deployerMnemonic: 'TO BE INSERTED AT TIME OF DEPLOYMENT',
  slippage_tolerance: '0.01',
  base_asset_symbol: 'OSMO',
  second_asset_symbol: 'ATOM',
  multisigAddr: 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n',
  runTests: false,
  mainnet: true,
  feeCollectorDenom: mars,
  safetyFundDenom: axlUSDC,
  swapRoutes: [
    { denom_in: osmo, denom_out: axlUSDC, route: [{ pool_id: 678, token_out_denom: axlUSDC }] },
    {
      denom_in: atom,
      denom_out: axlUSDC,
      route: [
        { pool_id: 1, token_out_denom: osmo },
        { pool_id: 678, token_out_denom: axlUSDC },
      ],
    },
    {
      denom_in: osmo,
      denom_out: mars,
      route: [{ pool_id: 907, token_out_denom: mars }],
    },
    {
      denom_in: atom,
      denom_out: mars,
      route: [
        { pool_id: 1, token_out_denom: 'uosmo' },
        { pool_id: 907, token_out_denom: mars },
      ],
    },
    {
      denom_in: axlUSDC,
      denom_out: mars,
      route: [
        { pool_id: 678, token_out_denom: osmo },
        { pool_id: 907, token_out_denom: mars },
      ],
    },
  ],
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  swapperDexName: 'osmosis',
  assets: [osmoAsset, atomAsset, axlUSDCAsset],
  vaults: [],
  oracleConfigs: [atomOracle, axlUSDCOracle],
  targetHealthFactor: '1.2',
  incentiveEpochDuration: 86400,
  maxWhitelistedIncentiveDenoms: 10,
}
