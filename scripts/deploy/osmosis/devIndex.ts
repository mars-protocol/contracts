import { taskRunner } from '../base'
import { osmosisDevnet } from './devnetConfig'

void (async function () {
  await taskRunner(osmosisDevnet)
})()
