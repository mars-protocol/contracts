import { taskRunner } from '../base'
import { neutronTetstnetMultisigConfig } from './config_testnet_multisig'

void (async function () {
  await taskRunner(neutronTetstnetMultisigConfig)
})()
