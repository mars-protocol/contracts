import { InstantiateMsg as NftInstantiateMsg } from './generated/mars-account-nft/MarsAccountNft.types'
import { InstantiateMsg as RedBankInstantiateMsg } from './generated/mars-mock-red-bank/MarsMockRedBank.types'
import { InstantiateMsg as VaultInstantiateMsg } from './generated/mars-mock-vault/MarsMockVault.types'
import { InstantiateMsg as OracleInstantiateMsg } from './generated/mars-mock-oracle/MarsMockOracle.types'
import { InstantiateMsg as RoverInstantiateMsg } from './generated/mars-credit-manager/MarsCreditManager.types'
import { InstantiateMsg as SwapperInstantiateMsg } from './generated/mars-swapper-base/MarsSwapperBase.types'
import { InstantiateMsg as ZapperInstantiateMsg } from './generated/mars-zapper-base/MarsZapperBase.types'

export type InstantiateMsgs =
  | NftInstantiateMsg
  | RedBankInstantiateMsg
  | VaultInstantiateMsg
  | OracleInstantiateMsg
  | RoverInstantiateMsg
  | SwapperInstantiateMsg
  | ZapperInstantiateMsg
