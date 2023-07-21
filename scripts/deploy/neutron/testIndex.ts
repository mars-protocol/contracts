import { taskRunner } from '../base'
import { neutronTestnetConfig } from './config_testnet.js'

void (async function () {
  await taskRunner(neutronTestnetConfig)
})()
