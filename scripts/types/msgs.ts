import { InstantiateMsg as ParamsInstantiateMsg } from './generated/mars-params/MarsParams.types'
import { InstantiateMsg as RedBankInstantiateMsg } from './generated/mars-red-bank/MarsRedBank.types'
import { InstantiateMsg as AddressProviderInstantiateMsg } from './generated/mars-address-provider/MarsAddressProvider.types'
import { InstantiateMsg as IncentivesInstantiateMsg } from './generated/mars-incentives/MarsIncentives.types'
import { InstantiateMsg as RewardsInstantiateMsg } from './generated/mars-rewards-collector-base/MarsRewardsCollectorBase.types'
import { InstantiateMsg as OsmosisOracleInstantiateMsg } from './generated/mars-oracle-osmosis/MarsOracleOsmosis.types'
import { InstantiateMsg as WasmOracleInstantiateMsg } from './generated/mars-oracle-wasm/MarsOracleWasm.types'
import { InstantiateMsg as OsmosisSwapperInstantiateMsg } from './generated/mars-swapper-osmosis/MarsSwapperOsmosis.types'
import { InstantiateMsg as AstroportSwapperInstantiateMsg } from './generated/mars-swapper-astroport/MarsSwapperAstroport.types'
import { InstantiateMsg as NftInstantiateMsg } from './generated/mars-account-nft/MarsAccountNft.types'
import { InstantiateMsg as VaultInstantiateMsg } from './generated/mars-mock-vault/MarsMockVault.types'
import { InstantiateMsg as RoverInstantiateMsg } from './generated/mars-credit-manager/MarsCreditManager.types'
import { InstantiateMsg as ZapperInstantiateMsg } from './generated/mars-zapper-base/MarsZapperBase.types'
import { InstantiateMsg as HealthInstantiateMsg } from './generated/mars-rover-health/MarsRoverHealth.types'

export type InstantiateMsgs =
  | ParamsInstantiateMsg
  | RedBankInstantiateMsg
  | AddressProviderInstantiateMsg
  | IncentivesInstantiateMsg
  | RewardsInstantiateMsg
  | OsmosisOracleInstantiateMsg
  | WasmOracleInstantiateMsg
  | OsmosisSwapperInstantiateMsg
  | AstroportSwapperInstantiateMsg
  | NftInstantiateMsg
  | VaultInstantiateMsg
  | RoverInstantiateMsg
  | ZapperInstantiateMsg
  | HealthInstantiateMsg

export interface UpdateOwner {
  owner: string
}
