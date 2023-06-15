import { InstantiateMsg as ParamsInstantiateMsg } from './generated/mars-params/MarsParams.types'
import { InstantiateMsg as AstroportSwapperInstantiateMsg } from './generated/mars-swapper-astroport/MarsSwapperAstroport.types'
import { InstantiateMsg as RedBankInstantiateMsg } from './generated/mars-red-bank/MarsRedBank.types'
import { InstantiateMsg as AddressProviderInstantiateMsg } from './generated/mars-address-provider/MarsAddressProvider.types'
import { InstantiateMsg as IncentivesInstantiateMsg } from './generated/mars-incentives/MarsIncentives.types'
import { InstantiateMsg as RewardsInstantiateMsg } from './generated/mars-rewards-collector/MarsRewardsCollector.types'
import { InstantiateMsg as WasmOracleInstantiateMsg } from './generated/mars-oracle-wasm/MarsOracleWasm.types'
import { InstantiateMsg as OsmosisSwapperInstantiateMsg } from './generated/mars-swapper-osmosis/MarsSwapperOsmosis.types'
import { InstantiateMsg as OsmosisOracleInstantiateMsg } from './generated/mars-oracle-osmosis/MarsOracleOsmosis.types'

export type InstantiateMsgs =
  | RedBankInstantiateMsg
  | AddressProviderInstantiateMsg
  | IncentivesInstantiateMsg
  | WasmOracleInstantiateMsg
  | RewardsInstantiateMsg
  | ParamsInstantiateMsg
  | AstroportSwapperInstantiateMsg
  | OsmosisSwapperInstantiateMsg
  | OsmosisOracleInstantiateMsg

export interface UpdateOwner {
  owner: string
}
