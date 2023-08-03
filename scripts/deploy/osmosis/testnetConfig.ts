import { DeploymentConfig, AssetConfig, OracleConfig, VaultConfig } from '../../types/config'

// assets based off of OSMO-TEST-5: https://docs.osmosis.zone/osmosis-core/asset-info/
const uosmo = 'uosmo'
const aUSDC = 'ibc/6F34E1BD664C36CE49ACC28E60D62559A5F96C4F9A6CCE4FC5A67B2852E24CFE' // axelar USDC
const atom = 'ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477'
const mars = 'ibc/2E7368A14AC9AB7870F32CFEA687551C5064FA861868EDF7437BC877358A81F9'
const usdcOsmo = 'gamm/pool/5'
const atomOsmo = 'gamm/pool/12'

const protocolAdminAddr = 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n'

// note the following addresses are all 'mars' bech32 prefix
const safetyFundAddr = 'mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575'
const feeCollectorAddr = 'mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x'

export const osmoAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'OSMO',
  denom: uosmo,
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  liquidation_threshold: '0.61',
  max_loan_to_value: '0.59',
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '2500000000000',
  reserve_factor: '0.2',
  interest_rate_model: {
    optimal_utilization_rate: '0.8',
    base: '0',
    slope_1: '0.2',
    slope_2: '2',
  },
}

export const atomAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'ATOM',
  denom: atom,
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  liquidation_threshold: '0.7',
  max_loan_to_value: '0.68',
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '100000000000',
  reserve_factor: '0.2',
  interest_rate_model: {
    optimal_utilization_rate: '0.8',
    base: '0',
    slope_1: '0.2',
    slope_2: '2',
  },
}

export const USDCAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'aUSDC',
  denom: aUSDC,
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  liquidation_threshold: '0.75',
  max_loan_to_value: '0.74',
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '500000000000',
  reserve_factor: '0.2',
  interest_rate_model: {
    optimal_utilization_rate: '0.8',
    base: '0',
    slope_1: '0.2',
    slope_2: '2',
  },
}

export const usdcOsmoVault: VaultConfig = {
  addr: 'osmo1l3q4mrhkzjyernjhg8lz2t52ddw589y5qc0z7y8y28h6y5wcl46sg9n28j',
  symbol: 'usdcOsmoVault',
  deposit_cap: {
    denom: aUSDC,
    amount: '1000000000',
  },
  liquidation_threshold: '0.65',
  max_loan_to_value: '0.63',
  whitelisted: true,
}

export const atomOsmoVault: VaultConfig = {
  addr: 'osmo1m45ap4rq4m2mfjkcqu9ks9mxmyx2hvx0cdca9sjmrg46q7lghzqqhxxup5',
  symbol: 'atomOsmoVault',
  deposit_cap: {
    denom: aUSDC,
    amount: '1000000000',
  },
  liquidation_threshold: '0.65',
  max_loan_to_value: '0.63',
  whitelisted: true,
}

export const osmoOracle: OracleConfig = {
  denom: uosmo,
  price_source: {
    fixed: {
      price: '1',
    },
  },
}

export const atomOracle: OracleConfig = {
  denom: atom,
  price_source: {
    geometric_twap: {
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
      window_size: 1800,
      pool_id: 12,
    },
  },
}
export const USDCOracle: OracleConfig = {
  denom: aUSDC,
  price_source: {
    staked_geometric_twap: {
      transitive_denom: uosmo,
      pool_id: 5,
      window_size: 1800,
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
    },
  },
}

export const usdcOsmoOracle: OracleConfig = {
  denom: usdcOsmo,
  price_source: {
    xyk_liquidity_token: {
      pool_id: 5,
    },
  },
}

export const atomOsmoOracle: OracleConfig = {
  denom: atomOsmo,
  price_source: {
    xyk_liquidity_token: {
      pool_id: 12,
    },
  },
}

export const osmosisTestnetConfig = {
  oracleName: 'osmosis',
  oracleBaseDenom: 'uosmo',
  rewardsCollectorName: 'osmosis',
  atomDenom: atom,
  baseAssetDenom: uosmo,
  gasPrice: '0.1uosmo',
  chainId: 'osmo-test-5',
  chainPrefix: 'osmo',
  channelId: 'channel-2083',
  marsDenom: mars,
  rewardsCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'https://rpc.osmotest5.osmosis.zone',
  safetyFundFeeShare: '0.5',
  deployerMnemonic:
    'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process',
  slippage_tolerance: '0.01',
  base_asset_symbol: 'OSMO',
  runTests: true,
  mainnet: false,
  feeCollectorDenom: mars,
  safetyFundDenom: aUSDC,
  swapRoutes: [
    { denom_in: atom, denom_out: uosmo, route: [{ pool_id: 12, token_out_denom: uosmo }] },
  ],
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  swapperDexName: 'osmosis',
  assets: [osmoAsset, atomAsset, USDCAsset],
  vaults: [usdcOsmoVault, atomOsmoVault],
  oracleConfigs: [osmoOracle, atomOracle, USDCOracle, atomOsmoOracle, usdcOsmoOracle],
  targetHealthFactor: '1.2',
  incentiveEpochDuration: 604800, // 1 week
  maxWhitelistedIncentiveDenoms: 10,
}

export const osmosisTestMultisig: DeploymentConfig = {
  oracleName: 'osmosis',
  oracleBaseDenom: 'uusd',
  rewardsCollectorName: 'osmosis',
  atomDenom: atom,
  baseAssetDenom: 'uosmo',
  gasPrice: '0.1uosmo',
  chainId: 'osmo-test-5',
  chainPrefix: 'osmo',
  channelId: 'channel-2083',
  marsDenom: mars,
  rewardsCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'https://rpc.osmotest5.osmosis.zone',
  safetyFundFeeShare: '0.5',
  deployerMnemonic:
    'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process',
  slippage_tolerance: '0.01',
  base_asset_symbol: 'OSMO',
  multisigAddr: 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n',
  runTests: false,
  mainnet: false,
  feeCollectorDenom: mars,
  safetyFundDenom: aUSDC,
  swapRoutes: [
    { denom_in: atom, denom_out: 'uosmo', route: [{ pool_id: 12, token_out_denom: 'uosmo' }] },
  ],
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  swapperDexName: 'osmosis',
  assets: [osmoAsset, atomAsset, USDCAsset],
  vaults: [usdcOsmoVault, atomOsmoVault],
  oracleConfigs: [osmoOracle, atomOracle, USDCOracle, atomOsmoOracle, usdcOsmoOracle],
  targetHealthFactor: '1.2',
  incentiveEpochDuration: 604800, // 1 week
  maxWhitelistedIncentiveDenoms: 10,
}
