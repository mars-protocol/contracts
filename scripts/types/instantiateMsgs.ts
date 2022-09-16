import { InstantiateMsg as NftInstantiateMsg } from './generated/account-nft/AccountNft.types'
import { InstantiateMsg as RedBankInstantiateMsg } from './generated/mock-red-bank/MockRedBank.types'
import { InstantiateMsg as VaultInstantiateMsg } from './generated/mock-vault/MockVault.types'
import { InstantiateMsg as OracleInstantiateMsg } from './generated/mock-oracle/MockOracle.types'
import { InstantiateMsg as RoverInstantiateMsg } from './generated/credit-manager/CreditManager.types'
import { InstantiateMsg as SwapperInstantiateMsg } from './generated/swapper-base/SwapperBase.types'

export type InstantiateMsgs =
  | NftInstantiateMsg
  | RedBankInstantiateMsg
  | VaultInstantiateMsg
  | OracleInstantiateMsg
  | RoverInstantiateMsg
  | SwapperInstantiateMsg
