import { taskRunner } from '../base'
import { osmosisTestnetConfig } from './config'

void (async function () {
  await taskRunner({
    config: osmosisTestnetConfig,
    swapperContractName: 'mars_swapper_osmosis',
    zapperContractName: 'mars_zapper_osmosis',
  })
})()
