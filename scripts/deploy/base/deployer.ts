import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import {
  AssetConfig,
  AstroportConfig,
  DeploymentConfig,
  OracleConfig,
  SwapperExecuteMsg,
  TestActions,
  VaultConfig,
  isAstroportRoute,
} from '../../types/config'
import { printBlue, printGray, printGreen, printRed, printYellow } from '../../utils/chalk'
import { ARTIFACTS_PATH, Storage } from './storage'
import fs from 'fs'
import { InstantiateMsgs } from '../../types/msgs'
import { InstantiateMsg as NftInstantiateMsg } from '../../types/generated/mars-account-nft/MarsAccountNft.types'
import { InstantiateMsg as VaultInstantiateMsg } from '../../types/generated/mars-mock-vault/MarsMockVault.types'
import { InstantiateMsg as HealthInstantiateMsg } from '../../types/generated/mars-rover-health/MarsRoverHealth.types'
import { InstantiateMsg as ZapperInstantiateMsg } from '../../types/generated/mars-zapper-base/MarsZapperBase.types'
import {
  ExecuteMsg as CreditManagerExecute,
  InstantiateMsg as RoverInstantiateMsg,
} from '../../types/generated/mars-credit-manager/MarsCreditManager.types'
import {
  InstantiateMsg as AstroportSwapperInstantiateMsg,
  AstroportConfig as SwapperAstroportConfig,
} from '../../types/generated/mars-swapper-astroport/MarsSwapperAstroport.types'
import { InstantiateMsg as OsmosisSwapperInstantiateMsg } from '../../types/generated/mars-swapper-osmosis/MarsSwapperOsmosis.types'
import { InstantiateMsg as ParamsInstantiateMsg } from '../../types/generated/mars-params/MarsParams.types'
import { ExecuteMsg as ParamsExecuteMsg } from '../../types/generated/mars-params/MarsParams.types'
import {
  InstantiateMsg as RedBankInstantiateMsg,
  ExecuteMsg as RedBankExecuteMsg,
  QueryMsg as RedBankQueryMsg,
} from '../../types/generated/mars-red-bank/MarsRedBank.types'
import {
  AddressResponseItem,
  InstantiateMsg as AddressProviderInstantiateMsg,
} from '../../types/generated/mars-address-provider/MarsAddressProvider.types'
import { InstantiateMsg as IncentivesInstantiateMsg } from '../../types/generated/mars-incentives/MarsIncentives.types'
import { InstantiateMsg as RewardsInstantiateMsg } from '../../types/generated/mars-rewards-collector-base/MarsRewardsCollectorBase.types'
import {
  WasmOracleCustomInitParams,
  InstantiateMsg as WasmOracleInstantiateMsg,
} from '../../types/generated/mars-oracle-wasm/MarsOracleWasm.types'
import { InstantiateMsg as OsmosisOracleInstantiateMsg } from '../../types/generated/mars-oracle-osmosis/MarsOracleOsmosis.types'
import { ExecuteMsg as WasmOracleExecuteMsg } from '../../types/generated/mars-oracle-wasm/MarsOracleWasm.types'
import { Rover } from './test-actions-credit-manager'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { getAddress, getWallet, setupClient } from './setup-deployer'
import { coin } from '@cosmjs/stargate'
import { Coin } from '@cosmjs/amino'
import { writeFile } from 'fs/promises'
import { join, resolve } from 'path'
import assert from 'assert'
import { MarsAccountNftClient } from '../../types/generated/mars-account-nft/MarsAccountNft.client'
import {
  MarsCreditManagerClient,
  MarsCreditManagerQueryClient,
} from '../../types/generated/mars-credit-manager/MarsCreditManager.client'
import { kebabCase } from 'lodash'
import { MarsRoverHealthClient } from '../../types/generated/mars-rover-health/MarsRoverHealth.client'

