import { DeploymentConfig, VaultType } from '../../types/config'

const uosmo = 'uosmo'
const uatom = 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2'
const udig = 'ibc/307E5C96C8F60D1CBEE269A9A86C0834E1DB06F2B3788AE4F716EDB97A48B97D'
const ujuno = 'ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED'
const gammPool1 = 'gamm/pool/1'
const gammPool497 = 'gamm/pool/497'

const vaultOsmoAtom1 = 'osmo1v40lnedgvake8p7f49gvqu0q3vc9sx3qpc0jqtyfdyw25d4vg8us38an37'
const vaultOsmoAtom7 = 'osmo108q2krqr0y9g0rtesenvsw68sap2xefelwwjs0wedyvdl0cmrntqvllfjk'
const vaultOsmoAtom14 = 'osmo1eht92w5dr0vx8dzl6dn9770yq0ycln50zfhzvz8uc6928mp8vvgqwcram9'
const vaultJunoOsmo1 = 'osmo1g5hryv0gp9dzlchkp3yxk8fmcf5asjun6cxkvyffetqzkwmvy75qfmeq3f'
const vaultJunoOsmo7 = 'osmo1jtuvr47taunfdhwrkns0cufwa3qlsz66qwwa9vvn4cc5eltzrtxs4zkaus'
const vaultJunoOsmo14 = 'osmo1rclt7lsfp0c89ydf9umuhwlg28maw6z87jak3ly7u2lefnyzdz2s8gsepe'

export const osmosisTestnetConfig: DeploymentConfig = {
  allowedCoins: [
    { denom: uosmo, priceSource: { fixed: { price: '1' } } },
    { denom: uatom, priceSource: { arithmetic_twap: { pool_id: 1, window_size: 1800 } } },
    { denom: ujuno, priceSource: { arithmetic_twap: { pool_id: 497, window_size: 1800 } } },
    { denom: gammPool1, priceSource: { xyk_liquidity_token: { pool_id: 1 } } },
    { denom: gammPool497, priceSource: { xyk_liquidity_token: { pool_id: 497 } } },
  ],
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
  // Latest from: https://github.com/mars-protocol/outposts/blob/master/scripts/deploy/addresses/osmo-test-4.json
  oracle: { addr: 'osmo1dqz2u3c8rs5e7w5fnchsr2mpzzsxew69wtdy0aq4jsd76w7upmsstqe0s8' },
  redBank: { addr: 'osmo1t0dl6r27phqetfu0geaxrng0u9zn8qgrdwztapt5xr32adtwptaq6vwg36' },
  swapRoutes: [
    { denomIn: uosmo, denomOut: uatom, route: [{ token_out_denom: uatom, pool_id: '1' }] },
    { denomIn: uatom, denomOut: uosmo, route: [{ token_out_denom: uosmo, pool_id: '1' }] },
    { denomIn: uosmo, denomOut: ujuno, route: [{ token_out_denom: ujuno, pool_id: '497' }] },
    { denomIn: ujuno, denomOut: uosmo, route: [{ token_out_denom: uosmo, pool_id: '497' }] },
  ],
  // Latest from: https://stats.apollo.farm/api/vaults/v1/all
  vaults: [
    {
      vault: { address: vaultOsmoAtom1 },
      config: {
        deposit_cap: { denom: 'uosmo', amount: '1000000000' }, // 1000 osmo
        liquidation_threshold: '0.535',
        max_ltv: '0.506',
        whitelisted: true,
      },
    },
    {
      vault: { address: vaultOsmoAtom7 },
      config: {
        deposit_cap: { denom: 'uosmo', amount: '1000000000' }, // 1000 osmo
        liquidation_threshold: '0.535',
        max_ltv: '0.506',
        whitelisted: true,
      },
    },
    {
      vault: { address: vaultOsmoAtom14 },
      config: {
        deposit_cap: { denom: 'uosmo', amount: '1000000000' }, // 1000 osmo
        liquidation_threshold: '0.535',
        max_ltv: '0.506',
        whitelisted: true,
      },
    },
    {
      vault: { address: vaultJunoOsmo1 },
      config: {
        deposit_cap: { denom: 'uosmo', amount: '500000000' }, // 500 osmo
        liquidation_threshold: '0.441',
        max_ltv: '0.4115',
        whitelisted: true,
      },
    },
    {
      vault: { address: vaultJunoOsmo7 },
      config: {
        deposit_cap: { denom: 'uosmo', amount: '500000000' }, // 500 osmo
        liquidation_threshold: '0.441',
        max_ltv: '0.4115',
        whitelisted: true,
      },
    },
    {
      vault: { address: vaultJunoOsmo14 },
      config: {
        deposit_cap: { denom: 'uosmo', amount: '500000000' }, // 500 osmo
        liquidation_threshold: '0.441',
        max_ltv: '0.4115',
        whitelisted: true,
      },
    },
  ],
  testActions: {
    vault: {
      depositAmount: '1000000',
      withdrawAmount: '1000000',
      mock: {
        config: {
          deposit_cap: { denom: uosmo, amount: '100000000' }, // 100 osmo
          liquidation_threshold: '0.585',
          max_ltv: '0.569',
          whitelisted: true,
        },
        vaultTokenDenom: udig,
        type: VaultType.LOCKED,
        lockup: { time: 900 }, // 15 mins
        baseToken: gammPool1,
      },
    },
    outpostsDeployerMnemonic:
      'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process',
    borrowAmount: '10',
    repayAmount: '11',
    defaultCreditLine: '100000000000',
    depositAmount: '100',
    secondaryDenom: uatom,
    startingAmountForTestUser: '2500000',
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
        {
          denom: uatom,
          amount: '1',
        },
        { denom: uosmo, amount: '3' },
      ],
      denomOut: gammPool1,
    },
  },
}
