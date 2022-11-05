import { taskRunner } from '../base'
import { osmosisTestnetConfig } from './config.js'

void (async function () {
  await taskRunner(osmosisTestnetConfig)
})()
