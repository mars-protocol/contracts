import { taskRunner } from '../base'
import { neutronTestnetConfig } from './testnet-config'

void (async function () {
  await taskRunner({
    config: {
      ...neutronTestnetConfig,
      multisigAddr: 'neutron1ltzuv25ltw9mkwuvvmt7e54a6ene283hfj7l0c',
    },
    label: 'multisig-owner',
  })
})()
