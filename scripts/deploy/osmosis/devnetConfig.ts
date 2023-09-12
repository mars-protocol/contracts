import { DeploymentConfig, AssetConfig, OracleConfig, VaultConfig } from '../../types/config'

// Mainnet markets:
const osmo = 'uosmo'
const atom = 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2'
const axl = 'ibc/903A61A498756EA560B85A85132D3AEE21B5DEDD41213725D22ABF276EA6945E'
const stAtom = 'ibc/C140AFD542AE77BD7DCC83F13FDD8C5E5BB8C4929785E6EC2F4C636F98F17901'
const wbtc = 'ibc/D1542AA8762DB13087D8364F3EA6509FD6F009A34F00426AF9E4F9FA85CBBF1F'
const axlUSDC = 'ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858'
const eth = 'ibc/EA1D43981D5C9A1C4AAEA9C23BB1D4FA126BA9BC7020A25E0AE4AA841EA25DC5'
const atomOsmoPool = 'gamm/pool/1'
const usdcOsmoPool = 'gamm/pool/678'
const ethOsmoPool = 'gamm/pool/704'
const wbtcOsmoPool = 'gamm/pool/712'
const atomStAtomPool = 'gamm/pool/803'

const mars = 'ibc/573FCD90FACEE750F55A8864EF7D38265F07E5A9273FA0E8DAFD39951332B580'

const pythContractAddr = 'osmo13ge29x4e2s63a8ytz2px8gurtyznmue4a69n5275692v3qn3ks8q7cwck7'
const protocolAdminAddr = 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n'

// note the following addresses are all 'mars' bech32 prefix
const safetyFundAddr = 'mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575'
const feeCollectorAddr = 'mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x'

// ----------------------------------- Markets -----------------------------------

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
  protocol_liquidation_fee: '0.1',
  liquidation_threshold: '0.75',
  max_loan_to_value: '0.73',
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '10000000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
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
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.1',
  liquidation_threshold: '0.75',
  max_loan_to_value: '0.74',
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '700000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.7',
    base: '0',
    slope_1: '0.2',
    slope_2: '3',
  },
}

export const axlAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'AXL',
  denom: axl,
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.1',
  liquidation_threshold: '0.45',
  max_loan_to_value: '0.44',
  red_bank: {
    borrow_enabled: false,
    deposit_enabled: true,
  },
  deposit_cap: '400000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.17',
    slope_2: '3',
  },
}

export const stAtomAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'stATOM',
  denom: stAtom,
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.1',
  liquidation_threshold: '0.55',
  max_loan_to_value: '0.545',
  red_bank: {
    borrow_enabled: false,
    deposit_enabled: true,
  },
  deposit_cap: '200000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.1',
    slope_2: '3',
  },
}

export const wbtcAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'wBTC',
  denom: wbtc,
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.1',
  liquidation_threshold: '0.8',
  max_loan_to_value: '0.78',
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '2000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.1',
    slope_2: '3',
  },
}

export const axlUSDCAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'axlUSDC',
  denom: axlUSDC,
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.1',
  liquidation_threshold: '0.8',
  max_loan_to_value: '0.795',
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '3000000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.8',
    base: '0',
    slope_1: '0.125',
    slope_2: '2',
  },
}

export const ethAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'ETH',
  denom: eth,
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.1',
  liquidation_threshold: '0.8',
  max_loan_to_value: '0.78',
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '300000000000000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.1',
    slope_2: '3',
  },
}

export const atomOsmoVault: VaultConfig = {
  addr: 'osmo1g3kmqpp8608szfp0pdag3r6z85npph7wmccat8lgl3mp407kv73qlj7qwp',
  symbol: 'atomOsmoVault',
  deposit_cap: {
    denom: axlUSDC,
    amount: '2000000000000',
  },
  liquidation_threshold: '0.75',
  max_loan_to_value: '0.73',
  whitelisted: true,
}

