import { DeploymentConfig, VaultType } from '../../types/config'

const uatom = 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2'
const udig = 'ibc/307E5C96C8F60D1CBEE269A9A86C0834E1DB06F2B3788AE4F716EDB97A48B97D'
const ucro = 'ibc/E6931F78057F7CC5DA0FD6CEF82FF39373A6E0452BF1FD76910B93292CF356C1'

export const osmosisTestnetConfig: DeploymentConfig = {
  // Get the latest addresses from: https://github.com/mars-protocol/outposts/blob/master/scripts/deploy/addresses/osmo-test-4.json
  oracleAddr: 'osmo1hkkx42777dyfz7wc8acjjhfdh9x2ugcjvdt7shtft6ha9cn420cquz3u3j',
  redBankAddr: 'osmo1g30recyv8pfy3qd4qn3dn7plc0rn5z68y5gn32j39e96tjhthzxsw3uvvu',
  baseDenom: 'uosmo',
  secondaryDenom: uatom,
  chainId: 'osmo-test-4',
  chainPrefix: 'osmo',
  deployerMnemonic:
    'rely wonder join knock during sudden slow plate segment state agree also arrest mandate grief ordinary lonely lawsuit hurt super banana rule velvet cart',
  redBankDeployerMnemonic:
    'elevator august inherit simple buddy giggle zone despair marine rich swim danger blur people hundred faint ladder wet toe strong blade utility trial process',
  rpcEndpoint: 'https://rpc-test.osmosis.zone',
  defaultGasPrice: 0.1,
  startingAmountForTestUser: 2e6,
  vaultTokenDenom: udig,
  vaultLockup: { time: 86400 }, // 1 day
  maxCloseFactor: 0.6,
  depositAmount: 100,
  toGrantCreditLines: [
    { denom: 'uosmo', amount: '100000000000' },
    { denom: uatom, amount: '100000000000' },
  ],
  borrowAmount: 10,
  repayAmount: 8,
  swapAmount: 12,
  swapRoute: [
    {
      token_out_denom: uatom,
      pool_id: '1',
    },
  ],
  slippage: 0.4,
  withdrawAmount: 12,
  vaultDepositAmount: 10,
  vaultDepositCap: { denom: 'uosmo', amount: '100000000000' },
  vaultMaxLTV: 0.65,
  vaultLiquidationThreshold: 0.75,
  vaultType: VaultType.LOCKED,
  vaultWithdrawAmount: 1_000_000,
  lpToken: { denom: ucro, price: 3 },
  zap: [
    { denom: uatom, amount: 3, price: 2.135 },
    { denom: 'uosmo', amount: 3, price: 1 },
  ],
  unzapAmount: 1000000,
  maxValueForBurn: 1000000,
  maxUnlockingPositions: 10,
}
