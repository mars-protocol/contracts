import { Positions } from '../../../types/generated/mars-credit-manager/MarsCreditManager.types'

import init, {
  compute_health_js,
  max_withdraw_estimate_js,
  max_borrow_estimate_js,
} from '../../pkg-web'
import { HealthValuesResponse } from '../../../types/generated/mars-rover-health/MarsRoverHealth.types'
import { DataFetcher } from '../../DataFetcher'
import { oracle, params } from '../../../deploy/addresses/devnet-deployer-owner.json'

const getFetcher = (cmAddress: string) => {
  return new DataFetcher(
    compute_health_js,
    max_withdraw_estimate_js,
    max_borrow_estimate_js,
    cmAddress,
    oracle,
    params,
    'https://rpc.devnet.osmosis.zone',
  )
}

export const fetchPositions = async (cmAddress: string, accountId: string): Promise<Positions> => {
  const dataFetcher = getFetcher(cmAddress)
  return await dataFetcher.fetchPositions(accountId)
}

export const fetchHealth = async (
  cmAddress: string,
  accountId: string,
): Promise<HealthValuesResponse> => {
  await init()
  const dataFetcher = getFetcher(cmAddress)
  return await dataFetcher.computeHealth(accountId)
}
