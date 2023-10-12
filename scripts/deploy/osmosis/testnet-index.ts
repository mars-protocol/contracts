import { taskRunner } from '../base'
import { osmosisTestnetConfig } from './testnet-config'

void (async function () {
  await taskRunner(osmosisTestnetConfig)
})()
