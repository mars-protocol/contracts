import { DeploymentConfig, VaultType } from '../../types/config'

const uosmo = 'uosmo'
const uatom = 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2'
const udig = 'ibc/307E5C96C8F60D1CBEE269A9A86C0834E1DB06F2B3788AE4F716EDB97A48B97D'
const gammPool1 = 'gamm/pool/1'

const autoCompoundingVault = 'osmo1v40lnedgvake8p7f49gvqu0q3vc9sx3qpc0jqtyfdyw25d4vg8us38an37'

export const osmosisTestnetConfig: DeploymentConfig = {
  allowedCoins: [uosmo, uatom, gammPool1],
  chain: {
    baseDenom: uosmo,
    defaultGasPrice: 0.1,
    id: 'osmo-test-4',
    prefix: 'osmo',
    rpcEndpoint: 'https://rpc-test.osmosis.zone',
  },
  deployerMnemonic:
    'rely wonder join knock during sudden slow plate segment state agree also arrest mandate grief ordinary lonely lawsuit hurt super banana rule velvet cart',
  maxCloseFactor: '0.6',
  maxUnlockingPositions: '10',
  maxValueForBurn: '1000000',
  // Get the latest addresses from: https://github.com/mars-protocol/outposts/blob/master/scripts/deploy/addresses/osmo-test-4.json
  oracle: {
    addr: 'osmo1jnkun9gcajn96a4yh7atzkq98c9sm0xrsqk7xtes07ujyn7xh5rqjymxxv',
    vaultPricing: [
      {
        addr: autoCompoundingVault,
        base_denom: gammPool1,
        method: 'preview_redeem',
        vault_coin_denom:
          'factory/osmo1v40lnedgvake8p7f49gvqu0q3vc9sx3qpc0jqtyfdyw25d4vg8us38an37/cwVTT',
      },
    ],
  },
  redBank: { addr: 'osmo18w58j2dlpre6kslls9w88aur5ud8000wvg8pw4fp80p6q97g6qtqvhztpv' },
  swapRoutes: [
    { denomIn: uosmo, denomOut: uatom, route: [{ token_out_denom: uatom, pool_id: '1' }] },
    { denomIn: uatom, denomOut: uosmo, route: [{ token_out_denom: uosmo, pool_id: '1' }] },
  ],
  zapper: { addr: 'osmo150dpk65f6deunksn94xtvu249hnr2hwqe335ukucltlwh3uz87hq898s7q' },
  vaults: [
    {
      // https://github.com/apollodao/apollo-config/blob/master/config.json#L114
      vault: { address: autoCompoundingVault },
      config: {
        deposit_cap: { denom: uosmo, amount: '100000000000000000000000000' }, // 100 osmo
        liquidation_threshold: '0.75',
        max_ltv: '0.65',
        whitelisted: true,
      },
    },
  ],
  testActions: {
    vault: {
      depositAmount: '1000000',
      withdrawAmount: '1',
      mock: {
        config: {
          deposit_cap: { denom: uosmo, amount: '100000000000000000000000000' }, // 100 osmo
          liquidation_threshold: '0.75',
          max_ltv: '0.65',
          whitelisted: true,
        },
        vaultTokenDenom: udig,
        type: VaultType.LOCKED,
        lockup: { time: 900 }, // 15 mins
        baseToken: { denom: gammPool1, price: '1.75' },
      },
    },
    outpostsDeployerMnemonic:
      'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process',
    borrowAmount: '10',
    repayAmount: '11',
    defaultCreditLine: '100000000000',
    depositAmount: '100',
    secondaryDenom: uatom,
    startingAmountForTestUser: '2000000',
    swap: {
      slippage: '0.4',
      amount: '40',
      route: [
        {
          token_out_denom: uatom,
          pool_id: '1',
        },
      ],
    },
    unzapAmount: '1000000',
    withdrawAmount: '12',
    zap: {
      coinsIn: [
        { denom: uatom, amount: '1', price: '2.135' },
        { denom: uosmo, amount: '3', price: '1' },
      ],
      denomOut: { denom: gammPool1, price: '1.75' },
    },
  },
}
