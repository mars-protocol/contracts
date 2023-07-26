import { taskRunner } from '../base'
import { osmosisTestMultisig } from './testnetConfig'

void (async function () {
  await taskRunner(osmosisTestMultisig)
})()
