import { taskRunner } from '../base'
import { neutronMainnetConfig } from './config_mainnet.js'

void (async function () {
  await taskRunner(neutronMainnetConfig)
})()
