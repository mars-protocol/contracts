import { taskRunner } from '../base'
import { neutronTestnetConfig } from './config.js'

void (async function () {
  await taskRunner(neutronTestnetConfig)
})()
