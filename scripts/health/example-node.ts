import { DataFetcher } from './DataFetcher'
import { Storage } from '../deploy/base/storage'
import { compute_health_js } from './pkg-node'
import { osmosisTestnetConfig } from '../deploy/osmosis/testnet-config'
;(async () => {
  const storage = await Storage.load(osmosisTestnetConfig.chain.id, 'testnet-deployer-owner')
  const dataFetcher = new DataFetcher(
    compute_health_js,
    storage.addresses.creditManager!,
    osmosisTestnetConfig.oracle.addr,
    osmosisTestnetConfig.redBank.addr,
    osmosisTestnetConfig.chain.rpcEndpoint,
  )
  const health = await dataFetcher.fetchHealth('9')
  console.log(health)
})()
