import { DeploymentConfig, VaultType } from '../../types/config'

// Note: since osmo-test-5 upgrade, testnet and mainnet denoms are no longer the same. Reference asset info here: https://docs.osmosis.zone/osmosis-core/asset-info/
const uosmo = 'uosmo'
const nUSDC = 'ibc/B3504E092456BA618CC28AC671A71FB08C6CA0FD0BE7C8A5B5A3E2DD933CC9E4' // noble
const nUSDC_osmo = 'gamm/pool/6'

const nUSDC_OSMO_vault_1 = 'osmo1q40xvrzpldwq5he4ftsf7zm2jf80tj373qaven38yqrvhex8r9rs8n94kv'
const nUSDC_OSMO_vault_7 = 'osmo14lu7m4ganxs20258dazafrjfaulmfxruq9n0r0th90gs46jk3tuqwfkqwn'
const nUSDC_OSMO_vault_14 = 'osmo1fmq9hw224fgz8lk48wyd0gfg028kvvzggt6c3zvnaqkw23x68cws5nd5em'

const nUSDC_OSMO_Config = (addr: string) => ({
  addr,
  deposit_cap: { denom: nUSDC, amount: '1000000000' }, // 1000 atom
  max_loan_to_value: '0.63',
  liquidation_threshold: '0.65',
  whitelisted: true,
})

export const osmosisTestnetConfig: DeploymentConfig = {
  allowedCoins: [uosmo, nUSDC, nUSDC, nUSDC_osmo],
  chain: {
    baseDenom: uosmo,
    defaultGasPrice: 0.1,
    id: 'osmo-test-5',
    prefix: 'osmo',
    rpcEndpoint: 'https://rpc.osmotest5.osmosis.zone',
  },
  deployerMnemonic:
    'rely wonder join knock during sudden slow plate segment state agree also arrest mandate grief ordinary lonely lawsuit hurt super banana rule velvet cart',
  maxUnlockingPositions: '10',
  maxValueForBurn: '1000000',
  // Latest from: https://github.com/mars-protocol/outposts/blob/master/scripts/deploy/addresses/osmo-test-5.json
  oracle: { addr: 'osmo1khe29uw3t85nmmp3mtr8dls7v2qwsfk3tndu5h4w5g2r5tzlz5qqarq2e2' },
  redBank: { addr: 'osmo1dl4rylasnd7mtfzlkdqn2gr0ss4gvyykpvr6d7t5ylzf6z535n9s5jjt8u' },
  params: { addr: 'osmo1xvg28lrr72662t9u0hntt76lyax9zvptdvdmff4k2q9dhjm8x6ws9zym4v' },
  swapRoutes: [
    { denomIn: uosmo, denomOut: nUSDC, route: [{ token_out_denom: nUSDC, pool_id: '6' }] },
    { denomIn: nUSDC, denomOut: uosmo, route: [{ token_out_denom: uosmo, pool_id: '6' }] },
  ],
  // Latest from: https://api.apollo.farm/api/graph?query=query+MyQuery+%7B%0A++vaults%28network%3A+osmo_test_5%29+%7B%0A++++label%0A++++contract_address%0A++%7D%0A%7D
  vaults: [
    nUSDC_OSMO_Config(nUSDC_OSMO_vault_1),
    nUSDC_OSMO_Config(nUSDC_OSMO_vault_7),
    nUSDC_OSMO_Config(nUSDC_OSMO_vault_14),
  ],
  swapperContractName: 'mars_swapper_osmosis',
  zapperContractName: 'mars_v2_zapper_osmosis',
  testActions: {
    allowedCoinsConfig: [
      { denom: uosmo, priceSource: { fixed: { price: '1' } }, grantCreditLine: true },
      {
        denom: nUSDC,
        priceSource: { geometric_twap: { pool_id: 5, window_size: 1800 } },
        grantCreditLine: true,
      },
      {
        denom: nUSDC_osmo,
        priceSource: { xyk_liquidity_token: { pool_id: 6 } },
        grantCreditLine: false,
      },
    ],
    vault: {
      depositAmount: '1000000',
      withdrawAmount: '1000000',
      mock: {
        config: {
          deposit_cap: { denom: nUSDC, amount: '100000000' }, // 100 usdc
          liquidation_threshold: '0.585',
          max_loan_to_value: '0.569',
          whitelisted: true,
        },
        vaultTokenDenom: uosmo,
        type: VaultType.LOCKED,
        lockup: { time: 900 }, // 15 mins
        baseToken: nUSDC_osmo,
      },
    },
    outpostsDeployerMnemonic:
      'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process',
    borrowAmount: '10',
    repayAmount: '11',
    defaultCreditLine: '100000000000',
    depositAmount: '100',
    lendAmount: '10',
    reclaimAmount: '5',
    secondaryDenom: nUSDC,
    startingAmountForTestUser: '4000000',
    swap: {
      slippage: '0.4',
      amount: '40',
      route: [
        {
          token_out_denom: nUSDC,
          pool_id: '1',
        },
      ],
    },
    unzapAmount: '1000000',
    withdrawAmount: '12',
    zap: {
      coinsIn: [
        {
          denom: nUSDC,
          amount: '1',
        },
        { denom: uosmo, amount: '3' },
      ],
      denomOut: nUSDC_osmo,
    },
  },
}
