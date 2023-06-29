import { taskRunner } from '../base'
import { osmosisMainnet } from './mainnetConfig'

void (async function () {
  await taskRunner(osmosisMainnet)
})()
