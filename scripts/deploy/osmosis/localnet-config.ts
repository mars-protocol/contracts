import { DeploymentConfig, AssetConfig, OracleConfig } from '../../types/config'

// Localnet denoms
export const osmo = 'uosmo'
const atom = 'uatom'
const usdc = 'uusdc'

// Pool denoms (these will match the pools created by setup-localnet.ts)
// Note: Pool IDs are sequential starting from 1
const osmoAtomPool = 'gamm/pool/1' // OSMO/ATOM Pool
const osmoUsdcPool = 'gamm/pool/2' // OSMO/USDC Pool
const atomUsdcPool = 'gamm/pool/3' // ATOM/USDC Pool

// Localnet addresses
// Use the validator address from the genesis account
const protocolAdminAddr = 'osmo1ztdmncgxw3xwqdn8vhusfy0u9mne0w629syqsn'

const defaultCreditLine = '1000000000000' // 1M tokens

// ----------------------------------- Markets -----------------------------------

export const osmoAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'OSMO',
  denom: osmo,
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.25',
  liquidation_threshold: '0.7',
  max_loan_to_value: '0.68',
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '10000000000000', // 10M OSMO
  reserve_factor: '0.2',
  interest_rate_model: {
    optimal_utilization_rate: '0.7',
    base: '0',
    slope_1: '0.2',
    slope_2: '3',
  },
}

export const atomAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'ATOM',
  denom: atom,
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.25',
  liquidation_threshold: '0.725',
  max_loan_to_value: '0.7',
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '1000000000000', // 1M ATOM
  reserve_factor: '0.2',
  interest_rate_model: {
    optimal_utilization_rate: '0.7',
    base: '0',
    slope_1: '0.2',
    slope_2: '3',
  },
}

export const usdcAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'USDC',
  denom: usdc,
  liquidation_bonus: {
    max_lb: '0.1',
    min_lb: '0.02',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.25',
  liquidation_threshold: '0.8',
  max_loan_to_value: '0.78',
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '10000000000000', // 10M USDC
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.8',
    base: '0',
    slope_1: '0.1',
    slope_2: '3',
  },
}

export const osmoAtomPoolAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: osmoAtomPool,
  denom: osmoAtomPool,
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.25',
  liquidation_threshold: '0.75',
  max_loan_to_value: '0.73',
  red_bank: {
    borrow_enabled: false,
    deposit_enabled: false,
  },
  deposit_cap: '0',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
}

export const osmoUsdcPoolAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: osmoUsdcPool,
  denom: osmoUsdcPool,
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.25',
  liquidation_threshold: '0.77',
  max_loan_to_value: '0.75',
  red_bank: {
    borrow_enabled: false,
    deposit_enabled: false,
  },
  deposit_cap: '0',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
}

export const atomUsdcPoolAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: atomUsdcPool,
  denom: atomUsdcPool,
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.25',
  liquidation_threshold: '0.77',
  max_loan_to_value: '0.75',
  red_bank: {
    borrow_enabled: false,
    deposit_enabled: false,
  },
  deposit_cap: '0',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
}

// ----------------------------------- Oracles -----------------------------------

export const osmoOracle: OracleConfig = {
  denom: osmo,
  price_source: {
    fixed: {
      price: '1000000', // $1 per OSMO for testing
    },
  },
}

export const atomOracle: OracleConfig = {
  denom: atom,
  price_source: {
    fixed: {
      price: '10000000', // $10 per ATOM for testing
    },
  },
}

export const usdcOracle: OracleConfig = {
  denom: usdc,
  price_source: {
    fixed: {
      price: '1000000', // $1 per USDC
    },
  },
}

export const usdOracle: OracleConfig = {
  denom: 'usd',
  price_source: {
    fixed: {
      price: '1000000',
    },
  },
}

export const osmoAtomPoolOracle: OracleConfig = {
  denom: osmoAtomPool,
  price_source: {
    xyk_liquidity_token: {
      pool_id: 1,
    },
  },
}

export const osmoUsdcPoolOracle: OracleConfig = {
  denom: osmoUsdcPool,
  price_source: {
    xyk_liquidity_token: {
      pool_id: 2,
    },
  },
}

export const atomUsdcPoolOracle: OracleConfig = {
  denom: atomUsdcPool,
  price_source: {
    xyk_liquidity_token: {
      pool_id: 3,
    },
  },
}

// -------------------------------- Deployment Config --------------------------------

export const osmosisLocalnetConfig: DeploymentConfig = {
  mainnet: false,
  deployerMnemonic:
    'bottom loan skill merry east cradle onion journey palm apology verb edit desert impose absurd oil bubble sweet glove shallow size build burst effort',
  multisigAddr: undefined,
  safetyFundAddr: protocolAdminAddr, // Same as admin for localnet
  feeCollectorAddr: protocolAdminAddr, // Same as admin for localnet
  protocolAdminAddr,
  marsDenom: osmo, // Use OSMO as MARS for localnet
  atomDenom: atom,
  chain: {
    baseDenom: osmo,
    defaultGasPrice: 0.025,
    id: 'localosmosis',
    prefix: 'osmo',
    rpcEndpoint: 'http://localhost:26657',
  },
  oracle: {
    name: 'osmosis',
    baseDenom: 'usd',
  },
  rewardsCollector: {
    name: 'osmosis',
    timeoutSeconds: 600,
    channelId: 'channel-0',
    safetyFundFeeShare: '0.5',
    revenueShare: '0.1',
    feeCollectorConfig: {
      target_denom: osmo,
      transfer_type: 'ibc',
    },
    safetyFundConfig: {
      target_denom: usdc, // Must be different from fee_collector_denom
      transfer_type: 'ibc',
    },
    revenueShareConfig: {
      target_denom: usdc,
      transfer_type: 'ibc',
    },
    slippageTolerance: '0.01',
  },
  incentives: {
    epochDuration: 604800, // 1 week (minimum required)
    maxWhitelistedIncentiveDenoms: 10,
  },
  swapper: {
    name: 'osmosis',
    routes: [],
  },
  targetHealthFactor: '1.05',
  creditLineCoins: [
    { denom: osmo, creditLine: defaultCreditLine },
    { denom: atom, creditLine: defaultCreditLine },
    { denom: usdc, creditLine: defaultCreditLine },
  ],
  maxValueForBurn: '10000',
  maxUnlockingPositions: '1',
  maxSlippage: '0.2',
  zapperContractName: 'mars_zapper_osmosis',
  runTests: false,
  assets: [osmoAsset, atomAsset, usdcAsset, osmoAtomPoolAsset, osmoUsdcPoolAsset, atomUsdcPoolAsset],
  vaults: [],
  oracleConfigs: [
    usdOracle,
    osmoOracle,
    atomOracle,
    usdcOracle,
    osmoAtomPoolOracle,
    osmoUsdcPoolOracle,
    atomUsdcPoolOracle,
  ],
}
