import { DeploymentConfig, VaultType } from '../../types/config'

const uatom = 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2'
const udig = 'ibc/307E5C96C8F60D1CBEE269A9A86C0834E1DB06F2B3788AE4F716EDB97A48B97D'

export const osmosisTestnetConfig: DeploymentConfig = {
  // Get the latest addresses from: https://github.com/mars-protocol/outposts/blob/master/scripts/deploy/addresses/osmo-test-4.json
  oracleAddr: 'osmo1y3y3ek83hyc4y2te8kytymg599q9sycv9dsufysapra5gglpr4ys25nh94',
  redBankAddr: 'osmo1w5rqrdhut890jplmsqnr8gj3uf0wq6lj5rfdnhrtl63lpf6e7v6qalrhhn',
  baseDenom: 'uosmo',
  secondaryDenom: uatom,
  chainId: 'osmo-test-4',
  chainPrefix: 'osmo',
  deployerMnemonic:
    'rely wonder join knock during sudden slow plate segment state agree also arrest mandate grief ordinary lonely lawsuit hurt super banana rule velvet cart',
  rpcEndpoint: 'https://rpc-test.osmosis.zone',
  defaultGasPrice: 0.1,
  startingAmountForTestUser: 1e6,
  vaultTokenDenom: udig,
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
  vaultDepositAmount: 10,
  vaultType: VaultType.UNLOCKED,
  vaultWithdrawAmount: 1_000_000,
}
