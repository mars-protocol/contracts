import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { DeploymentConfig, TestActions } from '../../types/config'
import { printBlue, printGray, printGreen } from '../../utils/chalk'
import { ARTIFACTS_PATH, Storage } from './storage'
import fs from 'fs'
import { InstantiateMsgs } from '../../types/instantiateMsgs'
import { InstantiateMsg as NftInstantiateMsg } from '../../types/generated/mars-account-nft/MarsAccountNft.types'
import { InstantiateMsg as VaultInstantiateMsg } from '../../types/generated/mars-mock-vault/MarsMockVault.types'
import { InstantiateMsg as HealthInstantiateMsg } from '../../types/generated/mars-rover-health-types/MarsRoverHealthTypes.types'
import {
  ExecuteMsg as SwapperExecute,
  InstantiateMsg as SwapperInstantiateMsg,
} from '../../types/generated/mars-swapper-base/MarsSwapperBase.types'
import { InstantiateMsg as ZapperInstantiateMsg } from '../../types/generated/mars-zapper-base/MarsZapperBase.types'
import {
  ExecuteMsg as CreditManagerExecute,
  InstantiateMsg as RoverInstantiateMsg,
} from '../../types/generated/mars-credit-manager/MarsCreditManager.types'
import { Rover } from './rover'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { getAddress, getWallet, setupClient } from './setupDeployer'
import { coin } from '@cosmjs/stargate'
import { Coin } from '@cosmjs/amino'
import { writeFile } from 'fs/promises'
import { join, resolve } from 'path'
import assert from 'assert'
import {
  MarsSwapperBaseClient,
  MarsSwapperBaseQueryClient,
} from '../../types/generated/mars-swapper-base/MarsSwapperBase.client'
import { MarsAccountNftClient } from '../../types/generated/mars-account-nft/MarsAccountNft.client'
import {
  MarsCreditManagerClient,
  MarsCreditManagerQueryClient,
} from '../../types/generated/mars-credit-manager/MarsCreditManager.client'
import { InitOrUpdateAssetParams } from '../../types/generated/mars-mock-red-bank/MarsMockRedBank.types'
import { kebabCase } from 'lodash'
import { MarsMockOracleQueryClient } from '../../types/generated/mars-mock-oracle/MarsMockOracle.client'
import { MarsRoverHealthTypesClient } from '../../types/generated/mars-rover-health-types/MarsRoverHealthTypes.client'

export class Deployer {
  constructor(
    private config: DeploymentConfig,
    public cwClient: SigningCosmWasmClient,
    public deployerAddr: string,
    public storage: Storage,
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
    printGreen(`${this.config.chain.id} :: ${name} : ${this.storage.codeIds[name]}`)
  }

  async instantiate(name: keyof Storage['addresses'], codeId: number, msg: InstantiateMsgs) {
    if (this.storage.addresses[name]) {
      printGray(`Contract already instantiated :: ${name} :: ${this.storage.addresses[name]}`)
      return
    }
    const { contractAddress } = await this.cwClient.instantiate(
      this.deployerAddr,
      codeId,
      msg,
      `mars-${kebabCase(name)}`,
      'auto',
      { admin: this.config.multisigAddr ? this.config.multisigAddr : this.deployerAddr },
    )
    this.storage.addresses[name] = contractAddress
    printGreen(
      `${this.config.chain.id} :: ${name} Contract Address : ${this.storage.addresses[name]}`,
    )
  }
  async instantiateHealthContract() {
    const msg: HealthInstantiateMsg = {
      owner: this.deployerAddr,
    }
    await this.instantiate('healthContract', this.storage.codeIds.healthContract!, msg)
  }

  async setCmOnHealthContract() {
    if (this.storage.actions.healthContractConfigUpdate) {
      printGray('Credit manager address')
    } else {
      let hExec = new MarsRoverHealthTypesClient(
        this.cwClient,
        this.deployerAddr,
        this.storage.addresses.healthContract!,
      )

      printBlue('Setting credit manager address on health contract config')
      await hExec.updateConfig({ creditManager: this.storage.addresses.creditManager! })
    }
    this.storage.actions.healthContractConfigUpdate = true
  }
  async instantiateNftContract() {
    const msg: NftInstantiateMsg = {
      max_value_for_burn: this.config.maxValueForBurn,
      minter: this.deployerAddr,
      name: 'credit-manager-accounts',
      symbol: 'rNFT',
    }
    await this.instantiate('accountNft', this.storage.codeIds.accountNft!, msg)
  }

  async instantiateMockVault() {
    if (!this.config.testActions) {
      printGray('No test actions, mock vault not needed')
      return
    }

    const msg: VaultInstantiateMsg = {
      base_token_denom: this.config.testActions.vault.mock.baseToken,
      oracle: this.config.oracle.addr,
      vault_token_denom: this.config.testActions.vault.mock.vaultTokenDenom,
      lockup: this.config.testActions.vault.mock.lockup,
    }
    await this.instantiate('mockVault', this.storage.codeIds.mockVault!, msg)

    // Temporary until Token Factory is integrated into Cosmwasm or Apollo Vaults are in testnet
    if (!this.storage.actions.seedMockVault) {
      printBlue('Seeding mock vault')
      await this.transferCoin(
        this.storage.addresses.mockVault!,
        coin(10_000_000, this.config.testActions.vault.mock.vaultTokenDenom),
      )
      this.storage.actions.seedMockVault = true
    } else {
      printGray('Mock vault already seeded')
    }
  }

