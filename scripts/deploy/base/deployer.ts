import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { DeploymentConfig } from '../../types/config'
import { printBlue, printGray, printGreen } from '../../utils/chalk'
import { ARTIFACTS_PATH, Storage } from './storage'
import fs from 'fs'
import { InstantiateMsgs } from '../../types/instantiateMsgs'
import { InstantiateMsg as NftInstantiateMsg } from '../../types/generated/account-nft/AccountNft.types'
import { InstantiateMsg as VaultInstantiateMsg } from '../../types/generated/mock-vault/MockVault.types'
import { InstantiateMsg as SwapperInstantiateMsg } from '../../types/generated/swapper-base/SwapperBase.types'
import { InstantiateMsg as OracleAdapterInstantiateMsg } from '../../types/generated/mars-oracle-adapter/MarsOracleAdapter.types'
import { InstantiateMsg as RoverInstantiateMsg } from '../../types/generated/credit-manager/CreditManager.types'
import { Rover } from './rover'
import { AccountNftClient } from '../../types/generated/account-nft/AccountNft.client'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { getAddress, getWallet, setupClient } from './setupDeployer'
import { coins } from '@cosmjs/stargate'
import { Coin } from '@cosmjs/amino'
import { writeFile } from 'fs/promises'
import { join, resolve } from 'path'
import {
  SwapperBaseClient,
  SwapperBaseQueryClient,
} from '../../types/generated/swapper-base/SwapperBase.client'
import assert from 'assert'

export class Deployer {
  constructor(
    private config: DeploymentConfig,
    private cwClient: SigningCosmWasmClient,
    private deployerAddr: string,
    private storage: Storage,
  ) {}

  async saveStorage() {
    await this.storage.save()
  }

  async upload(name: keyof Storage['codeIds'], file: string) {
    if (this.storage.codeIds[name]) {
      printGray(`Wasm already uploaded :: ${name} :: ${this.storage.codeIds[name]}`)
      return
    }
    const wasm = fs.readFileSync(ARTIFACTS_PATH + file)
    const uploadResult = await this.cwClient.upload(this.deployerAddr, wasm, 'auto')
    this.storage.codeIds[name] = uploadResult.codeId
    printGreen(`${this.config.chainId} :: ${name} : ${this.storage.codeIds[name]}`)
  }

  async instantiate(name: keyof Storage['addresses'], codeId: number, msg: InstantiateMsgs) {
    if (this.storage.addresses[name]) {
      printGray(`Contract already instantiated :: ${name} :: ${this.storage.addresses[name]}`)
      return
    }
    const { contractAddress } = await this.cwClient.instantiate(
      this.deployerAddr,
      codeId,
      // @ts-expect-error expecting generic record
      msg,
      `mars-${name}`,
      'auto',
    )
    this.storage.addresses[name] = contractAddress
    printGreen(
      `${this.config.chainId} :: ${name} Contract Address : ${this.storage.addresses[name]}`,
    )
  }

  async instantiateNftContract() {
    const msg: NftInstantiateMsg = {
      minter: this.deployerAddr,
      name: 'credit-manger-accounts',
      symbol: 'rover-nft',
    }
    await this.instantiate('accountNft', this.storage.codeIds.accountNft!, msg)
  }

  async instantiateMockVault() {
    const msg: VaultInstantiateMsg = {
      asset_denoms: [this.config.baseDenom],
      lp_token_denom: this.config.vaultTokenDenom,
      oracle: this.config.oracleAddr,
    }
    await this.instantiate('mockVault', this.storage.codeIds.mockVault!, msg)
  }

  async instantiateMarsOracleAdapter() {
    const msg: OracleAdapterInstantiateMsg = {
      oracle: this.config.oracleAddr,
      owner: this.deployerAddr,
      vault_pricing: [
        {
          addr: this.storage.addresses.mockVault!,
          denom: this.config.vaultTokenDenom,
          method: 'preview_redeem',
        },
      ],
    }
    await this.instantiate('marsOracleAdapter', this.storage.codeIds.marsOracleAdapter!, msg)
  }

