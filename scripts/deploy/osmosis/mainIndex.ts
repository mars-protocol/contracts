import { taskRunner } from '../base'
import { osmosisMainnet } from './config.js'

void (async function () {
  await taskRunner(osmosisMainnet)
})()