  async instantiateSwapper() {
    const msg: SwapperInstantiateMsg = {
      owner: this.deployerAddr,
    }
    await this.instantiate('swapper', this.storage.codeIds.swapper!, msg)

    if (!this.storage.actions.setRoutes) {
      const swapClient = new MarsSwapperBaseClient(
        this.cwClient,
        this.deployerAddr,
        this.storage.addresses.swapper!,
      )

      for (const route of this.config.swapRoutes) {
        printBlue(`Setting ${route.denomIn}-${route.denomOut} route for swapper contract`)
        // @ts-expect-error ts-codegen cannot parse the generic
        await swapClient.setRoute(route)
      }

      const swapQuery = new MarsSwapperBaseQueryClient(
        this.cwClient,
        this.storage.addresses.swapper!,
      )
      const routes = await swapQuery.routes({})
      assert.equal(routes.length, this.config.swapRoutes.length)
      this.storage.actions.setRoutes = true
    } else {
      printGray("Swap contract's routes already set")
    }
  }

  async instantiateZapper() {
    const msg: ZapperInstantiateMsg = {}
    await this.instantiate('zapper', this.storage.codeIds.zapper!, msg)
  }

  async instantiateCreditManager() {
    const msg: RoverInstantiateMsg = {
      max_unlocking_positions: this.config.maxUnlockingPositions,
      allowed_coins: this.config.allowedCoins,
      vault_configs: this.config.vaults.map((v) => ({ config: v.config, vault: v.vault })),
      oracle: this.config.oracle.addr,
      owner: this.deployerAddr,
      red_bank: this.config.redBank.addr,
      max_close_factor: this.config.maxCloseFactor,
      swapper: this.storage.addresses.swapper!,
      zapper: this.storage.addresses.zapper!,
      health_contract: this.storage.addresses.healthContract!,
    }

    if (this.config.testActions) {
      msg.vault_configs.push({
        vault: {
          address: this.storage.addresses.mockVault!,
        },
        config: this.config.testActions.vault.mock.config,
      })
    }

    await this.instantiate('creditManager', this.storage.codeIds.creditManager!, msg)
  }

  async transferNftContractOwnership() {
    if (!this.storage.actions.proposedNewOwner) {
      const nftClient = new MarsAccountNftClient(
        this.cwClient,
        this.deployerAddr,
        this.storage.addresses.accountNft!,
      )
      await nftClient.updateConfig({
        updates: { proposed_new_minter: this.storage.addresses.creditManager! },
      })
      this.storage.actions.proposedNewOwner = true
      printBlue('Nft contract owner proposes Rover as new owner')
    } else {
      printGray('Nft contact owner change already proposed')
    }

    if (!this.storage.actions.acceptedOwnership) {
      const client = new MarsCreditManagerClient(
        this.cwClient,
        this.deployerAddr,
        this.storage.addresses.creditManager!,
      )
      await client.updateConfig({ updates: { account_nft: this.storage.addresses.accountNft } })
      this.storage.actions.acceptedOwnership = true
      printGreen(`Rover accepts ownership of Nft contract`)
    } else {
      printGray('Rover already accepted Nft contract ownership')
    }
  }

  async newUserRoverClient(testActions: TestActions) {
    const { client, address } = await this.generateNewAddress()
    printBlue(`New user: ${address}`)
    await this.transferCoin(
      address,
      coin(testActions.startingAmountForTestUser, this.config.chain.baseDenom),
    )
    return this.getRoverClient(address, client, testActions)
  }

  async saveDeploymentAddrsToFile(label: string) {
    const addressesDir = resolve(join(__dirname, '../../../deploy/addresses'))
    await writeFile(
      `${addressesDir}/${this.config.chain.id}-${label}.json`,
      JSON.stringify(this.storage.addresses),
    )
  }

  async grantCreditLines() {
    if (this.storage.actions.grantedCreditLines) {
      printGray('Credit lines already granted')
      return
    }

    const wallet = await getWallet(
      this.config.testActions!.outpostsDeployerMnemonic,
      this.config.chain.prefix,
    )
    const client = await setupClient(this.config, wallet)
    const addr = await getAddress(wallet)

    for (const denom of this.config.testActions?.allowedCoinsConfig
      .filter((c) => c.grantCreditLine)
      .map((c) => c.denom) ?? []) {
      const msg = {
        update_uncollateralized_loan_limit: {
          user: this.storage.addresses.creditManager,
          denom,
          new_limit: this.config.testActions!.defaultCreditLine,
        },
      }
      printBlue(
        `Granting credit line to Rover for: ${this.config.testActions!.defaultCreditLine} ${denom}`,
      )
      await client.execute(addr, this.config.redBank.addr, msg, 'auto')
    }

    this.storage.actions.grantedCreditLines = true
  }

