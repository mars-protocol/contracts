import { DeploymentConfig, AssetConfig, OracleConfig } from '../../types/config'
import { NeutronIbcConfig } from '../../types/generated/mars-rewards-collector-base/MarsRewardsCollectorBase.types'

const nobleUsdcDenom = 'ibc/B559A80D62249C8AA07A380E2A2BEA6E5CA9A6F079C912C3A9E9B494105E4F81'
const atomDenom = 'ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9'
const marsDenom = 'ibc/9598CDEB7C6DB7FC21E746C8E0250B30CD5154F39CA111A9D4948A4362F638BD'
const dAtomDenom =
  'factory/neutron1k6hr0f83e7un2wjf29cspk7j69jrnskk65k3ek2nj9dztrlzpj6q00rtsa/udatom'

const dAtomUsdcLpDenom =
  'factory/neutron1nfns3ck2ykrs0fknckrzd9728cyf77devuzernhwcwrdxw7ssk2s3tjf8r/astroport/share'
const dAtomUsdcPairAddr = 'neutron1nfns3ck2ykrs0fknckrzd9728cyf77devuzernhwcwrdxw7ssk2s3tjf8r'
const dAtomAtomLpDenom =
  'factory/neutron1yem82r0wf837lfkwvcu2zxlyds5qrzwkz8alvmg0apyrjthk64gqeq2e98/astroport/share'
const dAtomAtomPairAddr = 'neutron1yem82r0wf837lfkwvcu2zxlyds5qrzwkz8alvmg0apyrjthk64gqeq2e98'
const dAtomNtrnLpDenom =
  'factory/neutron1ke92yjl47eqy0mpgn9x4xups4szsm0ql6xhn4htw9zgn9wl5gm0quzh6ch/astroport/share'
const dAtomNtrnPairAddr = 'neutron1ke92yjl47eqy0mpgn9x4xups4szsm0ql6xhn4htw9zgn9wl5gm0quzh6ch'
const ntrnUsdcLpDenom =
  'factory/neutron18c8qejysp4hgcfuxdpj4wf29mevzwllz5yh8uayjxamwtrs0n9fshq9vtv/astroport/share'
const ntrnUsdcPairAddr = 'neutron18c8qejysp4hgcfuxdpj4wf29mevzwllz5yh8uayjxamwtrs0n9fshq9vtv'

const redemptionRateContractAddr =
  'neutron16cdl2nd8wtaggvgsczuqe38xndhdaf5znfmqttcl6krjj262c6ys62ldmr'

const protocolAdminAddr = 'neutron1ltzuv25ltw9mkwuvvmt7e54a6ene283hfj7l0c'

const marsNeutronChannelId = 'channel-16'
const chainId = 'neutron-1'
const rpcEndpoint = 'http://135.181.139.174:26657'

// Astroport configuration https://github.com/astroport-fi/astroport-changelog/blob/main/neutron/neutron-1/core_mainnet.json
const astroportFactory = 'neutron1hptk0k5kng7hjy35vmh009qd5m6l33609nypgf2yc6nqnewduqasxplt4e'
const astroportRouter = 'neutron1rwj6mfxzzrwskur73v326xwuff52vygqk73lr7azkehnfzz5f5wskwekf4'
const astroportIncentives = 'neutron173fd8wpfzyqnfnpwq2zhtgdstujrjz2wkprkjfr6gqg4gknctjyq6m3tch'

// note the following three addresses are all 'mars' bech32 prefix
const safetyFundAddr = 'mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575'
const feeCollectorAddr = 'mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x'

// Pyth configuration
const pythAddr = 'neutron1m2emc93m9gpwgsrsf2vylv9xvgqh654630v7dfrhrkmr5slly53spg85wv'
const pythAtomID = 'b00b60f88b03a6a625a8d1c048c3f66653edf217439983d037e7222c4e612819'
const pythUsdcID = 'eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a'
const pythNtrnID = 'a8e6517966a52cb1df864b2764f3629fde3f21d2b640b5c572fcd654cbccd65e'

