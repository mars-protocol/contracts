import { DeploymentConfig } from '../../types/config'
import { coins } from '@cosmjs/stargate'

export const osmosisTestnetConfig: DeploymentConfig = {
  baseDenom: 'uosmo',
  secondaryDenom: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2', // uatom
  chainId: 'osmo-test-4',
  chainPrefix: 'osmo',
  deployerMnemonic:
    'rely wonder join knock during sudden slow plate segment state agree also arrest mandate grief ordinary lonely lawsuit hurt super banana rule velvet cart',
  rpcEndpoint: 'https://rpc-test.osmosis.zone',
  defaultGasPrice: 0.1,
  startingAmountForTestUser: 1e6,
  mockRedbankCoins: [{ denom: 'uosmo', max_ltv: '0.8', liquidation_threshold: '0.9' }],
  seededFundsForMockRedBank: coins(100, 'uosmo'),
  oraclePrices: [{ denom: 'uosmo', price: '12.1' }],
  maxCloseFactor: 0.6,
  maxLiquidationBonus: 0.05,
  depositAmount: 100,
  borrowAmount: 10,
  repayAmount: 3,
  swapAmount: 12,
  swapRoute: {
    steps: [
      {
        denom_out: 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2',
        pool_id: 1,
      },
    ],
  },
  slippage: 0.4,
  withdrawAmount: 12,
}
