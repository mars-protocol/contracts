import {
  DeploymentConfig,
  AssetConfig,
  OracleConfig,
  VaultConfig,
  VaultType,
} from '../../types/config'

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

const defaultCreditLine = '100000000000'

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
  symbol: 'usdcOsmoVault',
  vault: {
    addr: 'osmo1l3q4mrhkzjyernjhg8lz2t52ddw589y5qc0z7y8y28h6y5wcl46sg9n28j',
    deposit_cap: {
      denom: aUSDC,
      amount: '1000000000',
    },
    liquidation_threshold: '0.65',
    max_loan_to_value: '0.63',
    whitelisted: true,
  },
}

export const atomOsmoVault: VaultConfig = {
  symbol: 'atomOsmoVault',
  vault: {
    addr: 'osmo1m45ap4rq4m2mfjkcqu9ks9mxmyx2hvx0cdca9sjmrg46q7lghzqqhxxup5',
    deposit_cap: {
      denom: aUSDC,
      amount: '1000000000',
    },
    liquidation_threshold: '0.65',
    max_loan_to_value: '0.63',
    whitelisted: true,
  },
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
      downtime_detector: { downtime: 'Duration30m', recovery: 7200 },
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
      downtime_detector: { downtime: 'Duration30m', recovery: 7200 },
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

const testActions = {
  vault: {
    depositAmount: '1000000',
    withdrawAmount: '1000000',
    mock: {
      config: {
        deposit_cap: { denom: aUSDC, amount: '100000000' }, // 100 usdc
        liquidation_threshold: '0.585',
        max_loan_to_value: '0.569',
        whitelisted: true,
      },
      vaultTokenDenom: uosmo,
      type: VaultType.LOCKED,
      lockup: { time: 900 }, // 15 mins
      baseToken: usdcOsmo,
    },
  },
  borrowAmount: '10',
  repayAmount: '11',
  depositAmount: '100',
  lendAmount: '10',
  reclaimAmount: '5',
  secondaryDenom: aUSDC,
  startingAmountForTestUser: '4000000',
  swap: {
    slippage: '0.4',
    amount: '40',
    route: [
      {
        token_out_denom: aUSDC,
        pool_id: '1',
      },
    ],
  },
  unzapAmount: '1000000',
  withdrawAmount: '12',
  zap: {
    coinsIn: [
      {
        denom: aUSDC,
        amount: '1',
      },
      { denom: uosmo, amount: '3' },
    ],
    denomOut: usdcOsmo,
  },
}

export const osmosisTestnetConfig: DeploymentConfig = {
  mainnet: false,
  deployerMnemonic: 'TO BE INSERTED AT TIME OF DEPLOYMENT',
  marsDenom: mars,
  atomDenom: atom,
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  chain: {
    baseDenom: uosmo,
    defaultGasPrice: 0.1,
    id: 'osmo-test-5',
    prefix: 'osmo',
    rpcEndpoint: 'https://rpc.osmotest5.osmosis.zone',
  },
  oracle: {
    name: 'osmosis',
    baseDenom: 'uosmo',
  },
  rewardsCollector: {
    name: 'osmosis',
    timeoutSeconds: 600,
    channelId: 'channel-2083',
    safetyFundFeeShare: '0.5',
    feeCollectorDenom: mars,
    safetyFundDenom: aUSDC,
    slippageTolerance: '0.01',
  },
  incentives: {
    epochDuration: 604800, // 1 week
    maxWhitelistedIncentiveDenoms: 10,
  },
  swapper: {
    name: 'osmosis',
    routes: [
      { denom_in: atom, denom_out: uosmo, route: [{ pool_id: 12, token_out_denom: uosmo }] },
      { denom_in: uosmo, denom_out: atom, route: [{ pool_id: 12, token_out_denom: atom }] },
      { denom_in: aUSDC, denom_out: uosmo, route: [{ pool_id: 5, token_out_denom: uosmo }] },
      { denom_in: uosmo, denom_out: aUSDC, route: [{ pool_id: 5, token_out_denom: aUSDC }] },
    ],
  },
  targetHealthFactor: '1.05',
  creditLineCoins: [
    { denom: uosmo, creditLine: defaultCreditLine },
    { denom: aUSDC, creditLine: defaultCreditLine },
    { denom: usdcOsmo, creditLine: defaultCreditLine },
  ],
  maxValueForBurn: '10000',
  maxUnlockingPositions: '1',
  maxSlippage: '0.2',
  zapperContractName: 'mars_zapper_osmosis',
  runTests: true,
  testActions: testActions,
  assets: [osmoAsset, atomAsset, USDCAsset],
  vaults: [usdcOsmoVault, atomOsmoVault],
  oracleConfigs: [osmoOracle, atomOracle, USDCOracle, atomOsmoOracle, usdcOsmoOracle],
}
