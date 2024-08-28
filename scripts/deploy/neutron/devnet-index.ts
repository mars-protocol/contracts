import { taskRunner } from '../base/index.js'
import { neutronDevnetConfig } from './devnet-config.js'

void (async function () {
  await taskRunner({
    config: neutronDevnetConfig,
    label: 'deployer-owner',
  })
})()
