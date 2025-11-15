import { taskRunner } from '../base'
import { osmosisLocalnetConfig } from './localnet-config'

void (async function () {
  await taskRunner({
    config: osmosisLocalnetConfig,
    label: 'deployer-owner',
  })
})()