export const usdcOsmoVault: VaultConfig = {
  addr: 'osmo1jfmwayj8jqp9tfy4v4eks5c2jpnqdumn8x8xvfllng0wfes770qqp7jl4j',
  symbol: 'usdcOsmoVault',
  deposit_cap: {
    denom: axlUSDC,
    amount: '750000000000',
  },
  liquidation_threshold: '0.77',
  max_loan_to_value: '0.75',
  whitelisted: true,
}

export const ethOsmoVault: VaultConfig = {
  addr: 'osmo1r235f4tdkwrsnj3mdm9hf647l754y6g6xsmz0nas5r4vr5tda3qsgtftef',
  symbol: 'ethOsmoVault',
  deposit_cap: {
    denom: axlUSDC,
    amount: '500000000000',
  },
  liquidation_threshold: '0.77',
  max_loan_to_value: '0.75',
  whitelisted: true,
}

export const wbtcOsmoVault: VaultConfig = {
  addr: 'osmo185gqewrlde8vrqw7j8lpad67v8jfrx9u7770k9q87tqqecctp5tq50wt2c',
  symbol: 'wbtcOsmoVault',
  deposit_cap: {
    denom: axlUSDC,
    amount: '250000000000',
  },
  liquidation_threshold: '0.77',
  max_loan_to_value: '0.75',
  whitelisted: true,
}

export const atomStAtomVault: VaultConfig = {
  addr: 'osmo1a6tcf60pyz8qq2n532dzcs7s7sj8klcmra04tvaqympzcvxqg9esn7xz7l',
  symbol: 'atomStAtomVault',
  deposit_cap: {
    denom: axlUSDC,
    amount: '3000000000000',
  },
  liquidation_threshold: '0.65',
  max_loan_to_value: '0.64',
  whitelisted: true,
}

// ----------------------------------- Oracle -----------------------------------

export const atomOsmoOracle: OracleConfig = {
  denom: atomOsmoPool,
  price_source: {
    xyk_liquidity_token: {
      pool_id: 1,
    },
  },
}

export const usdcOsmoOracle: OracleConfig = {
  denom: usdcOsmoPool,
  price_source: {
    xyk_liquidity_token: {
      pool_id: 678,
    },
  },
}

export const ethOsmoOracle: OracleConfig = {
  denom: ethOsmoPool,
  price_source: {
    xyk_liquidity_token: {
      pool_id: 704,
    },
  },
}

export const wbtcOsmoOracle: OracleConfig = {
  denom: wbtcOsmoPool,
  price_source: {
    xyk_liquidity_token: {
      pool_id: 712,
    },
  },
}

export const atomStAtomOracle: OracleConfig = {
  denom: atomStAtomPool,
  price_source: {
    xyk_liquidity_token: {
      pool_id: 803,
    },
  },
}