  async instantiateSwapper() {
    const msg: SwapperInstantiateMsg = {
      owner: this.deployerAddr,
    }
    await this.instantiate('swapper', this.storage.codeIds.swapper!, msg)

    await this.transferFunds(this.storage.addresses.swapper!, [
      { denom: this.config.baseDenom, amount: '100' },
    ])

    const swapClient = new SwapperBaseClient(
      this.cwClient,
      this.deployerAddr,
      this.storage.addresses.swapper!,
    )
    printBlue(
      `Setting ${this.config.baseDenom}-${this.config.secondaryDenom} route for swap contract`,
    )
    await swapClient.setRoute({
      denomIn: this.config.baseDenom,
      denomOut: this.config.secondaryDenom,
      route: this.config.swapRoute,
    })

    const swapQuery = new SwapperBaseQueryClient(this.cwClient, this.storage.addresses.swapper!)
    const routes = await swapQuery.routes({})
    assert.equal(routes.length, 1)
  }

  async instantiateCreditManager() {
    const msg: RoverInstantiateMsg = {
      allowed_coins: [this.config.baseDenom, this.config.secondaryDenom],
      allowed_vaults: [{ address: this.storage.addresses.mockVault! }],
      oracle: this.config.oracleAddr,
      owner: this.deployerAddr,
      red_bank: this.config.redBankAddr,
      max_close_factor: this.config.maxCloseFactor.toString(),
      max_liquidation_bonus: this.config.maxLiquidationBonus.toString(),
      swapper: this.storage.addresses.swapper!,
    }
    await this.instantiate('creditManager', this.storage.codeIds.creditManager!, msg)
  }

  async transferNftContractOwnership() {
    if (!this.storage.actions.proposedNewOwner) {
      const nftClient = new AccountNftClient(
        this.cwClient,
        this.deployerAddr,
        this.storage.addresses.accountNft!,
      )
      await nftClient.proposeNewOwner({ newOwner: this.storage.addresses.creditManager! })
      this.storage.actions.proposedNewOwner = true
      printBlue('Nft contract owner proposes Rover as new owner')
    } else {
      printGray('Nft contact owner change already proposed')
    }

    if (!this.storage.actions.acceptedOwnership) {
      const rover = this.getRoverClient(this.deployerAddr, this.cwClient)
      await rover.updateConfig({ account_nft: this.storage.addresses.accountNft })
      this.storage.actions.acceptedOwnership = true
      printGreen(`Rover accepts ownership of Nft contract`)
    } else {
      printGray('Rover already accepted Nft contract ownership')
    }
  }

  async newUserRoverClient() {
    const { client, address } = await this.generateNewAddress()
    printBlue(`New user: ${address}`)
    await this.transferFunds(
      address,
      coins(this.config.startingAmountForTestUser, this.config.baseDenom),
    )
    return this.getRoverClient(address, client)
  }

  async saveDeploymentAddrsToFile() {
    const addressesDir = resolve(join(__dirname, '../../../deploy/addresses'))
    await writeFile(
      `${addressesDir}/${this.config.chainId}.json`,
      JSON.stringify(this.storage.addresses),
    )
  }

  private async transferFunds(recipient: string, coins: Coin[]) {
    await this.cwClient.sendTokens(this.deployerAddr, recipient, coins, 'auto')
    const balance = await this.cwClient.getBalance(recipient, this.config.baseDenom)
    printBlue(`New balance: ${balance.amount} ${balance.denom}`)
  }

  private async generateNewAddress() {
    const { mnemonic } = await DirectSecp256k1HdWallet.generate(24)
    const wallet = await getWallet(mnemonic, this.config.chainPrefix)
    const client = await setupClient(this.config, wallet)
    const address = await getAddress(wallet)
    return { client, address }
  }

  private getRoverClient(address: string, client: SigningCosmWasmClient) {
    return new Rover(address, this.storage, this.config, client)
  }
}
