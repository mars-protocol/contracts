import { DataFetcher } from './DataFetcher'
import { compute_health_js, max_withdraw_estimate_js, max_borrow_estimate_js } from './pkg-node'
import { osmosisTestnetConfig } from '../deploy/osmosis/testnet-config'
import OsmosisAddresses from '../deploy/addresses/osmo-test-5-testnet-deployer-owner.json'
;(async () => {
  const dataFetcher = new DataFetcher(
    compute_health_js,
    max_withdraw_estimate_js,
    max_borrow_estimate_js,
    OsmosisAddresses.creditManager,
    OsmosisAddresses.oracle.addr,
    osmosisTestnetConfig.params.addr,
    osmosisTestnetConfig.chain.rpcEndpoint,
  )
  const health = await dataFetcher.computeHealth('2')
  console.log(health)
  const max_withdraw = await dataFetcher.maxWithdrawAmount('2', 'uosmo')
  console.log(max_withdraw)
  const max_borrow = await dataFetcher.maxBorrowAmount('2', 'uosmo', 'deposit')
  console.log(max_borrow)
})()
