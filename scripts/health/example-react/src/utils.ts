import { Positions } from '../../../types/generated/mars-credit-manager/MarsCreditManager.types'

import init, {
  compute_health_js,
  max_withdraw_estimate_js,
  max_borrow_estimate_js,
} from '../../pkg-web'
import { HealthResponse } from '../../../types/generated/mars-rover-health-types/MarsRoverHealthTypes.types'
import { DataFetcher } from '../../DataFetcher'
import { osmosisTestnetConfig } from '../../../deploy/osmosis/testnet-config'

const getFetcher = (cmAddress: string) => {
  return new DataFetcher(
    compute_health_js,
    max_withdraw_estimate_js,
    max_borrow_estimate_js,
    cmAddress,
    osmosisTestnetConfig.oracle.addr,
    osmosisTestnetConfig.redBank.addr,
    osmosisTestnetConfig.chain.rpcEndpoint,
  )
}

export const fetchPositions = async (cmAddress: string, accountId: string): Promise<Positions> => {
  const dataFetcher = getFetcher(cmAddress)
  return await dataFetcher.fetchPositions(accountId)
}

export const fetchHealth = async (
  cmAddress: string,
  accountId: string,
): Promise<HealthResponse> => {
  await init()
  const dataFetcher = getFetcher(cmAddress)
  return await dataFetcher.computeHealth(accountId)
}
