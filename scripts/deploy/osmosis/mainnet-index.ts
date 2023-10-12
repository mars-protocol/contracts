import { taskRunner } from '../base'
import { osmosisMainnetConfig } from './mainnet-config'

void (async function () {
  await taskRunner({
    config: {
      ...osmosisMainnetConfig,
      deployerMnemonic: 'TO BE INSERTED AT TIME OF DEPLOYMENT',
      multisigAddr: 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n',
    },
    label: 'multisig-owner',
  })
})()
