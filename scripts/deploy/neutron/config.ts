import { AssetConfig, OracleConfig } from '../../types/config'

const axlUSDCTestnet = 'ibc/EFB00E728F98F0C4BBE8CA362123ACAB466EDA2826DC6837E49F4C1902F21BBA' // TODO: This is actually ASTRO since there is no pool for axlUSDC on testnet
const atomTestnet = 'ibc/C4CFF46FD6DE35CA4CF4CE031E643C8FDC9BA4B99AE598E9B0ED98FE3A2319F9'
const marsTestnet = 'ibc/EFB00E728F98F0C4BBE8CA362123ACAB466EDA2826DC6837E49F4C1902F21BBA' // TODO: Use real MARS denom when it is available
const protocolAdminAddrTestnet = 'neutron1ke0vqqzyymlp5esr8gjwuzh94ysnpvj8er5hm7'
const astroportFactoryTestnet = 'neutron1jj0scx400pswhpjes589aujlqagxgcztw04srynmhf0f6zplzn2qqmhwj7'
const astroportRouterTestnet = 'neutron12jm24l9lr9cupufqjuxpdjnnweana4h66tsx5cl800mke26td26sq7m05p'

// note the following addresses are all 'mars' bech32 prefix
const safetyFundAddr = 'mars1s4hgh56can3e33e0zqpnjxh0t5wdf7u3pze575'
const feeCollectorAddr = 'mars17xpfvakm2amg962yls6f84z3kell8c5ldy6e7x'

export const ntrnAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'NTRM',
  denom: 'untrn',
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
    deposit_cap: '2500000000000',
    deposit_enabled: true,
  },
}

export const atomAsset: AssetConfig = {
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'ATOM',
  denom: atomTestnet,
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
    deposit_cap: '100000000000',
    deposit_enabled: true,
  },
}

export const axlUSDCAsset: AssetConfig = {
  denom: axlUSDCTestnet,
  credit_manager: {
    whitelisted: true,
  },
  symbol: 'axlUSDC',
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
    deposit_cap: '500000000000',
    deposit_enabled: true,
  },
}

export const ntrnOracleTestnet: OracleConfig = {
  denom: 'untrn',
  price_source: {
    astroport_twap: {
      window_size: 1800, // 30 minutes
      tolerance: 120, // 2 minutes
      pair_address: 'neutron1vwrktvvxnevy7s5t7v44z72pdxncnq9gdsjwq9607cdd6vl2lfcs33fpah',
      route_assets: [axlUSDCTestnet],
    },
  },
}

export const atomOracleTestnet: OracleConfig = {
  denom: atomTestnet,
  price_source: {
    astroport_twap: {
      window_size: 1800, // 30 minutes
      tolerance: 120, // 2 minutes
      pair_address: 'neutron1sm23jnz4lqd88etklvwlm66a0x6mhflaqlv65wwr7nwwxa6258ks6nshpq',
      route_assets: ['untrn'],
    },
  },
}

export const axlUSDCOracleTestnet: OracleConfig = {
  denom: axlUSDCTestnet,
  price_source: {
    fixed: {
      price: '1.0',
    },
  },
}

export const neutronTestnetConfig = {
  chainName: 'wasm',
  atomDenom: atomTestnet,
  baseAssetDenom: 'untrn',
  gasPrice: '0untrn',
  chainId: 'pion-1',
  chainPrefix: 'neutron',
  channelId: '', //TODO
  marsDenom: marsTestnet,
  rewardCollectorTimeoutSeconds: 600,
  rpcEndpoint: 'https://rpc-palvus.pion-1.ntrn.tech:443',
  safetyFundFeeShare: '0.5',
  deployerMnemonic: '', // TODO: Set mnemonic before deploying
  slippage_tolerance: '0.01',
  base_asset_symbol: 'NTRN',
  second_asset_symbol: 'ATOM',
  runTests: true,
  mainnet: false,
  feeCollectorDenom: marsTestnet,
  safetyFundDenom: axlUSDCTestnet,
  swapRoutes: [
    {
      denom_in: atomTestnet,
      denom_out: 'untrn',
      route: {
        factory: astroportFactoryTestnet,
        operations: [
          {
            astro_swap: {
              ask_asset_info: {
                native_token: {
                  denom: 'untrn',
                },
              },
              offer_asset_info: {
                native_token: {
                  denom: atomTestnet,
                },
              },
            },
          },
        ],
        oracle: '', // Will be filled in by deploy script
        router: astroportRouterTestnet,
      },
    },
    {
      denom_in: atomTestnet,
      denom_out: axlUSDCTestnet,
      route: {
        factory: astroportFactoryTestnet,
        operations: [
          {
            astro_swap: {
              ask_asset_info: {
                native_token: {
                  denom: axlUSDCTestnet,
                },
              },
              offer_asset_info: {
                native_token: {
                  denom: atomTestnet,
                },
              },
            },
          },
        ],
        oracle: '', // Will be filled in by deploy script
        router: astroportRouterTestnet,
      },
    },
    {
      denom_in: 'untrn',
      denom_out: axlUSDCTestnet,
      route: {
        factory: astroportFactoryTestnet,
        operations: [
          {
            astro_swap: {
              ask_asset_info: {
                native_token: {
                  denom: axlUSDCTestnet,
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
        router: astroportRouterTestnet,
      },
    },
  ],
  safetyFundAddr: safetyFundAddr,
  protocolAdminAddr: protocolAdminAddrTestnet,
  feeCollectorAddr: feeCollectorAddr,
  swapperDexName: 'astroport',
  assets: [ntrnAsset, atomAsset],
  vaults: [],
  oracleConfigs: [axlUSDCOracleTestnet, ntrnOracleTestnet, atomOracleTestnet],
  targetHealthFactor: '1.2',
  oracleCustomInitParams: {
    astroport_factory: 'neutron1jj0scx400pswhpjes589aujlqagxgcztw04srynmhf0f6zplzn2qqmhwj7',
  },
  incentiveEpochDuration: 604800, // 1 week
  maxWhitelistedIncentiveDenoms: 10,
}
