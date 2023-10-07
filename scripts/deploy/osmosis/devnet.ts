import { taskRunner } from '../base'
import { DeploymentConfig } from '../../types/config'

const osmo = 'uosmo'
const atom = 'ibc/27394FB092D2ECCD56123C74F36E4C1F926001CEADA9CA97EA622B25F41E5EB2'
// const axl = 'ibc/903A61A498756EA560B85A85132D3AEE21B5DEDD41213725D22ABF276EA6945E'
// const stAtom = 'ibc/C140AFD542AE77BD7DCC83F13FDD8C5E5BB8C4929785E6EC2F4C636F98F17901'
const wbtc = 'ibc/D1542AA8762DB13087D8364F3EA6509FD6F009A34F00426AF9E4F9FA85CBBF1F'
const axlUSDC = 'ibc/D189335C6E4A68B513C10AB227BF1C1D38C746766278BA3EEB4FB14124F1D858'
const eth = 'ibc/EA1D43981D5C9A1C4AAEA9C23BB1D4FA126BA9BC7020A25E0AE4AA841EA25DC5'

const defaultCreditLine = '100000000000000'

export const osmosisDevnetConfig: DeploymentConfig = {
  // multisigAddr: 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n',
  creditLineCoins: [
    // AXL and stAtom has borrowing disabled
    { denom: osmo, creditLine: defaultCreditLine },
    { denom: atom, creditLine: defaultCreditLine },
    { denom: wbtc, creditLine: defaultCreditLine },
    { denom: axlUSDC, creditLine: defaultCreditLine },
    { denom: eth, creditLine: '1000000000000000000000' },
  ],
  chain: {
    baseDenom: osmo,
    defaultGasPrice: 0.1,
    id: 'devnet',
    prefix: 'osmo',
    rpcEndpoint: 'https://rpc.devnet.osmosis.zone',
  },
  deployerMnemonic: 'TODO',
  maxUnlockingPositions: '1',
  maxSlippage: '0.2',
  maxValueForBurn: '10000',
  // oracle and redbank contract addresses can be found:  https://github.com/mars-protocol/red-bank/blob/master/README.md#osmosis-1
  addressProvider: { addr: 'osmo1x7udlkawmkz2u5th5x3cjxht2yvjgph7pg8l9rumaa3lak922dgsr3lmhc' },
  oracle: { addr: 'osmo1dh8f3rhg4eruc9w7c9d5e06eupqqrth7v32ladwkyphvnn66muzqxcfe60' },
  redBank: { addr: 'osmo1pvrlpmdv3ee6lgmxd37n29gtdahy4tec7c5nyer9aphvfr526z6sff9zdg' },
  incentives: { addr: 'osmo1aemnaq5x3jkttnd38g7lewh24nh90r9zwh8853qv3tkf47p2hnasaae0e4' },
  params: { addr: 'osmo1dpwu03xc45vpqur6ry69xjhltq4v0snrhaukcp4fvhucx0wypzhs978lnp' },
  swapper: { addr: 'osmo17c4retwuyxjxzv9f2q9r0272s8smktpzhjetssttxxdavarjtujsjqafa2' },
  rewardsCollector: { addr: 'osmo19fppgzdenrxwdg2k3te0a48mfee4npdrctghzrcqltwck7e4y6ts7t8428' },
  runTests: false,
  vaults: [],
  zapperContractName: 'mars_zapper_osmosis',
}

void (async function () {
  await taskRunner({
    config: osmosisDevnetConfig,
    label: 'devnet',
  })
})()
