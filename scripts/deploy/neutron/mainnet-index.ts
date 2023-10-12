import { taskRunner } from '../base'
import { neutronMainnetConfig } from './mainnet-config'

void (async function () {
  await taskRunner({
    config: neutronMainnetConfig,
    label: 'multisig-owner',
  })
})()
