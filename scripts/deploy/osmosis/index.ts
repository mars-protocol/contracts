import { taskRunner } from '../base'
import { osmosisMultisig, osmosisTestnetConfig } from './config.js'

void (async function () {
  await taskRunner(osmosisTestnetConfig, osmosisMultisig)
})()
