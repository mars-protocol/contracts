import { DeploymentConfig, AssetConfig, OracleConfig, VaultConfig } from '../../types/config'

// assets based off of OSMO-TEST-5: https://docs.osmosis.zone/osmosis-core/asset-info/
const osmo = 'uosmo'
const atom = 'ibc/A8C2D23A1E6F95DA4E48BA349667E322BD7A6C996D8A4AAE8BA72E190F3D1477'
const nUSDC = 'ibc/40F1B2458AEDA66431F9D44F48413240B8D28C072463E2BF53655728683583E3' // noble
const mars = 'ibc/2E7368A14AC9AB7870F32CFEA687551C5064FA861868EDF7437BC877358A81F9'

const pythContractAddr = 'UPDATE'
const protocolAdminAddr = 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n'

// note the following addresses are all 'mars' bech32 prefix
const safetyFundAddr = 'mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575'
const feeCollectorAddr = 'mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x'

export const osmoAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'OSMO',
  denom: osmo,
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
}

export const USDCAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'nUSDC',
  denom: nUSDC,
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
}

export const usdcOsmoVault: VaultConfig = {
  addr: 'osmo1fmq9hw224fgz8lk48wyd0gfg028kvvzggt6c3zvnaqkw23x68cws5nd5em',
  symbol: 'usdcOsmoVault',
  deposit_cap: {
    denom: nUSDC,
    amount: '1000000000',
  },
  liquidation_threshold: '0.65',
  max_loan_to_value: '0.63',
  whitelisted: true,
}

export const atomOracle: OracleConfig = {
  denom: atom,
  price_source: {
    pyth: {
      contract_addr: pythContractAddr,
      price_feed_id: 'UPDATE',
      max_staleness: 60,
      denom_decimals: 6,
      max_confidence: '5',
      max_deviation: '4',
    },
  },
}

export const USDCOracle: OracleConfig = {
  denom: nUSDC,
  price_source: {
    staked_geometric_twap: {
      transitive_denom: osmo,
      pool_id: 6,
      window_size: 1800,
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
    },
  },
}

export const osmosisTestnetConfig: DeploymentConfig = {
  oracleName: 'osmosis',
  oracleBaseDenom: 'uusd',
  rewardsCollectorName: 'osmosis',
  atomDenom: atom,
  baseAssetDenom: osmo,
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
  runTests: false,
  mainnet: false,
  feeCollectorDenom: mars,
  safetyFundDenom: nUSDC,
  swapRoutes: [
    { denom_in: atom, denom_out: osmo, route: [{ pool_id: 12, token_out_denom: osmo }] },
  ],
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  swapperDexName: 'osmosis',
  assets: [osmoAsset, atomAsset, USDCAsset],
  vaults: [usdcOsmoVault],
  oracleConfigs: [atomOracle, USDCOracle],
  targetHealthFactor: '1.2',
  incentiveEpochDuration: 86400,
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
  safetyFundDenom: nUSDC,
  swapRoutes: [
    { denom_in: atom, denom_out: 'uosmo', route: [{ pool_id: 1, token_out_denom: 'uosmo' }] },
  ],
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  swapperDexName: 'osmosis',
  assets: [osmoAsset, atomAsset, USDCAsset],
  vaults: [usdcOsmoVault],
  oracleConfigs: [atomOracle, USDCOracle],
  targetHealthFactor: '1.2',
  incentiveEpochDuration: 86400,
  maxWhitelistedIncentiveDenoms: 10,
}