type SwapperInstantiateMsg = AstroportSwapperInstantiateMsg | OsmosisSwapperInstantiateMsg
type OracleInstantiateMsg = WasmOracleInstantiateMsg | OsmosisOracleInstantiateMsg

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

  async assertDeployerBalance() {
    const accountBalance = await this.cwClient.getBalance(
      this.deployerAddr,
      this.config.chain.baseDenom,
    )
    printYellow(
      `${this.config.chain.baseDenom} account balance is: ${accountBalance.amount} (${
        Number(accountBalance.amount) / 1e6
      } ${this.config.chain.prefix})`,
    )
    if (Number(accountBalance.amount) < 1_000_000 && this.config.chain.id === 'osmo-test-5') {
      printRed(
        `not enough ${this.config.chain.prefix} tokens to complete action, you may need to go to a test faucet to get more tokens.`,
      )
    }
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
    await this.instantiate('health', this.storage.codeIds.health!, msg)
  }

  async setConfigOnHealthContract() {
    if (this.storage.actions.healthContractConfigUpdate) {
      printGray('health contract config already updated')
    } else {
      const hExec = new MarsRoverHealthClient(
        this.cwClient,
        this.deployerAddr,
        this.storage.addresses.health!,
      )

      printBlue('Setting credit manager address on health contract config')
      await hExec.updateConfig({
        creditManager: this.storage.addresses.creditManager!,
      })
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
      oracle: this.storage.addresses.zapper!,
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

  async instantiateZapper() {
    const msg: ZapperInstantiateMsg = {}
    await this.instantiate('zapper', this.storage.codeIds.zapper!, msg)
  }

  async instantiateCreditManager() {
    const msg: RoverInstantiateMsg = {
      params: this.storage.addresses.params!,
      max_unlocking_positions: this.config.maxUnlockingPositions,
      max_slippage: this.config.maxSlippage,
      oracle: this.storage.addresses.oracle!,
      owner: this.deployerAddr,
      red_bank: this.storage.addresses.redBank!,
      swapper: this.storage.addresses.swapper!,
      zapper: this.storage.addresses.zapper!,
      health_contract: this.storage.addresses.health!,
      incentives: this.storage.addresses.incentives!,
    }

    await this.instantiate('creditManager', this.storage.codeIds.creditManager!, msg)
  }

  async setConfigOnCreditManagerContract() {
    if (this.storage.actions.creditManagerContractConfigUpdate) {
      printGray('credit manager contract config already updated')
    } else {
      const hExec = new MarsCreditManagerClient(
        this.cwClient,
        this.deployerAddr,
        this.storage.addresses.creditManager!,
      )

      printBlue(
        'Setting health and credit-manager addresses in nft contract via credit manager contract',
      )
      await hExec.updateNftConfig({
        config: {
          health_contract_addr: this.storage.addresses.health!,
          credit_manager_contract_addr: this.storage.addresses.creditManager!,
        },
      })

      printBlue('Setting rewards-collector address in credit manager contract')
      await hExec.updateConfig({
        updates: {
          rewards_collector: this.storage.addresses.rewardsCollector!,
        },
      })
    }
    this.storage.actions.creditManagerContractConfigUpdate = true
  }

  async transferNftContractOwnership() {
    if (!this.storage.actions.proposedNewOwner) {
      const nftClient = new MarsAccountNftClient(
        this.cwClient,
        this.deployerAddr,
        this.storage.addresses.accountNft!,
      )
      await nftClient.updateOwnership({
        transfer_ownership: {
          new_owner: this.storage.addresses.creditManager!,
        },
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
    assert.equal(creditManagerConfig.ownership.proposed, this.config.multisigAddr)
  }

  async updateHealthOwner() {
    if (!this.config.multisigAddr) throw new Error('No multisig addresses to transfer ownership to')

    const hExec = new MarsRoverHealthClient(
      this.cwClient,
      this.deployerAddr,
      this.storage.addresses.health!,
    )

    await hExec.updateOwner({
      propose_new_owner: {
        proposed: this.config.multisigAddr,
      },
    })

    printGreen('Owner updated to Multisig for Health Contract')

    const creditManagerConfig = await hExec.config()
    assert.equal(creditManagerConfig.owner_response.proposed, this.config.multisigAddr)
  }

  private async transferCoin(recipient: string, coin: Coin) {
    await this.cwClient.sendTokens(this.deployerAddr, recipient, [coin], 2)
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

  async instantiateAddressProvider() {
    const msg: AddressProviderInstantiateMsg = {
      owner: this.deployerAddr,
      prefix: this.config.chain.prefix,
    }
    await this.instantiate('addressProvider', this.storage.codeIds['addressProvider']!, msg)
  }

  async instantiateRedBank() {
    const msg: RedBankInstantiateMsg = {
      owner: this.deployerAddr,
      config: {
        address_provider: this.storage.addresses['addressProvider']!,
      },
    }
    await this.instantiate('redBank', this.storage.codeIds['redBank']!, msg)
  }

  async instantiateIncentives() {
    const msg: IncentivesInstantiateMsg = {
      owner: this.deployerAddr,
      address_provider: this.storage.addresses['addressProvider']!,
      epoch_duration: this.config.incentives.epochDuration,
      max_whitelisted_denoms: this.config.incentives.maxWhitelistedIncentiveDenoms,
    }
    await this.instantiate('incentives', this.storage.codeIds.incentives!, msg)
  }

  async instantiateOracle(init_params?: WasmOracleCustomInitParams) {
    const msg: OracleInstantiateMsg = {
      owner: this.deployerAddr,
      base_denom: this.config.oracle.baseDenom,
      custom_init: init_params,
    }
    await this.instantiate('oracle', this.storage.codeIds.oracle!, msg)
  }

  async instantiateRewards() {
    const msg: RewardsInstantiateMsg = {
      owner: this.deployerAddr,
      address_provider: this.storage.addresses['addressProvider']!,
      safety_tax_rate: this.config.rewardsCollector.safetyFundFeeShare,
      safety_fund_denom: this.config.rewardsCollector.safetyFundDenom,
      fee_collector_denom: this.config.rewardsCollector.feeCollectorDenom,
      channel_id: this.config.rewardsCollector.channelId,
      timeout_seconds: this.config.rewardsCollector.timeoutSeconds,
      slippage_tolerance: this.config.rewardsCollector.slippageTolerance,
      neutron_ibc_config: this.config.rewardsCollector.neutronIbcConfig,
    }
    await this.instantiate('rewardsCollector', this.storage.codeIds['rewardsCollector']!, msg)
  }

  async instantiateSwapper() {
    const msg: SwapperInstantiateMsg = {
      owner: this.deployerAddr,
    }

    await this.instantiate('swapper', this.storage.codeIds.swapper!, msg)
  }

  async instantiateParams() {
    const msg: ParamsInstantiateMsg = {
      owner: this.deployerAddr,
      address_provider: this.storage.addresses['addressProvider']!,
      target_health_factor: this.config.targetHealthFactor,
    }
    await this.instantiate('params', this.storage.codeIds.params!, msg)
  }

  async updateAssetParams(assetConfig: AssetConfig) {
    if (this.storage.actions.assetsSet.includes(assetConfig.denom)) {
      printBlue(`${assetConfig.symbol} already updated in Params contract`)
      return
    }
    printBlue(`Updating ${assetConfig.symbol}...`)

    const msg: ParamsExecuteMsg = {
      update_asset_params: {
        add_or_update: {
          params: {
            credit_manager: {
              hls: assetConfig.credit_manager.hls,
              whitelisted: assetConfig.credit_manager.whitelisted,
            },
            denom: assetConfig.denom,
            liquidation_bonus: assetConfig.liquidation_bonus,
            liquidation_threshold: assetConfig.liquidation_threshold,
            protocol_liquidation_fee: assetConfig.protocol_liquidation_fee,
            max_loan_to_value: assetConfig.max_loan_to_value,
            red_bank: {
              borrow_enabled: assetConfig.red_bank.borrow_enabled,
              deposit_enabled: assetConfig.red_bank.deposit_enabled,
            },
            deposit_cap: assetConfig.deposit_cap,
          },
        },
      },
    }

    await this.cwClient.execute(this.deployerAddr, this.storage.addresses['params']!, msg, 'auto')

    printYellow(`${assetConfig.symbol} updated.`)

    this.storage.actions.assetsSet.push(assetConfig.denom)
  }

  async initializeMarket(assetConfig: AssetConfig) {
    if (this.storage.actions.redBankMarketsSet.includes(assetConfig.denom)) {
      printBlue(`${assetConfig.symbol} already initialized in red-bank contract`)
      return
    }
    printBlue(`Initializing ${assetConfig.symbol}...`)

    const msg: RedBankExecuteMsg = {
      init_asset: {
        denom: assetConfig.denom,
        params: {
          reserve_factor: assetConfig.reserve_factor,
          interest_rate_model: {
            optimal_utilization_rate: assetConfig.interest_rate_model.optimal_utilization_rate,
            base: assetConfig.interest_rate_model.base,
            slope_1: assetConfig.interest_rate_model.slope_1,
            slope_2: assetConfig.interest_rate_model.slope_2,
          },
        },
      },
    }

    await this.cwClient.execute(this.deployerAddr, this.storage.addresses['redBank']!, msg, 'auto')

    printYellow(`${assetConfig.symbol} initialized`)

    this.storage.actions.redBankMarketsSet.push(assetConfig.denom)
  }

  async updateVaultConfig(vaultConfig: VaultConfig) {
    if (this.storage.actions.vaultsSet.includes(vaultConfig.vault.addr)) {
      printBlue(`${vaultConfig.symbol} already updated in Params contract`)
      return
    }
    printBlue(`Updating ${vaultConfig.symbol}...`)

    const msg: ParamsExecuteMsg = {
      update_vault_config: {
        add_or_update: {
          config: vaultConfig.vault,
        },
      },
    }

    await this.cwClient.execute(this.deployerAddr, this.storage.addresses['params']!, msg, 'auto')

    printYellow(`${vaultConfig.symbol} updated.`)

    this.storage.actions.vaultsSet.push(vaultConfig.vault.addr)
  }

  async updateSwapperAstroportConfig(config: AstroportConfig) {
    printBlue(`Updating swapper astroport config...`)

    const swapperConfig: SwapperAstroportConfig = {
      router: config.router,
      factory: config.factory,
      oracle: this.storage.addresses.oracle!,
    }

    await this.cwClient.execute(
      this.deployerAddr,
      this.storage.addresses.swapper!,
      { update_config: { config: swapperConfig } },
      'auto',
    )

    printYellow(`Swapper astroport config updated.`)
  }

  async setAstroportIncentivesAddress(addr: string) {
    printBlue(`Updating address provider with astroport incentives...`)

    const address: AddressResponseItem = {
      address: addr,
      address_type: 'astroport_incentives',
    }

    await this.cwClient.execute(
      this.deployerAddr,
      this.storage.addresses.addressProvider!,
      { set_address: address },
      'auto',
    )

    printYellow(`Address provider updated.`)
  }

  async setRoutes() {
    printBlue('Setting Swapper Routes')
    for (const route of this.config.swapper.routes) {
      const routeKey = `${route.denom_in} -> ${route.denom_out}`

      if (this.storage.actions.routesSet.includes(routeKey)) {
        printBlue(`${routeKey} already set in Swapper contract`)
        continue
      }

      printBlue(`Setting route: ${routeKey}`)

      if (isAstroportRoute(route.route)) {
        route.route.oracle = this.storage.addresses.oracle!
      }

      await this.cwClient.execute(
        this.deployerAddr,
        this.storage.addresses.swapper!,
        {
          set_route: route,
        } satisfies SwapperExecuteMsg,
        'auto',
      )

      this.storage.actions.routesSet.push(routeKey)
    }

    printYellow(`${this.config.chain.id} :: Swapper Routes have been set`)
  }

  async updateAddressProvider() {
    printBlue('Updating addresses in Address Provider...')
    const addressesToSet: AddressResponseItem[] = [
      {
        address: this.storage.addresses['rewardsCollector']!,
        address_type: 'rewards_collector',
      },
      {
        address: this.storage.addresses.incentives!,
        address_type: 'incentives',
      },
      {
        address: this.storage.addresses.oracle!,
        address_type: 'oracle',
      },
      {
        address: this.storage.addresses['redBank']!,
        address_type: 'red_bank',
      },
      {
        address: this.config.feeCollectorAddr,
        address_type: 'fee_collector',
      },
      {
        address: this.config.safetyFundAddr,
        address_type: 'safety_fund',
      },
      {
        address: this.config.protocolAdminAddr,
        address_type: 'protocol_admin',
      },
      {
        address: this.storage.addresses.swapper!,
        address_type: 'swapper',
      },
      {
        address: this.storage.addresses.params!,
        address_type: 'params',
      },
      {
        address: this.storage.addresses.creditManager!,
        address_type: 'credit_manager',
      },
    ]

    for (const addrObj of addressesToSet) {
      if (this.storage.actions.addressProviderSet[addrObj.address_type]) {
        printBlue(`Address already updated for ${addrObj.address_type}.`)
        continue
      }
      printBlue(`Setting ${addrObj.address_type} to ${addrObj.address}`)
      await this.cwClient.execute(
        this.deployerAddr,
        this.storage.addresses['addressProvider']!,
        { set_address: addrObj },
        'auto',
      )
      this.storage.actions.addressProviderSet[addrObj.address_type] = true
    }
    printGreen('Address Provider update completed')
  }

  async recordTwapSnapshots(denoms: string[]) {
    const msg: WasmOracleExecuteMsg = {
      custom: {
        record_twap_snapshots: {
          denoms,
        },
      },
    }

    await this.cwClient.execute(this.deployerAddr, this.storage.addresses.oracle!, msg, 'auto')

    printYellow(`Twap snapshots recorded for denoms: ${denoms.join(',')}.`)
  }

  async setOracle(oracleConfig: OracleConfig) {
    if (this.storage.actions.oraclePricesSet.includes(oracleConfig.denom)) {
      printBlue(`${oracleConfig.denom} already set in Oracle contract`)
      return
    }

    printBlue(`Setting oracle price source: ${JSON.stringify(oracleConfig)}`)

    const msg = {
      set_price_source: oracleConfig,
    }
    await this.cwClient.execute(this.deployerAddr, this.storage.addresses.oracle!, msg, 'auto')

    printYellow('Oracle Price is set.')

    this.storage.actions.oraclePricesSet.push(oracleConfig.denom)

    // try {
    //   const oracleResult = (await this.cwClient.queryContractSmart(this.storage.addresses.oracle!, {
    //     price: { denom: oracleConfig.denom },
    //   })) as { price: number; denom: string }

    //   printGreen(
    //     `${this.config.chain.id} :: ${oracleConfig.denom} oracle price:  ${JSON.stringify(
    //       oracleResult,
    //     )}`,
    //   )
    // } catch (e) {
    //   // Querying astroport TWAP can fail if enough TWAP snapshots have not been recorded yet
    //   if (!Object.keys(oracleConfig.price_source).includes('astroport_twap')) {
    //     throw e
    //   }
    // }
  }

  async executeDeposit() {
    const msg = { deposit: {} }
    const coins = [
      {
        denom: this.config.atomDenom,
        amount: '1000000',
      },
    ]

    await this.cwClient.execute(
      this.deployerAddr,
      this.storage.addresses['redBank']!,
      msg,
      'auto',
      undefined,
      coins,
    )
    printYellow('Deposit Executed.')

    printYellow('Querying user position:')
    const msgTwo: RedBankQueryMsg = { user_position: { user: this.deployerAddr } }
    console.log(await this.cwClient.queryContractSmart(this.storage.addresses['redBank']!, msgTwo))
  }

  async executeBorrow() {
    const msg = {
      borrow: {
        denom: this.config.atomDenom,
        amount: '300000',
      },
    }

    await this.cwClient.execute(this.deployerAddr, this.storage.addresses['redBank']!, msg, 'auto')
    printYellow('Borrow executed:')

    const msgTwo = { user_position: { user: this.deployerAddr } }
    console.log(await this.cwClient.queryContractSmart(this.storage.addresses['redBank']!, msgTwo))
  }

  async executeRepay() {
    const msg = { repay: {} }
    const coins = [
      {
        denom: this.config.atomDenom,
        amount: '300005',
      },
    ]

    await this.cwClient.execute(
      this.deployerAddr,
      this.storage.addresses['redBank']!,
      msg,
      'auto',
      undefined,
      coins,
    )
    printYellow('Repay executed:')

    const msgTwo = { user_position: { user: this.deployerAddr } }
    console.log(await this.cwClient.queryContractSmart(this.storage.addresses['redBank']!, msgTwo))
  }

  async executeWithdraw() {
    const msg = {
      withdraw: {
        denom: this.config.atomDenom,
        amount: '1000000',
      },
    }

    await this.cwClient.execute(this.deployerAddr, this.storage.addresses['redBank']!, msg, 'auto')
    printYellow('Withdraw executed:')

    const msgTwo = { user_position: { user: this.deployerAddr } }
    console.log(await this.cwClient.queryContractSmart(this.storage.addresses['redBank']!, msgTwo))
  }

  async executeRewardsSwap() {
    // Send some coins to the contract
    const coins = [
      {
        denom: this.config.atomDenom,
        amount: '20000',
      },
    ]

    const deployerAtomBalance = await this.cwClient.getBalance(
      this.deployerAddr,
      this.config.atomDenom,
    )

    if (Number(deployerAtomBalance.amount) < Number(coins[0].amount)) {
      printRed(
        `not enough ATOM tokens to complete rewards-collector swap action, ${this.deployerAddr} has ${deployerAtomBalance.amount} ATOM but needs ${coins[0].amount}.`,
      )
    } else {
      await this.cwClient.sendTokens(
        this.deployerAddr,
        this.storage.addresses['rewardsCollector']!,
        coins,
        'auto',
      )

      // Check contract balance before swap
      const atomBalanceBefore = await this.cwClient.getBalance(
        this.storage.addresses['rewardsCollector']!,
        this.config.atomDenom,
      )
      const baseAssetBalanceBefore = await this.cwClient.getBalance(
        this.storage.addresses['rewardsCollector']!,
        this.config.chain.baseDenom,
      )
      printYellow(
        `Rewards Collector balance:
        ${atomBalanceBefore.amount} ${atomBalanceBefore.denom}
        ${baseAssetBalanceBefore.amount} ${baseAssetBalanceBefore.denom}`,
      )

      // Execute swap
      const msg = {
        swap_asset: {
          denom: this.config.atomDenom,
        },
      }
      await this.cwClient.execute(
        this.deployerAddr,
        this.storage.addresses['rewardsCollector']!,
        msg,
        'auto',
      )
      // Check contract balance after swap
      const atomBalanceAfter = await this.cwClient.getBalance(
        this.storage.addresses['rewardsCollector']!,
        this.config.atomDenom,
      )
      const baseAssetBalanceAfter = await this.cwClient.getBalance(
        this.storage.addresses['rewardsCollector']!,
        this.config.chain.baseDenom,
      )
      printYellow(
        `Swap executed. Rewards Collector balance:
        ${atomBalanceAfter.amount} ${atomBalanceAfter.denom},
        ${baseAssetBalanceAfter.amount} ${baseAssetBalanceAfter.denom}`,
      )

      // swapped all atom balance
      assert.equal(Number(atomBalanceAfter.amount), 0)
      // base asset balance should be greater after swap
      assert(Number(baseAssetBalanceAfter.amount) > Number(baseAssetBalanceBefore.amount))
    }
  }

  async updateIncentivesContractOwner() {
    if (!this.config.multisigAddr) throw new Error('No multisig addresses to transfer ownership to')

    const msg = {
      update_owner: {
        propose_new_owner: {
          proposed: this.config.multisigAddr,
        },
      },
    }
    await this.cwClient.execute(this.deployerAddr, this.storage.addresses.incentives!, msg, 'auto')
    printYellow('Owner updated to Multisig for Incentives')
    const incentivesConfig = (await this.cwClient.queryContractSmart(
      this.storage.addresses.incentives!,
      {
        config: {},
      },
    )) as { proposed_new_owner: string }

    printRed(`${incentivesConfig.proposed_new_owner}`)
    assert.equal(incentivesConfig.proposed_new_owner, this.config.multisigAddr)
  }

  async updateRedBankContractOwner() {
    if (!this.config.multisigAddr) throw new Error('No multisig addresses to transfer ownership to')

    const msg = {
      update_owner: {
        propose_new_owner: {
          proposed: this.config.multisigAddr,
        },
      },
    }
    await this.cwClient.execute(this.deployerAddr, this.storage.addresses['redBank']!, msg, 'auto')
    printYellow('Owner updated to Multisig for Red Bank')
    const redbankConfig = (await this.cwClient.queryContractSmart(
      this.storage.addresses['redBank']!,
      {
        config: {},
      },
    )) as { proposed_new_owner: string }

    assert.equal(redbankConfig.proposed_new_owner, this.config.multisigAddr)
  }

  async updateOracleContractOwner() {
    if (!this.config.multisigAddr) throw new Error('No multisig addresses to transfer ownership to')

    const msg = {
      update_owner: {
        propose_new_owner: {
          proposed: this.config.multisigAddr,
        },
      },
    }
    await this.cwClient.execute(this.deployerAddr, this.storage.addresses.oracle!, msg, 'auto')
    printYellow('Owner updated to Multisig for Oracle')
    const oracleConfig = (await this.cwClient.queryContractSmart(this.storage.addresses.oracle!, {
      config: {},
    })) as { proposed_new_owner: string }

    assert.equal(oracleConfig.proposed_new_owner, this.config.multisigAddr)
  }

  async updateRewardsContractOwner() {
    if (!this.config.multisigAddr) throw new Error('No multisig addresses to transfer ownership to')

    const msg = {
      update_owner: {
        propose_new_owner: {
          proposed: this.config.multisigAddr,
        },
      },
    }
    await this.cwClient.execute(
      this.deployerAddr,
      this.storage.addresses['rewardsCollector']!,
      msg,
      'auto',
    )
    printYellow('Owner updated to Multisig for Rewards Collector')
    const rewardsConfig = (await this.cwClient.queryContractSmart(
      this.storage.addresses['rewardsCollector']!,
      {
        config: {},
      },
    )) as { proposed_new_owner: string }

    assert.equal(rewardsConfig.proposed_new_owner, this.config.multisigAddr)
  }

  async updateSwapperContractOwner() {
    if (!this.config.multisigAddr) throw new Error('No multisig addresses to transfer ownership to')

    const msg = {
      update_owner: {
        propose_new_owner: {
          proposed: this.config.multisigAddr,
        },
      },
    }
    await this.cwClient.execute(this.deployerAddr, this.storage.addresses.swapper!, msg, 'auto')
    printYellow('Owner updated to Multisig for Swapper')
    const swapperConfig = (await this.cwClient.queryContractSmart(this.storage.addresses.swapper!, {
      owner: {},
    })) as { proposed: string }

    assert.equal(swapperConfig.proposed, this.config.multisigAddr)
  }

  async updateParamsContractOwner() {
    if (!this.config.multisigAddr) throw new Error('No multisig addresses to transfer ownership to')

    const msg = {
      update_owner: {
        propose_new_owner: {
          proposed: this.config.multisigAddr,
        },
      },
    }
    await this.cwClient.execute(this.deployerAddr, this.storage.addresses.params!, msg, 'auto')
    printYellow('Owner updated to Multisig for Params')
    const paramsConfig = (await this.cwClient.queryContractSmart(this.storage.addresses.params!, {
      owner: {},
    })) as { proposed: string }

    assert.equal(paramsConfig.proposed, this.config.multisigAddr)
  }

  async updateAddressProviderContractOwner() {
    if (!this.config.multisigAddr) throw new Error('No multisig addresses to transfer ownership to')

    const msg = {
      update_owner: {
        propose_new_owner: {
          proposed: this.config.multisigAddr,
        },
      },
    }
    await this.cwClient.execute(
      this.deployerAddr,
      this.storage.addresses['addressProvider']!,
      msg,
      'auto',
    )
    printYellow('Owner updated to Multisig for Rewards Collector')
    const addressProviderConfig = (await this.cwClient.queryContractSmart(
      this.storage.addresses['addressProvider']!,
      {
        config: {},
      },
    )) as { proposed_new_owner: string }

    assert.equal(addressProviderConfig.proposed_new_owner, this.config.multisigAddr)
  }
}
