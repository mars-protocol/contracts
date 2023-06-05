import { DataFetcher } from './DataFetcher'
import { compute_health_js } from './pkg-node'
import { osmosisTestnetConfig } from '../deploy/osmosis/testnet-config'
import OsmosisAddresses from '../deploy/addresses/osmo-test-4.json'
;(async () => {
  const dataFetcher = new DataFetcher(
    compute_health_js,
    OsmosisAddresses.creditManager,
    osmosisTestnetConfig.oracle.addr,
    osmosisTestnetConfig.params.addr,
    osmosisTestnetConfig.chain.rpcEndpoint,
  )
  const health = await dataFetcher.fetchHealth('9')
  console.log(health)
})()
