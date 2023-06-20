import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { DeploymentConfig } from '../../types/config'
import { getDeployer, getWallet, setupClient } from './setupDeployer'
import { Storage } from './storage'
import { ExecuteMsg, OwnerUpdate } from '../../types/generated/mars-red-bank/MarsRedBank.types'
import { Coin } from '@cosmjs/proto-signing'
import { MarsAddresses } from '../../types/storageItems'
import { printBlue, printGreen, printRed, printYellow } from '../../utils/chalk'

class Signer {
  address: string
  client: SigningCosmWasmClient

  constructor(address: string, client: SigningCosmWasmClient) {
    this.address = address
    this.client = client
  }

  static async fromMnemonic(config: DeploymentConfig, mnemonic: string): Promise<Signer> {
    const wallet = await getWallet(config, mnemonic)
    const client = await setupClient(config, wallet)
    const address = await getDeployer(wallet)
    return new Signer(address, client)
  }

  async execute(contractAddress: string, msg: any, funds: readonly Coin[] = []) {
    return await this.client.execute(this.address, contractAddress, msg, 'auto', '', funds)
  }
}

export async function transferOwnership(config: DeploymentConfig) {
  const mnemonic = process.env.MNEMONIC
  if (!mnemonic) {
    throw new Error('The environment variable MNEMONIC is not set.')
  }
  const ownerSigner = await Signer.fromMnemonic(config, mnemonic)

  const newOwnerMnemonic = process.env.NEW_OWNER_MNEMONIC
  if (!newOwnerMnemonic) {
    throw new Error('The environment variable NEW_OWNER_MNEMONIC is not set.')
  }
  const newOwnerSigner = await Signer.fromMnemonic(config, newOwnerMnemonic)

  const storage = await Storage.load(config.chainId)

  try {
    for (const contract in storage.addresses) {
      const contractAddress = storage.addresses[contract as keyof MarsAddresses]
      if (!contractAddress) {
        throw new Error(`Contract address for ${contract} is not set in storage.`)
      }

      if (storage.execute.contractOwner[contract] !== newOwnerSigner.address) {
        const propose_owner: OwnerUpdate = {
          propose_new_owner: { proposed: newOwnerSigner.address },
        }
        const propose_msg: ExecuteMsg = { update_owner: propose_owner }

        await ownerSigner.execute(contractAddress, propose_msg)

        printYellow(`Ownership of ${contract} is proposed to ${newOwnerSigner.address}`)

        const accept_owner: OwnerUpdate = 'accept_proposed'
        const accept_msg: ExecuteMsg = { update_owner: accept_owner }
        await newOwnerSigner.execute(contractAddress, accept_msg)

        printGreen(`Ownership of ${contract} is accepted by ${newOwnerSigner.address}`)

        storage.execute.contractOwner[contract] = newOwnerSigner.address
      } else {
        printBlue('Ownership of ${contract} is already set to ${newOwnerSigner.address}')
      }

      if (storage.execute.contractAdmin[contract] !== newOwnerSigner.address) {
        printYellow(`Updating admin of ${contract} to ${newOwnerSigner.address}`)
        await ownerSigner.client.updateAdmin(
          ownerSigner.address,
          contractAddress,
          newOwnerSigner.address,
          'auto',
          '',
        )
        printGreen(`Admin of ${contract} is now ${newOwnerSigner.address}`)
        storage.execute.contractAdmin[contract] = newOwnerSigner.address
      } else {
        printBlue('Admin of ${contract} is already set to ${newOwnerSigner.address}')
      }
    }
  } catch (e) {
    printRed(e)
  } finally {
    await storage.save()
  }
}
