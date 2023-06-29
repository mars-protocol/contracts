import { taskRunner } from '../base'
import { osmosisMainnet } from './testnetConfig'

void (async function () {
  await taskRunner(osmosisMainnet)
})()