export const atomOracle: OracleConfig = {
  denom: atom,
  price_source: {
    pyth: {
      contract_addr: pythContractAddr,
      price_feed_id: 'b00b60f88b03a6a625a8d1c048c3f66653edf217439983d037e7222c4e612819',
      max_staleness: 60,
      denom_decimals: 6,
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const axlOracle: OracleConfig = {
  denom: axl,
  price_source: {
    pyth: {
      contract_addr: pythContractAddr,
      price_feed_id: '60144b1d5c9e9851732ad1d9760e3485ef80be39b984f6bf60f82b28a2b7f126',
      max_staleness: 60,
      denom_decimals: 6,
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const stAtomOracle: OracleConfig = {
  denom: stAtom,
  price_source: {
    staked_geometric_twap: {
      downtime_detector: {
        downtime: 'duration30m',
        recovery: 7200,
      },
      pool_id: 803,
      transitive_denom: atom,
      window_size: 1800,
    },
  },
}

export const wbtcOracle: OracleConfig = {
  denom: wbtc,
  price_source: {
    pyth: {
      contract_addr: pythContractAddr,
      price_feed_id: 'e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43',
      max_staleness: 60,
      denom_decimals: 8,
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const axlUSDCOracle: OracleConfig = {
  denom: axlUSDC,
  price_source: {
    pyth: {
      contract_addr: pythContractAddr,
      price_feed_id: 'eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a',
      max_staleness: 60,
      denom_decimals: 6,
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const ethOracle: OracleConfig = {
  denom: eth,
  price_source: {
    pyth: {
      contract_addr: pythContractAddr,
      price_feed_id: 'ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace',
      max_staleness: 60,
      denom_decimals: 18,
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const osmoOracle: OracleConfig = {
  denom: osmo,
  price_source: {
    pyth: {
      contract_addr: pythContractAddr,
      price_feed_id: '5867f5683c757393a0670ef0f701490950fe93fdb006d181c8265a831ac0c5c6',
      max_staleness: 60,
      denom_decimals: 6,
      max_confidence: '0.1',
      max_deviation: '0.15',
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

// TWAP

export const atomOracleTwap: OracleConfig = {
  denom: atom,
  price_source: {
    geometric_twap: {
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
      window_size: 1800,
      pool_id: 1,
    },
  },
}

export const axlOracleTwap: OracleConfig = {
  denom: axl,
  price_source: {
    geometric_twap: {
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
      window_size: 1800,
      pool_id: 812,
    },
  },
}

export const stAtomOracleTwap: OracleConfig = {
  denom: stAtom,
  price_source: {
    staked_geometric_twap: {
      downtime_detector: {
        downtime: 'duration30m',
        recovery: 7200,
      },
      pool_id: 803,
      transitive_denom: atom,
      window_size: 1800,
    },
  },
}

export const wbtcOracleTwap: OracleConfig = {
  denom: wbtc,
  price_source: {
    geometric_twap: {
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
      window_size: 1800,
      pool_id: 712,
    },
  },
}

export const axlUSDCOracleTwap: OracleConfig = {
  denom: axlUSDC,
  price_source: {
    geometric_twap: {
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
      window_size: 1800,
      pool_id: 678,
    },
  },
}

export const ethOracleTwap: OracleConfig = {
  denom: eth,
  price_source: {
    geometric_twap: {
      downtime_detector: { downtime: 'duration30m', recovery: 7200 },
      window_size: 1800,
      pool_id: 704,
    },
  },
}

export const osmoOracleTwap: OracleConfig = {
  denom: osmo,
  price_source: {
    fixed: {
      price: '1',
    },
  },
}

// ----------------------------------- Deployment -----------------------------------

export const osmosisDevnet: DeploymentConfig = {
  oracleName: 'osmosis',
  oracleBaseDenom: 'uusd',
  // oracleBaseDenom: 'uosmo',
  rewardsCollectorName: 'osmosis',
  atomDenom: atom,
  baseAssetDenom: osmo,
  gasPrice: '0.1uosmo',
  chainId: 'devnet',
  chainPrefix: 'osmo',
  channelId: 'channel-557',
  marsDenom: mars,
  rewardsCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'https://rpc.devnet.osmosis.zone',
  safetyFundFeeShare: '0.5',
  deployerMnemonic:
    'TODO',
  slippage_tolerance: '0.01',
  base_asset_symbol: 'OSMO',
  // multisigAddr: 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n',
  runTests: false,
  mainnet: false,
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
  assets: [osmoAsset, atomAsset, axlAsset, stAtomAsset, wbtcAsset, axlUSDCAsset, ethAsset],
  vaults: [atomOsmoVault, usdcOsmoVault, ethOsmoVault, wbtcOsmoVault, atomStAtomVault],
  oracleConfigs: [
    usdOracle,
    osmoOracle,
    atomOracle,
    axlOracle,
    stAtomOracle,
    wbtcOracle,
    axlUSDCOracle,
    ethOracle,
    atomOsmoOracle,
    usdcOsmoOracle,
    ethOsmoOracle,
    wbtcOsmoOracle,
    atomStAtomOracle,
  ],
  // oracleConfigs: [osmoOracleTwap, atomOracleTwap, axlOracleTwap, stAtomOracleTwap, wbtcOracleTwap, axlUSDCOracleTwap, ethOracleTwap, atomOsmoOracle, usdcOsmoOracle, ethOsmoOracle, wbtcOsmoOracle, atomStAtomOracle],
  targetHealthFactor: '1.2',
  incentiveEpochDuration: 604800, // 1 week
  maxWhitelistedIncentiveDenoms: 10,
}
