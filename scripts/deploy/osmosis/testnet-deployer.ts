import { taskRunner } from '../base/index.rover'
import { osmosisTestnetConfig } from './testnet-config'

void (async function () {
  await taskRunner({
    config: osmosisTestnetConfig,
    label: 'testnet-deployer-owner',
  })
})()
