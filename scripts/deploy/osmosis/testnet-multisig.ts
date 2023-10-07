import { taskRunner } from '../base'
import { osmosisTestnetConfig } from './testnet-config'

void (async function () {
  await taskRunner({
    config: {
      ...osmosisTestnetConfig,
      testActions: undefined,
      oracle: { addr: 'osmo1lcdrm4wpycdlruxv34t6rmvmy9fnehynrkwme00vfyqnzcg0qqxqwdlyzg' },
      redBank: { addr: 'osmo1t5w9qqp0drassayyv23m6sh70kw754xxd78t8tmscljysnnv0avqk87a6f' },
      multisigAddr: 'osmo14w4x949nwcrqgfe53pxs3k7x53p0gvlrq34l5n',
    },
    label: 'testnet-multisig',
  })
})()
