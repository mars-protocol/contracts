import { DeploymentConfig, AssetConfig, OracleConfig } from '../../types/config'
import { NeutronIbcConfig } from '../../types/generated/mars-rewards-collector-base/MarsRewardsCollectorBase.types'

const axlUsdcDenom = 'ibc/F91EA2C0A23697A1048E08C2F787E3A58AC6F706A1CD2257A504925158CFC0F3'
const atomDenom = 'ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9'
const marsDenom = 'ibc/584A4A23736884E0C198FD1EE932455A9357A492A7B94324E4A02B5628687831'

const protocolAdminAddr = 'neutron1ltzuv25ltw9mkwuvvmt7e54a6ene283hfj7l0c'

const marsNeutronChannelId = 'channel-97'
const gasPrice = '0.01untrn'
const chainId = 'pion-1'
const rpcEndpoint = 'https://testnet-neutron-rpc.marsprotocol.io:443'

// Astroport configuration
const astroportFactory = 'neutron1jj0scx400pswhpjes589aujlqagxgcztw04srynmhf0f6zplzn2qqmhwj7'
const astroportRouter = 'neutron12jm24l9lr9cupufqjuxpdjnnweana4h66tsx5cl800mke26td26sq7m05p'
const astroportNtrnAtomPair = 'neutron1sm23jnz4lqd88etklvwlm66a0x6mhflaqlv65wwr7nwwxa6258ks6nshpq'
const astroportMarsUsdcPair = 'neutron1xf0awuvhkrq553p543narplfp5x7ltjwwys6dyzfnmvs75v3yfqsw65lyp'
const astroportAtomUsdcPair = 'neutron18j9jfw9dlw3ep8fkyfftpag70656cv7q60plluaf9tjwyn9wx0pq9t9wtu'

// note the following three addresses are all 'mars' bech32 prefix
const safetyFundAddr = 'mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575'
const feeCollectorAddr = 'mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x'

// Pyth configuration
const pythAddr = 'neutron1f86ct5az9qpz2hqfd5uxru02px2a3tz5zkw7hugd7acqq496dcms22ehpy'
const pythUsdcID = 'eaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a'

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
export const marsOracle: OracleConfig = {
  denom: marsDenom,
  price_source: {
    astroport_twap: {
      window_size: 300, // 5 minutes
      tolerance: 120, // 2 minutes
      pair_address: astroportMarsUsdcPair,
    },
  },
}

export const ntrnOracle: OracleConfig = {
  denom: 'untrn',
  price_source: {
    astroport_twap: {
      window_size: 300, // 5 minutes
      tolerance: 120, // 2 minutes
      pair_address: astroportNtrnAtomPair,
    },
  },
}

export const atomOracle: OracleConfig = {
  denom: atomDenom,
  price_source: {
    astroport_twap: {
      window_size: 300, // 5 minutes
      tolerance: 120, // 2 minutes
      pair_address: astroportAtomUsdcPair,
    },
  },
}