  async setupOraclePrices() {
    if (this.storage.actions.oraclePricesSet) {
      printGray('Oracle prices already set')
      return
    }

    for (const coin of this.config.testActions?.allowedCoinsConfig ?? []) {
      const oQuery = new MarsMockOracleQueryClient(this.cwClient, this.config.oracle.addr)
      try {
        await oQuery.price({ denom: coin.denom })
        printGray(`Price source already set for ${coin.denom}`)
      } catch (e) {
        const msg = {
          set_price_source: {
            denom: coin.denom,
            price_source: coin.priceSource,
          },
        }
        printBlue(`Setting price source for ${coin.denom}: ${JSON.stringify(coin.priceSource)}`)
        const { client, addr } = await this.getOutpostsDeployer()
        await client.execute(addr, this.config.oracle.addr, msg, 'auto')
      }
    }
    this.storage.actions.oraclePricesSet = true
  }

  async setupRedBankMarkets() {
    if (this.storage.actions.redBankMarketsSet) {
      printGray('Red bank markets already set')
      return
    }

    const { client, addr } = await this.getOutpostsDeployer()

    for (const denom of this.config.testActions?.allowedCoinsConfig.map((c) => c.denom) ?? []) {
      try {
        await client.queryContractSmart(this.config.redBank.addr, {
          market: {
            denom,
          },
        })
        printGray(`Market for ${denom} already set`)
      } catch {
        const msg: {
          init_asset: {
            denom: string
            params: InitOrUpdateAssetParams
          }
        } = {
          init_asset: {
            denom,
            params: {
              max_loan_to_value: '0.65',
              reserve_factor: '0.2',
              liquidation_threshold: '0.7',
              liquidation_bonus: '0.1',
              interest_rate_model: {
                optimal_utilization_rate: '0.1',
                base: '0.3',
                slope_1: '0.25',
                slope_2: '0.3',
              },
              deposit_cap: '1000000000',
              deposit_enabled: true,
              borrow_enabled: true,
            },
          },
        }
        printBlue(`Setting market for ${denom}`)
        await client.execute(addr, this.config.redBank.addr, msg, 'auto')
      }
    }
    this.storage.actions.redBankMarketsSet = true
  }

  async updateCreditManagerOwner() {
    if (!this.config.multisigAddr) throw new Error('No multisig addresses to transfer ownership to')

    const msg: CreditManagerExecute = {
      update_owner: {
        propose_new_owner: {
          proposed: this.config.multisigAddr,
        },
      },
    }
    await this.cwClient.execute(
      this.deployerAddr,
      this.storage.addresses.creditManager!,
      msg,
      'auto',
    )
    printGreen('Owner updated to Multisig for Credit Manager Contract')

    const cmQuery = new MarsCreditManagerQueryClient(
      this.cwClient,
      this.storage.addresses.creditManager!,
    )
    const creditManagerConfig = await cmQuery.config()
    assert.equal(creditManagerConfig.proposed_new_owner, this.config.multisigAddr)
  }

  async updateSwapperOwner() {
    if (!this.config.multisigAddr) throw new Error('No multisig addresses to transfer ownership to')

    const msg: SwapperExecute = {
      update_owner: {
        propose_new_owner: {
          proposed: this.config.multisigAddr,
        },
      },
    }
    await this.cwClient.execute(this.deployerAddr, this.storage.addresses.swapper!, msg, 'auto')
    printGreen('Owner updated to Multisig for Swapper Contract')

    const swQuery = new MarsSwapperBaseQueryClient(this.cwClient, this.storage.addresses.swapper!)
    const swapperOwner = await swQuery.owner()
    assert.equal(swapperOwner.proposed, this.config.multisigAddr)
  }

  private async getOutpostsDeployer() {
    const wallet = await getWallet(
      this.config.testActions!.outpostsDeployerMnemonic,
      this.config.chain.prefix,
    )
    const client = await setupClient(this.config, wallet)
    const addr = await getAddress(wallet)
    return { client, addr }
  }

  private async transferCoin(recipient: string, coin: Coin) {
    await this.cwClient.sendTokens(this.deployerAddr, recipient, [coin], 'auto')
    const balance = await this.cwClient.getBalance(recipient, coin.denom)
    printBlue(`New balance: ${balance.amount} ${balance.denom}`)
  }

  private async generateNewAddress() {
    const { mnemonic } = await DirectSecp256k1HdWallet.generate(24)
    const wallet = await getWallet(mnemonic, this.config.chain.prefix)
    const client = await setupClient(this.config, wallet)
    const address = await getAddress(wallet)
    return { client, address }
  }

  private getRoverClient(address: string, client: SigningCosmWasmClient, testActions: TestActions) {
    return new Rover(address, this.storage, this.config, client, testActions)
  }
}
