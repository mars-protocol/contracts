import { DeploymentConfig, AssetConfig, OracleConfig } from '../../types/config'

// Note: since osmo-test-5 upgrade, testnet and mainnet denoms are no longer the same. Reference asset info here: https://docs.osmosis.zone/osmosis-core/asset-info/
// const uosmo = 'uosmo'
// const uatom = 'ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477'
// const aUSDC = 'ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE' // axelar
// const nUSDC = 'ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4' // noble
// const atom_osmo = 'gamm/pool/12'
// const nUSDC_osmo = 'gamm/pool/6'
// const aUSDC_osmo = 'gamm/pool/5'

// Mainnet:
const axlUSDC = 'ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858'
const usdcTest = 'ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE'
const atomTest = 'ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477'
const atom = 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2'
const marsTest = 'ibc/2E7368A14AC9AB7870F32CFEA687551C5064FA861868EDF7437BC877358A81F9'
const mars = 'ibc/573FCD90FACEE750F55A8864EF7D38265F07E5A9273FA0E8DAFD39951332B580'
// note the following three addresses are all 'mars' bech32 prefix
const safetyFundAddr = 'mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575'
const protocolAdminAddr = 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n'
const feeCollectorAddr = 'mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x'
const marsOsmoPool = 907
// const marsOsmoPoolTest = 9

export const osmoAsset: AssetConfig = {
  denom: 'uosmo',
  max_loan_to_value: '0.59',
  liquidation_threshold: '0.61',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  // reserve_factor: '0.2',
  // interest_rate_model: {
  //   optimal_utilization_rate: '0.6',
  //   base: '0',
  //   slope_1: '0.15',
  //   slope_2: '3',
  // },
  symbol: 'OSMO',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_cap: '2500000000000',
    deposit_enabled: true,
  },
}

export const atomAsset: AssetConfig = {
  denom: atom,
  max_loan_to_value: '0.68',
  liquidation_threshold: '0.7',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  // reserve_factor: '0.2',
  // interest_rate_model: {
  //   optimal_utilization_rate: '0.6',
  //   base: '0',
  //   slope_1: '0.15',
  //   slope_2: '3',
  // },
  symbol: 'ATOM',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_cap: '100000000000',
    deposit_enabled: true,
  },
}

export const atomAssetTest: AssetConfig = {
  denom: atomTest,
  max_loan_to_value: '0.68',
  liquidation_threshold: '0.7',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  // reserve_factor: '0.2',
  // interest_rate_model: {
  //   optimal_utilization_rate: '0.6',
  //   base: '0',
  //   slope_1: '0.15',
  //   slope_2: '3',
  // },
  symbol: 'ATOM',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_cap: '100000000000',
    deposit_enabled: true,
  },
}

export const axlUSDCAsset: AssetConfig = {
  denom: axlUSDC,
  max_loan_to_value: '0.74',
  liquidation_threshold: '0.75',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  // reserve_factor: '0.2',
  // interest_rate_model: {
  //   optimal_utilization_rate: '0.8',
  //   base: '0',
  //   slope_1: '0.2',
  //   slope_2: '2',
  // },
  symbol: 'axlUSDC',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_cap: '500000000000',
    deposit_enabled: true,
  },
}

export const axlUSDCAssetTest: AssetConfig = {
  denom: usdcTest,
  max_loan_to_value: '0.74',
  liquidation_threshold: '0.75',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  // reserve_factor: '0.2',
  // interest_rate_model: {
  //   optimal_utilization_rate: '0.8',
  //   base: '0',
  //   slope_1: '0.2',
  //   slope_2: '2',
  // },
  symbol: 'axlUSDC',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_cap: '500000000000',
    deposit_enabled: true,
  },
}

export const marsAssetTest: AssetConfig = {
  denom: marsTest,
  max_loan_to_value: '0.74',
  liquidation_threshold: '0.75',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  // reserve_factor: '0.2',
  // interest_rate_model: {
  //   optimal_utilization_rate: '0.8',
  //   base: '0',
  //   slope_1: '0.2',
  //   slope_2: '2',
  // },
  symbol: 'mars',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_cap: '500000000000',
    deposit_enabled: true,
  },
}

// export const osmoOracle: OracleConfig = {
//   denom: 'uosmo',
//   price: '1',
// }

export const atomOracle: OracleConfig = {
  denom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
  price_source: {
    geometric_twap: {
      pool_id: 1,
      window_size: 1800,
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
    },
  },
}

// export const atomOracleTest: OracleConfig = {
//   denom: atomTest,
//   pool_id: 'TBD',
//   window_size: 1800,
//   downtime_detector: { downtime: 'duration30m', recovery: 7200 },
// }

export const axlUSDCOracle: OracleConfig = {
  denom: 'ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858',
  price_source: {
    geometric_twap: {
      pool_id: 678,
      window_size: 1800,
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
    },
  },
}

export const axlUSDCOracleTest: OracleConfig = {
  denom: usdcTest,
  price_source: {
    geometric_twap: {
      pool_id: 5,
      window_size: 1800,
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
    },
  },
}

export const marsOracleTest: OracleConfig = {
  denom: marsTest,
  price_source: {
    geometric_twap: {
      pool_id: 9,
      window_size: 1800,
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
    },
  },
}

