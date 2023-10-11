import { taskRunner } from '../base'
import { osmosisMainnet } from './NEW_mainnetConfig'

void (async function () {
  await taskRunner({
    config: {
      ...osmosisMainnet,
      deployerMnemonic: 'TO BE INSERTED AT TIME OF DEPLOYMENT',
      multisigAddr: 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n',
    },
    label: 'mainnet',
  })
})()