export const axlUSDCOracle: OracleConfig = {
  denom: axlUsdcDenom,
  price_source: {
    pyth: {
      contract_addr: pythAddr,
      price_feed_id: pythUsdcID,
      denom_decimals: 6,
      max_staleness: 300, // 5 minutes
      max_confidence: '0.1',
      max_deviation: '0.1',
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

// Router configurations
export const atomUsdcRoute = {
  denom_in: atomDenom,
  denom_out: axlUsdcDenom,
  route: {
    factory: astroportFactory,
    operations: [
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: axlUsdcDenom,
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
  denom_out: axlUsdcDenom,
  route: {
    factory: astroportFactory,
    operations: [
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: axlUsdcDenom,
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
              denom: axlUsdcDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: atomDenom,
            },
          },
        },
      },
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: marsDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: axlUsdcDenom,
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
              denom: axlUsdcDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: 'untrn',
            },
          },
        },
      },
      {
        astro_swap: {
          ask_asset_info: {
            native_token: {
              denom: marsDenom,
            },
          },
          offer_asset_info: {
            native_token: {
              denom: axlUsdcDenom,
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
  denom_in: axlUsdcDenom,
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
              denom: axlUsdcDenom,
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
  max_loan_to_value: '0.35',
  liquidation_threshold: '0.40',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  // liquidation_bonus: '0.15',
  // reserve_factor: '0.1',
  // interest_rate_model: {
  //   optimal_utilization_rate: '0.6',
  //   base: '0',
  //   slope_1: '0.15',
  //   slope_2: '3',
  // },
  symbol: 'NTRN',
  credit_manager: {
    whitelisted: false,
  },
  red_bank: {
    deposit_cap: '5000000000000',
    borrow_enabled: true,
    deposit_enabled: true,
  },
}

export const atomAsset: AssetConfig = {
  denom: atomDenom,
  max_loan_to_value: '0.68',
  liquidation_threshold: '0.7',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  // liquidation_bonus: '0.1',
  // reserve_factor: '0.1',
  // interest_rate_model: {
  //   optimal_utilization_rate: '0.7',
  //   base: '0',
  //   slope_1: '0.2',
  //   slope_2: '3',
  // },
  symbol: 'ATOM',
  credit_manager: {
    whitelisted: false,
  },
  red_bank: {
    deposit_cap: '150000000000',
    borrow_enabled: true,
    deposit_enabled: true,
  },
}

export const axlUSDCAsset: AssetConfig = {
  denom: axlUsdcDenom,
  max_loan_to_value: '0.74',
  liquidation_threshold: '0.75',
  liquidation_bonus: {
    max_lb: '0.05',
    min_lb: '0',
    slope: '2',
    starting_lb: '0',
  },
  protocol_liquidation_fee: '0.5',
  // liquidation_bonus: '0.1',
  // reserve_factor: '0.1',
  // interest_rate_model: {
  //   optimal_utilization_rate: '0.8',
  //   base: '0',
  //   slope_1: '0.125',
  //   slope_2: '2',
  // },
  symbol: 'axlUSDC',
  credit_manager: {
    whitelisted: false,
  },
  red_bank: {
    deposit_cap: '500000000000',
    borrow_enabled: true,
    deposit_enabled: true,
  },
}

export const neutronTetstnetMultisigConfig: DeploymentConfig = {
  oracleName: 'wasm',
  oracleBaseDenom: 'uusd',
  rewardsCollectorName: 'neutron',
  rewardsCollectorNeutronIbcConfig: neutronIbcConfig,
  atomDenom: atomDenom,
  baseAssetDenom: 'untrn',
  gasPrice: gasPrice,
  chainId: chainId,
  chainPrefix: 'neutron',
  channelId: marsNeutronChannelId,
  marsDenom: marsDenom,
  rewardsCollectorTimeoutSeconds: 600,
  rpcEndpoint: rpcEndpoint,
  safetyFundFeeShare: '0.5',
  deployerMnemonic:
    'bundle bundle orchard jeans office umbrella bird around taxi arrive infant discover elder they joy misery photo crunch gift fancy pledge attend adult eight',
  multisigAddr: protocolAdminAddr,
  slippage_tolerance: '0.01',
  base_asset_symbol: 'NTRN',
  runTests: false,
  mainnet: false,
  feeCollectorDenom: marsDenom,
  safetyFundDenom: axlUsdcDenom,
  swapRoutes: [atomUsdcRoute, atomMarsRoute, ntrnUsdcRoute, ntrnMarsRoute, usdcMarsRoute],
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddr,
  feeCollectorAddr: feeCollectorAddr,
  swapperDexName: 'astroport',
  assets: [ntrnAsset, atomAsset, axlUSDCAsset],
  vaults: [],
  oracleConfigs: [usdOracle, axlUSDCOracle, marsOracle, atomOracle, ntrnOracle],
  oracleCustomInitParams: {
    astroport_factory: astroportFactory,
  },
  incentiveEpochDuration: 604800, // 1 week
  maxWhitelistedIncentiveDenoms: 10,
  targetHealthFactor: '1.2',
}