// axlUSDC does not have a pool on testnet so config can't have swapRoutes configured correctly
export const osmosisTestnetConfig: DeploymentConfig = {
  oracleName: 'osmosis',
  oracleBaseDenom: 'uusd',
  rewardsCollectorName: 'osmosis',
  atomDenom: atom,
  baseAssetDenom: 'uosmo',
  gasPrice: '0.1uosmo',
  chainId: 'osmo-test-4',
  chainPrefix: 'osmo',
  channelId: 'channel-2083',
  marsDenom: marsTest,
  rewardsCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'https://rpc-test.osmosis.zone',
  safetyFundFeeShare: '0.5',
  deployerMnemonic:
    'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process',
  slippage_tolerance: '0.01',
  base_asset_symbol: 'OSMO',
  runTests: false,
  mainnet: false,
  feeCollectorDenom: marsTest,
  safetyFundDenom: axlUSDC,
  swapRoutes: [
    { denom_in: atom, denom_out: 'uosmo', route: [{ pool_id: 1, token_out_denom: 'uosmo' }] },
  ],
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  swapperDexName: 'osmosis',
  assets: [osmoAsset, atomAsset, axlUSDCAsset],
  vaults: [],
  oracleConfigs: [atomOracle, axlUSDCOracle],
  incentiveEpochDuration: 86400,
  maxWhitelistedIncentiveDenoms: 10,
  targetHealthFactor: '1.2',
}

// axlUSDC does not have a pool on testnet so config can't have swapRoutes configured correctly
export const osmosisTestMultisig: DeploymentConfig = {
  oracleName: 'osmosis',
  oracleBaseDenom: 'uusd',
  rewardsCollectorName: 'osmosis',
  atomDenom: atom,
  baseAssetDenom: 'uosmo',
  gasPrice: '0.1uosmo',
  chainId: 'osmo-test-4',
  chainPrefix: 'osmo',
  channelId: 'channel-2083',
  marsDenom: marsTest,
  rewardsCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'https://rpc-test.osmosis.zone',
  safetyFundFeeShare: '0.5',
  deployerMnemonic:
    'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process',
  slippage_tolerance: '0.01',
  base_asset_symbol: 'OSMO',
  multisigAddr: 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n',
  runTests: false,
  mainnet: false,
  feeCollectorDenom: marsTest,
  safetyFundDenom: axlUSDC,
  swapRoutes: [
    { denom_in: atom, denom_out: 'uosmo', route: [{ pool_id: 1, token_out_denom: 'uosmo' }] },
  ],
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  swapperDexName: 'osmosis',
  assets: [osmoAsset, atomAsset, axlUSDCAsset],
  vaults: [],
  oracleConfigs: [atomOracle, axlUSDCOracle],
  incentiveEpochDuration: 86400,
  maxWhitelistedIncentiveDenoms: 10,
  targetHealthFactor: '1.2',
}

export const osmosisMainnet: DeploymentConfig = {
  oracleName: 'osmosis',
  oracleBaseDenom: 'uusd',
  rewardsCollectorName: 'osmosis',
  atomDenom: atom,
  baseAssetDenom: 'uosmo',
  gasPrice: '0.1uosmo',
  chainId: 'osmosis-1',
  chainPrefix: 'osmo',
  channelId: 'channel-557',
  marsDenom: mars,
  rewardsCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'https://rpc.osmosis.zone',
  safetyFundFeeShare: '0.5',
  deployerMnemonic: 'TO BE INSERTED AT TIME OF DEPLOYMENT',
  slippage_tolerance: '0.01',
  base_asset_symbol: 'OSMO',
  multisigAddr: 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n',
  runTests: false,
  mainnet: true,
  feeCollectorDenom: mars,
  safetyFundDenom: axlUSDC,
  swapRoutes: [
    { denom_in: 'uosmo', denom_out: axlUSDC, route: [{ pool_id: 678, token_out_denom: axlUSDC }] },
    {
      denom_in: atom,
      denom_out: axlUSDC,
      route: [
        { pool_id: 1, token_out_denom: 'uosmo' },
        { pool_id: 678, token_out_denom: axlUSDC },
      ],
    },
    {
      denom_in: 'uosmo',
      denom_out: mars,
      route: [{ pool_id: marsOsmoPool, token_out_denom: mars }],
    },
    {
      denom_in: atom,
      denom_out: mars,
      route: [
        { pool_id: 1, token_out_denom: 'uosmo' },
        { pool_id: marsOsmoPool, token_out_denom: mars },
      ],
    },
    {
      denom_in: axlUSDC,
      denom_out: mars,
      route: [
        { pool_id: 678, token_out_denom: 'uosmo' },
        { pool_id: marsOsmoPool, token_out_denom: mars },
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
  incentiveEpochDuration: 86400,
  maxWhitelistedIncentiveDenoms: 10,
  targetHealthFactor: '1.2',
}

export const osmosisLocalConfig: DeploymentConfig = {
  oracleName: 'osmosis',
  oracleBaseDenom: 'uosmo',
  rewardsCollectorName: 'osmosis',
  atomDenom: atom,
  baseAssetDenom: 'uosmo',
  gasPrice: '0.1uosmo',
  chainId: 'localosmosis',
  chainPrefix: 'osmo',
  channelId: 'channel-1',
  marsDenom: 'umars',
  rewardsCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'http://localhost:26657',
  safetyFundFeeShare: '0.2',
  deployerMnemonic:
    'notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius',
  slippage_tolerance: '0.05',
  base_asset_symbol: 'OSMO',
  runTests: false,
  mainnet: false,
  feeCollectorDenom: axlUSDC,
  safetyFundDenom: axlUSDC,
  swapRoutes: [
    { denom_in: atom, denom_out: 'uosmo', route: [{ pool_id: 1, token_out_denom: 'uosmo' }] },
  ],
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  swapperDexName: 'osmosis',
  assets: [osmoAsset, atomAsset, axlUSDCAsset],
  vaults: [],
  oracleConfigs: [atomOracle, axlUSDCOracle],
  incentiveEpochDuration: 86400,
  maxWhitelistedIncentiveDenoms: 10,
  targetHealthFactor: '1.2',
}
