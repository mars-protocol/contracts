import { taskRunner } from '../base'
import { osmosisTestnetConfig } from './testnetConfig'

void (async function () {
  await taskRunner(osmosisTestnetConfig)
})()
