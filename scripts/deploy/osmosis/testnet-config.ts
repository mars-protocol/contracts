import { DeploymentConfig, VaultType } from '../../types/config'

const uosmo = 'uosmo'
const uatom = 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2'
const udig = 'ibc/307E5C96C8F60D1CBEE269A9A86C0834E1DB06F2B3788AE4F716EDB97A48B97D'
const ujuno = 'ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED'
const gammPool1 = 'gamm/pool/1'
const gammPool497 = 'gamm/pool/497'

const vaultOsmoAtom1 = 'osmo1zktjv92f76epswjvyxzzt3yyskpw7k6jsyu0kmq4zzc5fphrjumqlahctp'
const vaultOsmoAtom7 = 'osmo167j3yttwzcm3785tzk4jse2qdkppcy2xxrn5u6srqv7s93wnq6yqw8zhg5'
const vaultOsmoAtom14 = 'osmo1tp2m6g39h8mvhnu3plqjyen5s63023gj8w873l8wvly0cd77l6hsaa73wt'
const atomOsmoConfig = {
  config: {
    deposit_cap: { denom: uatom, amount: '1000000000' }, // 1000 atom
    max_ltv: '0.63',
    liquidation_threshold: '0.65',
    whitelisted: true,
  },
}

const vaultJunoOsmo1 = 'osmo1r6h0pafu3wq0kf6yv09qhc8qvuku2d6fua0rpwwv46h7hd8u586scxspjf'
const vaultJunoOsmo7 = 'osmo1gr5epxn67q6202l3hy0mcnu7qc039v22pa6x2tsk23zwg235n9jsq6pmes'
const vaultJunoOsmo14 = 'osmo1d6knwkelyr9eklewnn9htkess4ttpxpf2cze9ec0xfw7e3fj0ggssqzfpp'
const junoOsmoConfig = {
  config: {
    deposit_cap: { denom: uatom, amount: '500000000' }, // 500 atom
    max_ltv: '0.65',
    liquidation_threshold: '0.66',
    whitelisted: true,
  },
}

export const osmosisTestnetConfig: DeploymentConfig = {
  allowedCoins: [uosmo, uatom, ujuno, gammPool1, gammPool497],
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
      ...atomOsmoConfig,
    },
    {
      vault: { address: vaultOsmoAtom7 },
      ...atomOsmoConfig,
    },
    {
      vault: { address: vaultOsmoAtom14 },
      ...atomOsmoConfig,
    },
    {
      vault: { address: vaultJunoOsmo1 },
      ...junoOsmoConfig,
    },
    {
      vault: { address: vaultJunoOsmo7 },
      ...junoOsmoConfig,
    },
    {
      vault: { address: vaultJunoOsmo14 },
      ...junoOsmoConfig,
    },
  ],
  swapperContractName: 'mars_swapper_osmosis',
  zapperContractName: 'mars_zapper_osmosis',
  testActions: {
    allowedCoinsConfig: [
      { denom: uosmo, priceSource: { fixed: { price: '1' } }, grantCreditLine: true },
      {
        denom: uatom,
        priceSource: { geometric_twap: { pool_id: 1, window_size: 1800 } },
        grantCreditLine: true,
      },
      {
        denom: ujuno,
        priceSource: { geometric_twap: { pool_id: 497, window_size: 1800 } },
        grantCreditLine: true,
      },
      {
        denom: gammPool1,
        priceSource: { xyk_liquidity_token: { pool_id: 1 } },
        grantCreditLine: false,
      },
      {
        denom: gammPool497,
        priceSource: { xyk_liquidity_token: { pool_id: 497 } },
        grantCreditLine: false,
      },
    ],
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
    lendAmount: '10',
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
