import { DeploymentConfig } from '../../types/config'

const uatom = 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2'

export const osmosisTestnetConfig: DeploymentConfig = {
  // Get the latest addresses from: https://github.com/mars-protocol/outposts/blob/master/scripts/deploy/addresses/osmo-test-4.json
  oracleAddr: 'osmo1kgv8rr9eglkv52hwf0v96cs5s7ztw06tx3a6zrrcrwgmuuru36cqgmz2xa',
  redBankAddr: 'osmo1dkn4vr75uep4gmd0gatuu7zlapahps7kdapy8wwztcygdu5wy8lqtw2yuj',
  baseDenom: 'uosmo',
  secondaryDenom: uatom,
  chainId: 'osmo-test-4',
  chainPrefix: 'osmo',
  deployerMnemonic:
    'rely wonder join knock during sudden slow plate segment state agree also arrest mandate grief ordinary lonely lawsuit hurt super banana rule velvet cart',
  rpcEndpoint: 'https://rpc-test.osmosis.zone',
  defaultGasPrice: 0.1,
  startingAmountForTestUser: 1e6,
  vaultTokenDenom: 'xCompounder',
  maxCloseFactor: 0.6,
  maxLiquidationBonus: 0.05,
  depositAmount: 100,
  borrowAmount: 10,
  repayAmount: 3,
  swapAmount: 12,
  swapRoute: {
    steps: [
      {
        denom_out: uatom,
        pool_id: 1,
      },
    ],
  },
  slippage: 0.4,
  withdrawAmount: 12,
}
