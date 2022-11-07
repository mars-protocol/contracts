import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { DeploymentConfig } from '../../types/config'
import { printBlue, printGray, printGreen } from '../../utils/chalk'
import { ARTIFACTS_PATH, Storage } from './storage'
import fs from 'fs'
import { InstantiateMsgs } from '../../types/instantiateMsgs'
import { InstantiateMsg as NftInstantiateMsg } from '../../types/generated/mars-account-nft/MarsAccountNft.types'
import { InstantiateMsg as VaultInstantiateMsg } from '../../types/generated/mars-mock-vault/MarsMockVault.types'
import { InstantiateMsg as SwapperInstantiateMsg } from '../../types/generated/mars-swapper-base/MarsSwapperBase.types'
import { InstantiateMsg as ZapperInstantiateMsg } from '../../types/generated/mars-mock-zapper/MarsMockZapper.types'
import { InstantiateMsg as RoverInstantiateMsg } from '../../types/generated/mars-credit-manager/MarsCreditManager.types'
import { InstantiateMsg as OracleAdapterInstantiateMsg } from '../../types/generated/mars-oracle-adapter/MarsOracleAdapter.types'
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
      base_token_denom: this.config.baseDenom,
      oracle: this.config.oracleAddr,
      vault_token_denom: this.config.vaultTokenDenom,
    }
    await this.instantiate('mockVault', this.storage.codeIds.mockVault!, msg)

    // Temporary until Token Factory is integrated into Cosmwasm or Apollo Vaults are in testnet
    if (!this.storage.actions.seedMockVault) {
      printBlue('Seeding mock vault')
      await this.transferCoin(
        this.storage.addresses.mockVault!,
        coin(10_000_000, this.config.vaultTokenDenom),
      )
      this.storage.actions.seedMockVault = true
    } else {
      printGray('Mock vault already seeded')
    }
  }

  async instantiateMarsOracleAdapter() {
    const msg: OracleAdapterInstantiateMsg = {
      oracle: this.config.oracleAddr,
      owner: this.deployerAddr,
      vault_pricing: [
        {
          addr: this.storage.addresses.mockVault!,
          method: 'preview_redeem',
          base_denom: this.config.baseDenom,
          vault_coin_denom: this.config.vaultTokenDenom,
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

    if (!this.storage.actions.setRouteAndSeedSwapper) {
      printBlue(`Seeding swapper w/ ${this.config.baseDenom}`)
      await this.transferCoin(this.storage.addresses.swapper!, coin(100, this.config.baseDenom))

      const swapClient = new MarsSwapperBaseClient(
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

      const swapQuery = new MarsSwapperBaseQueryClient(
        this.cwClient,
        this.storage.addresses.swapper!,
      )
      const routes = await swapQuery.routes({})
      assert.equal(routes.length, 1)
      this.storage.actions.setRouteAndSeedSwapper = true
    } else {
      printGray('Swap contract already seeded with funds')
    }
  }

  async instantiateZapper() {
    const msg: ZapperInstantiateMsg = {
      oracle: this.storage.addresses.marsOracleAdapter!,
      lp_configs: [
        {
          lp_token_denom: this.config.lpToken.denom,
          lp_pair_denoms: [this.config.zap[0].denom, this.config.zap[1].denom],
        },
      ],
    }
    await this.instantiate('mockZapper', this.storage.codeIds.mockZapper!, msg)

    // Temporary until Token Factory is integrated into Cosmwasm or Apollo Vaults are in testnet
    if (!this.storage.actions.seedMockZapper) {
      printBlue('Seeding mock zapper')
      await this.transferCoin(
        this.storage.addresses.mockZapper!,
        coin(10_000_000, this.config.lpToken.denom),
      )
      this.storage.actions.seedMockZapper = true
    } else {
      printGray('Mock zapper already seeded')
    }
  }

  async instantiateCreditManager() {
    const msg: RoverInstantiateMsg = {
      allowed_coins: [this.config.baseDenom, this.config.secondaryDenom, this.config.lpToken.denom],
      allowed_vaults: [
        {
          config: {
            deposit_cap: this.config.vaultDepositCap,
            liquidation_threshold: this.config.vaultLiquidationThreshold.toString(),
            max_ltv: this.config.vaultMaxLTV.toString(),
            whitelisted: true,
          },
          vault: { address: this.storage.addresses.mockVault! },
        },
      ],
      oracle: this.storage.addresses.marsOracleAdapter!,
      owner: this.deployerAddr,
      red_bank: this.config.redBankAddr,
      max_close_factor: this.config.maxCloseFactor.toString(),
      max_liquidation_bonus: this.config.maxLiquidationBonus.toString(),
      swapper: this.storage.addresses.swapper!,
      zapper: this.storage.addresses.mockZapper!,
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
    await this.transferCoin(
      address,
      coin(this.config.startingAmountForTestUser, this.config.baseDenom),
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

  async grantCreditLines() {
    if (this.storage.actions.grantedCreditLines) {
      printGray('Credit lines already granted')
      return
    }

    const wallet = await getWallet(this.config.redBankDeployerMnemonic, this.config.chainPrefix)
    const client = await setupClient(this.config, wallet)
    const addr = await getAddress(wallet)

    for (const coin of this.config.toGrantCreditLines) {
      const msg = {
        update_uncollateralized_loan_limit: {
          user: this.storage.addresses.creditManager,
          denom: coin.denom,
          new_limit: coin.amount.toString(),
        },
      }

      printBlue(`Granting credit line to Rover for: ${coin.amount} ${coin.denom}`)
      await client.execute(addr, this.config.redBankAddr, msg, 'auto')
    }

    this.storage.actions.grantedCreditLines = true
  }

  async setupOraclePricesForZapDenoms() {
    if (this.storage.actions.oraclePricesSet) {
      printGray('Oracle prices already set')
      return
    }

    const { client, addr } = await this.getOutpostsDeployer()

    for (const coin of this.config.zap
      .map((c) => ({ denom: c.denom, price: c.price }))
      .concat(this.config.lpToken)) {
      try {
        await client.queryContractSmart(this.config.oracleAddr, {
          price: {
            denom: coin.denom,
          },
        })
        printGray(`Price for ${coin.denom} already set`)
      } catch {
        const msg = {
          set_price_source: {
            denom: coin.denom,
            price_source: {
              fixed: { price: coin.price.toString() },
            },
          },
        }
        console.log(JSON.stringify(msg))
        printBlue(`Setting price for ${coin.denom}: ${coin.price}`)
        await client.execute(addr, this.config.oracleAddr, msg, 'auto')
      }
    }
    this.storage.actions.oraclePricesSet = true
  }

  async setupRedBankMarketsForZapDenoms() {
    if (this.storage.actions.redBankMarketsSet) {
      printGray('Red bank markets already set')
      return
    }
    const { client, addr } = await this.getOutpostsDeployer()

    for (const denom of this.config.zap.map((c) => c.denom).concat(this.config.lpToken.denom)) {
      try {
        await client.queryContractSmart(this.config.redBankAddr, {
          market: {
            denom,
          },
        })
        printGray(`Market for ${denom} already set`)
      } catch {
        const msg = {
          init_asset: {
            denom,
            initial_borrow_rate: '0.1',
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
            symbol: denom,
          },
        }
        printBlue(`Setting market for ${denom}`)
        await client.execute(addr, this.config.redBankAddr, msg, 'auto')
      }
    }
    this.storage.actions.redBankMarketsSet = true
  }

  private async getOutpostsDeployer() {
    const wallet = await getWallet(this.config.redBankDeployerMnemonic, this.config.chainPrefix)
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
    const wallet = await getWallet(mnemonic, this.config.chainPrefix)
    const client = await setupClient(this.config, wallet)
    const address = await getAddress(wallet)
    return { client, address }
  }

  private getRoverClient(address: string, client: SigningCosmWasmClient) {
    return new Rover(address, this.storage, this.config, client)
  }
}