// IBC config for rewards-collector. See https://rest-palvus.pion-1.ntrn.tech/neutron-org/neutron/feerefunder/params
export const neutronIbcConfig: NeutronIbcConfig = {
  source_port: 'transfer',
  acc_fee: [
    {
      denom: 'untrn',
      amount: '1000',
    },
  ],
  timeout_fee: [
    {
      denom: 'untrn',
      amount: '1000',
    },
  ],
}

// Oracle configurations
export const ntrnOracle: OracleConfig = {
  denom: 'untrn',
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythNtrnID,
      denom_decimals: 6,
      max_staleness: 300, // 5 minutes
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const atomOracle: OracleConfig = {
  denom: atomDenom,
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythAtomID,
      denom_decimals: 6,
      max_staleness: 300, // 5 minutes
      max_confidence: '0.1',
      max_deviation: '0.15',
    },
  },
}

export const nobleUSDCOracle: OracleConfig = {
  denom: nobleUsdcDenom,
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythUsdcID,
      denom_decimals: 6,
      max_staleness: 300, // 5 minutes
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

export const dAtomOracle: OracleConfig = {
  denom: dAtomDenom,
  price_source: {
    lsd: {
      transitive_denom: atomDenom,
      redemption_rate: {
        contract_addr: redemptionRateContractAddr,
        max_staleness: 93600,
      },
      twap: {
        pair_address: dAtomAtomPairAddr,
        window_size: 1800,
        tolerance: 120,
      },
    },
  },
}

export const dAtomUsdcLpOracle: OracleConfig = {
  denom: dAtomUsdcLpDenom,
  price_source: {
    pcl_liquidity_token: {
      pair_address: dAtomUsdcPairAddr,
    },
  },
}

export const dAtomAtomLpOracle: OracleConfig = {
  denom: dAtomAtomLpDenom,
  price_source: {
    pcl_liquidity_token: {
      pair_address: dAtomAtomPairAddr,
    },
  },
}

export const dAtomNtrnLpOracle: OracleConfig = {
  denom: dAtomNtrnLpDenom,
  price_source: {
    pcl_liquidity_token: {
      pair_address: dAtomNtrnPairAddr,
    },
  },
}

export const ntrnUsdcLpOracle: OracleConfig = {
  denom: ntrnUsdcLpDenom,
  price_source: {
    pcl_liquidity_token: {
      pair_address: ntrnUsdcPairAddr,
    },
  },
}

// Asset configurations
export const ntrnAsset: AssetConfig = {
  denom: 'untrn',
  max_loan_to_value: '0.54',
  liquidation_threshold: '0.55',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'NTRN',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '5000000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
}

export const atomAsset: AssetConfig = {
  denom: atomDenom,
  max_loan_to_value: '0.74',
  liquidation_threshold: '0.75',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'ATOM',
  credit_manager: {
    whitelisted: true,
    hls: {
      max_loan_to_value: '0.86',
      liquidation_threshold: '0.865',
      correlations: [{ coin: { denom: dAtomDenom } }, { coin: { denom: dAtomAtomLpDenom } }],
    },
  },
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '150000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.8',
    base: '0',
    slope_1: '0.14',
    slope_2: '3',
  },
}

export const nobleUSDCAsset: AssetConfig = {
  denom: nobleUsdcDenom,
  max_loan_to_value: '0.795',
  liquidation_threshold: '0.8',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'nobleUSDC',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: true,
    deposit_enabled: true,
  },
  deposit_cap: '500000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.8',
    base: '0',
    slope_1: '0.2',
    slope_2: '2',
  },
}

export const dAtomAsset: AssetConfig = {
  denom: dAtomDenom,
  max_loan_to_value: '0.62',
  liquidation_threshold: '0.65',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.25',
  symbol: 'ATOM',
  credit_manager: {
    whitelisted: true,
    hls: {
      max_loan_to_value: '0.86',
      liquidation_threshold: '0.865',
      correlations: [{ coin: { denom: atomDenom } }, { coin: { denom: dAtomAtomLpDenom } }],
    },
  },
  red_bank: {
    borrow_enabled: false,
    deposit_enabled: true,
  },
  deposit_cap: '50000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
}

