import { taskRunner } from '../base'
import { osmosisTestnetConfig } from './config'

void (async function () {
  await taskRunner({ config: osmosisTestnetConfig, swapperContractName: 'swapper_osmosis' })
})()
