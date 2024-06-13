import { DeploymentConfig, AssetConfig, OracleConfig } from '../../types/config'
import { NeutronIbcConfig } from '../../types/generated/mars-rewards-collector-base/MarsRewardsCollectorBase.types'

const nobleUsdcDenom = 'ibc/4C19E7EC06C1AB2EC2D70C6855FEB6D48E9CE174913991DA0A517D21978E7E42'
const atomDenom = 'ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9'
const marsDenom = 'ibc/584A4A23736884E0C198FD1EE932455A9357A492A7B94324E4A02B5628687831'

const pclLpMarsUsdcDenom =
  'factory/neutron1sf456kx85dz0wfjs4sx0s80dyzmc360pfc0rdzactxt8xrse9ykqsdpy2y/astroport/share'
const pclLpMarsDenom = 'factory/neutron1wm8jd0hrw79pfhhm9xmuq43jwz4wtukvxfgkkw/mars'
const pclLpUsdcDenom = 'factory/neutron1wm8jd0hrw79pfhhm9xmuq43jwz4wtukvxfgkkw/usdc'
const pclLpMarsUsdcPairAddr = 'neutron1sf456kx85dz0wfjs4sx0s80dyzmc360pfc0rdzactxt8xrse9ykqsdpy2y'

const protocolAdminAddr = 'neutron1ke0vqqzyymlp5esr8gjwuzh94ysnpvj8er5hm7'

const marsNeutronChannelId = 'channel-97'
const chainId = 'pion-1'
const rpcEndpoint = 'https://rpc-palvus.pion-1.ntrn.tech'

// Astroport configuration
const astroportFactory = 'neutron1jj0scx400pswhpjes589aujlqagxgcztw04srynmhf0f6zplzn2qqmhwj7'
const astroportRouter = 'neutron12jm24l9lr9cupufqjuxpdjnnweana4h66tsx5cl800mke26td26sq7m05p'
const astroportIncentives = 'neutron1slxs8heecwyw0n6zmj7unj3nenrfhk2zpagfz2lt87dnevmksgwsq9adkn'

// note the following three addresses are all 'mars' bech32 prefix
const safetyFundAddr = 'mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575'
const feeCollectorAddr = 'mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x'

// Pyth configuration
const pythAddr = 'neutron15ldst8t80982akgr8w8ekcytejzkmfpgdkeq4xgtge48qs7435jqp87u3t'
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

export const pclLpMarsUsdcOracle: OracleConfig = {
  denom: pclLpMarsUsdcDenom,
  price_source: {
    pcl_liquidity_token: {
      pair_address: pclLpMarsUsdcPairAddr,
    },
  },
}

export const pclLpMarsOracle: OracleConfig = {
  denom: pclLpMarsDenom,
  price_source: {
    fixed: {
      price: '0.84',
    },
  },
}

export const pclLpUsdcOracle: OracleConfig = {
  denom: pclLpUsdcDenom,
  price_source: {
    fixed: {
      price: '1.05',
    },
  },
}

// Router configurations
export const atomUsdcRoute = {
  denom_in: atomDenom,
  denom_out: nobleUsdcDenom,
  route: {
    factory: astroportFactory,
    operations: [
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: nobleUsdcDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: atomDenom,
            },
          },
        },
      },
    ],
    oracle: '', // Will be filled in by deploy script
    router: astroportRouter,
  },
}

export const ntrnUsdcRoute = {
  denom_in: 'untrn',
  denom_out: nobleUsdcDenom,
  route: {
    factory: astroportFactory,
    operations: [
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: nobleUsdcDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: 'untrn',
            },
          },
        },
      },
    ],
    oracle: '', // Will be filled in by deploy script
    router: astroportRouter,
  },
}

export const atomMarsRoute = {
  denom_in: atomDenom,
  denom_out: marsDenom,
  route: {
    factory: astroportFactory,
    operations: [
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: marsDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: atomDenom,
            },
          },
        },
      },
    ],
    oracle: '', // Will be filled in by deploy script
    router: astroportRouter,
  },
}

export const ntrnMarsRoute = {
  denom_in: 'untrn',
  denom_out: marsDenom,
  route: {
    factory: astroportFactory,
    operations: [
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: marsDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: 'untrn',
            },
          },
        },
      },
    ],
    oracle: '', // Will be filled in by deploy script
    router: astroportRouter,
  },
}

export const usdcMarsRoute = {
  denom_in: nobleUsdcDenom,
  denom_out: marsDenom,
  route: {
    factory: astroportFactory,
    operations: [
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: marsDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: nobleUsdcDenom,
            },
          },
        },
      },
    ],
    oracle: '', // Will be filled in by deploy script
    router: astroportRouter,
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

export const pclLpMarsUsdcAsset: AssetConfig = {
  denom: pclLpMarsUsdcDenom,
  max_loan_to_value: '0.35',
  liquidation_threshold: '0.40',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'PCL_LP_MARS_USDC',
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

export const pclLpMarsAsset: AssetConfig = {
  denom: pclLpMarsDenom,
  max_loan_to_value: '0.74',
  liquidation_threshold: '0.75',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'PCL_LP_MARS',
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
    slope_1: '0.125',
    slope_2: '2',
  },
}

export const pclLpUsdcAsset: AssetConfig = {
  denom: pclLpUsdcDenom,
  max_loan_to_value: '0.74',
  liquidation_threshold: '0.75',
  liquidation_bonus: {
    max_lb: '0.2',
    min_lb: '0.05',
    slope: '1',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  symbol: 'PCL_LP_USDC',
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
    slope_1: '0.125',
    slope_2: '2',
  },
}

export const neutronTestnetConfig: DeploymentConfig = {
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
    routes: [atomUsdcRoute, atomMarsRoute, ntrnUsdcRoute, ntrnMarsRoute, usdcMarsRoute],
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
    pclLpMarsUsdcAsset,
    pclLpMarsAsset,
    pclLpUsdcAsset,
  ],
  vaults: [],
  oracleConfigs: [
    usdOracle,
    nobleUSDCOracle,
    atomOracle,
    ntrnOracle,
    pclLpMarsOracle,
    pclLpUsdcOracle,
    pclLpMarsUsdcOracle,
  ],
  astroportConfig: {
    factory: astroportFactory,
    router: astroportRouter,
    incentives: astroportIncentives,
  },
}
