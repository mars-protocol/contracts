import { taskRunner } from '../base'
import { osmo, osmosisMainnet } from './NEW_mainnetConfig'

void (async function () {
  await taskRunner({
    config: {
      ...osmosisMainnet,
      mainnet: false,
      deployerMnemonic: 'TO BE INSERTED AT TIME OF DEPLOYMENT',
      chain: {
        baseDenom: osmo,
        defaultGasPrice: 0.1,
        id: 'devnet',
        prefix: 'osmo',
        rpcEndpoint: 'https://rpc.devnet.osmosis.zone',
      },
      runTests: true,
    },
    label: 'devnet',
  })
})()
