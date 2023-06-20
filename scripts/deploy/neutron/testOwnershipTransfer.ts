import { transferOwnership } from '../base/transferOwnership'
import { neutronTestnetConfig } from './config.js'

void (async function () {
  await transferOwnership(neutronTestnetConfig)
})()
