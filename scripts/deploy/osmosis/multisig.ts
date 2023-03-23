import { taskRunner } from '../base'
import { osmosisTestMultisig } from './config.js'

void (async function () {
  await taskRunner(osmosisTestMultisig)
})()