export const dAtomUsdcLpAsset: AssetConfig = {
  denom: dAtomUsdcLpDenom,
  max_loan_to_value: '0.61',
  liquidation_threshold: '0.63',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.25',
  symbol: 'PCL_LP_dATOM_USDC',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: false,
    deposit_enabled: true,
  },
  deposit_cap: '1000000000000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
}

export const dAtomNtrnLpAsset: AssetConfig = {
  denom: dAtomNtrnLpDenom,
  max_loan_to_value: '0.50',
  liquidation_threshold: '0.53',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.25',
  symbol: 'PCL_LP_dATOM_NTRN',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: false,
    deposit_enabled: true,
  },
  deposit_cap: '1000000000000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
}

export const ntrnUsdcLpAsset: AssetConfig = {
  denom: ntrnUsdcLpDenom,
  max_loan_to_value: '0.66',
  liquidation_threshold: '0.68',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.25',
  symbol: 'PCL_LP_NTRN_USDC',
  credit_manager: {
    whitelisted: true,
  },
  red_bank: {
    borrow_enabled: false,
    deposit_enabled: true,
  },
  deposit_cap: '1000000000000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
}

export const dAtomAtomLpAsset: AssetConfig = {
  denom: dAtomAtomLpDenom,
  max_loan_to_value: '0',
  liquidation_threshold: '0.01',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.25',
  symbol: 'PCL_LP_dATOM_ATOM',
  credit_manager: {
    whitelisted: false,
    hls: {
      max_loan_to_value: '0.86',
      liquidation_threshold: '0.865',
      correlations: [{ coin: { denom: dAtomDenom } }, { coin: { denom: atomDenom } }],
    },
  },
  red_bank: {
    borrow_enabled: false,
    deposit_enabled: false,
  },
  deposit_cap: '1000000000000000000',
  reserve_factor: '0.1',
  interest_rate_model: {
    optimal_utilization_rate: '0.6',
    base: '0',
    slope_1: '0.15',
    slope_2: '3',
  },
}

export const neutronDevnetConfig: DeploymentConfig = {
  mainnet: false,
  deployerMnemonic: 'TO BE INSERTED AT TIME OF DEPLOYMENT',
  marsDenom: marsDenom,
  atomDenom: atomDenom,
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  chain: {
    baseDenom: 'untrn',
    defaultGasPrice: 0.02,
    id: chainId,
    prefix: 'neutron',
    rpcEndpoint: rpcEndpoint,
  },
  oracle: {
    name: 'wasm',
    baseDenom: 'uusd',
    customInitParams: {
      astroport_factory: astroportFactory,
    },
  },
  rewardsCollector: {
    name: 'neutron',
    timeoutSeconds: 600,
    channelId: marsNeutronChannelId,
    safetyFundFeeShare: '0.5',
    feeCollectorDenom: marsDenom,
    safetyFundDenom: nobleUsdcDenom,
    slippageTolerance: '0.01',
    neutronIbcConfig: neutronIbcConfig,
  },
  incentives: {
    epochDuration: 604800, // 1 week
    maxWhitelistedIncentiveDenoms: 10,
  },
  swapper: {
    name: 'astroport',
    routes: [],
  },
  targetHealthFactor: '1.05',
  creditLineCoins: [],
  maxValueForBurn: '10000',
  maxUnlockingPositions: '1',
  maxSlippage: '0.2',
  zapperContractName: 'mars_zapper_astroport',
  runTests: false,
  assets: [
    ntrnAsset,
    atomAsset,
    nobleUSDCAsset,
    dAtomAsset,
    dAtomUsdcLpAsset,
    dAtomNtrnLpAsset,
    ntrnUsdcLpAsset,
    dAtomAtomLpAsset,
  ],
  vaults: [],
  oracleConfigs: [
    usdOracle,
    nobleUSDCOracle,
    atomOracle,
    ntrnOracle,
    dAtomOracle,
    dAtomUsdcLpOracle,
    dAtomNtrnLpOracle,
    ntrnUsdcLpOracle,
    dAtomAtomLpOracle,
  ],
  astroportConfig: {
    factory: astroportFactory,
    router: astroportRouter,
    incentives: astroportIncentives,
  },
}
