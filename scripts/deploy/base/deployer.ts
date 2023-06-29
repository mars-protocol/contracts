import {
  AssetConfig,
  DeploymentConfig,
  OracleConfig,
  isAstroportRoute,
  VaultConfig,
} from '../../types/config'
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import * as fs from 'fs'
import { printBlue, printGreen, printRed, printYellow } from '../../utils/chalk'
import { ARTIFACTS_PATH, Storage } from './storage'
import { InstantiateMsgs } from '../../types/msg'
import { writeFile } from 'fs/promises'
import { join, resolve } from 'path'
import assert from 'assert'

import { SwapperExecuteMsg } from '../../types/config'
import { InstantiateMsg as AstroportSwapperInstantiateMsg } from '../../types/generated/mars-swapper-astroport/MarsSwapperAstroport.types'
import { InstantiateMsg as OsmosisSwapperInstantiateMsg } from '../../types/generated/mars-swapper-osmosis/MarsSwapperOsmosis.types'
import { InstantiateMsg as ParamsInstantiateMsg } from '../../types/generated/mars-params/MarsParams.types'
import { ExecuteMsg as ParamsExecuteMsg } from '../../types/generated/mars-params/MarsParams.types'
import {
  InstantiateMsg as RedBankInstantiateMsg,
  QueryMsg as RedBankQueryMsg,
} from '../../types/generated/mars-red-bank/MarsRedBank.types'
import { InstantiateMsg as AddressProviderInstantiateMsg } from '../../types/generated/mars-address-provider/MarsAddressProvider.types'
import { InstantiateMsg as IncentivesInstantiateMsg } from '../../types/generated/mars-incentives/MarsIncentives.types'
import { InstantiateMsg as RewardsInstantiateMsg } from '../../types/generated/mars-rewards-collector/MarsRewardsCollector.types'
import {
  WasmOracleCustomInitParams,
  InstantiateMsg as WasmOracleInstantiateMsg,
} from '../../types/generated/mars-oracle-wasm/MarsOracleWasm.types'
import { InstantiateMsg as OsmosisOracleInstantiateMsg } from '../../types/generated/mars-oracle-osmosis/MarsOracleOsmosis.types'
import { ExecuteMsg as WasmOracleExecuteMsg } from '../../types/generated/mars-oracle-wasm/MarsOracleWasm.types'
import { StorageItems } from '../../types/storageItems'

type SwapperInstantiateMsg = AstroportSwapperInstantiateMsg | OsmosisSwapperInstantiateMsg
type OracleInstantiateMsg = WasmOracleInstantiateMsg | OsmosisOracleInstantiateMsg

export class Deployer {
  constructor(
    public config: DeploymentConfig,
    public client: SigningCosmWasmClient,
    public deployerAddress: string,
    private storage: Storage,
  ) {}

  async saveStorage() {
    await this.storage.save()
  }

  async assertDeployerBalance() {
    const accountBalance = await this.client.getBalance(
      this.deployerAddress,
      this.config.baseAssetDenom,
    )
    printYellow(
      `${this.config.baseAssetDenom} account balance is: ${accountBalance.amount} (${
        Number(accountBalance.amount) / 1e6
      } ${this.config.chainPrefix})`,
    )
    if (Number(accountBalance.amount) < 1_000_000 && this.config.chainId === 'osmo-test-4') {
      printRed(
        `not enough ${this.config.chainPrefix} tokens to complete action, you may need to go to a test faucet to get more tokens.`,
      )
    }
  }

  async upload(name: keyof Storage['codeIds'], file: string) {
    if (this.storage.codeIds[name]) {
      printBlue(`Wasm already uploaded :: ${name} :: ${this.storage.codeIds[name]}`)
      return
    }

    const wasm = fs.readFileSync(ARTIFACTS_PATH + file)
    const uploadResult = await this.client.upload(this.deployerAddress, wasm, 'auto')
    this.storage.codeIds[name] = uploadResult.codeId
    printGreen(`${this.config.chainId} :: ${name} : ${this.storage.codeIds[name]}`)
  }

  setOwnerAddr() {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    printGreen(`Owner is set to: ${this.storage.owner}`)
  }

  setAddress(name: keyof StorageItems['addresses'], address: string) {
    this.storage.addresses[name] = address
    printGreen(`Address of ${name} is set to: ${this.storage.addresses[name]}`)
  }

  async instantiate(name: keyof Storage['addresses'], codeId: number, msg: InstantiateMsgs) {
    if (this.storage.addresses[name]) {
      printBlue(`Contract already instantiated :: ${name} :: ${this.storage.addresses[name]}`)
      return
    }

    const { contractAddress: redBankContractAddress } = await this.client.instantiate(
      this.deployerAddress,
      codeId,
      msg,
      `mars-${name}`,
      'auto',
      { admin: this.storage.owner },
    )

    this.storage.addresses[name] = redBankContractAddress
    printGreen(
      `${this.config.chainId} :: ${name} Contract Address : ${this.storage.addresses[name]}`,
    )
  }

  async instantiateAddressProvider() {
    const msg: AddressProviderInstantiateMsg = {
      owner: this.deployerAddress,
      prefix: this.config.chainPrefix,
    }
    await this.instantiate('address-provider', this.storage.codeIds['address-provider']!, msg)
  }

  async instantiateRedBank() {
    const msg: RedBankInstantiateMsg = {
      owner: this.deployerAddress,
      config: {
        address_provider: this.storage.addresses['address-provider']!,
      },
    }
    await this.instantiate('red-bank', this.storage.codeIds['red-bank']!, msg)
  }

  async instantiateIncentives() {
    const msg: IncentivesInstantiateMsg = {
      owner: this.deployerAddress,
      address_provider: this.storage.addresses['address-provider']!,
      epoch_duration: this.config.incentiveEpochDuration,
      max_whitelisted_denoms: this.config.maxWhitelistedIncentiveDenoms,
    }
    await this.instantiate('incentives', this.storage.codeIds.incentives!, msg)
  }

  async instantiateOracle(init_params?: WasmOracleCustomInitParams) {
    const msg: OracleInstantiateMsg = {
      owner: this.deployerAddress,
      base_denom: this.config.baseAssetDenom,
      custom_init: init_params,
    }
    await this.instantiate('oracle', this.storage.codeIds.oracle!, msg)
  }

  async instantiateRewards() {
    const msg: RewardsInstantiateMsg = {
      owner: this.deployerAddress,
      address_provider: this.storage.addresses['address-provider']!,
      safety_tax_rate: this.config.safetyFundFeeShare,
      safety_fund_denom: this.config.safetyFundDenom,
      fee_collector_denom: this.config.feeCollectorDenom,
      channel_id: this.config.channelId,
      timeout_seconds: this.config.rewardCollectorTimeoutSeconds,
      slippage_tolerance: this.config.slippage_tolerance,
    }
    await this.instantiate('rewards-collector', this.storage.codeIds['rewards-collector']!, msg)
  }

  async instantiateSwapper() {
    const msg: SwapperInstantiateMsg = {
      owner: this.storage.owner!,
    }

    await this.instantiate('swapper', this.storage.codeIds.swapper!, msg)
  }

  async instantiateParams() {
    const msg: ParamsInstantiateMsg = {
      owner: this.deployerAddress,
      target_health_factor: this.config.targetHealthFactor,
    }
    await this.instantiate('params', this.storage.codeIds.params!, msg)
  }

  async updateAssetParams(assetConfig: AssetConfig) {
    if (this.storage.execute.assetsUpdated.includes(assetConfig.denom)) {
      printBlue(`${assetConfig.symbol} already updated in Params contract`)
      return
    }
    printBlue(`Updating ${assetConfig.symbol}...`)

    const msg = {
      update_asset_params: {
        add_or_update: {
          params: {
            credit_manager: {
              whitelisted: assetConfig.credit_manager.whitelisted,
            },
            denom: assetConfig.denom,
            liquidation_bonus: assetConfig.liquidation_bonus,
            liquidation_threshold: assetConfig.liquidation_threshold,
            protocol_liquidation_fee: assetConfig.protocol_liquidation_fee,
            max_loan_to_value: assetConfig.max_loan_to_value,
            red_bank: {
              borrow_enabled: assetConfig.red_bank.borrow_enabled,
              deposit_enabled: assetConfig.red_bank.borrow_enabled,
              deposit_cap: assetConfig.red_bank.deposit_cap,
            },
          },
        },
      },
    }

    await this.client.execute(this.deployerAddress, this.storage.addresses['params']!, msg, 'auto')

    printYellow(`${assetConfig.symbol} updated.`)
  }

  async updateVaultConfig(vaultConfig: VaultConfig) {
    if (this.storage.execute.vaultsUpdated.includes(vaultConfig.addr)) {
      printBlue(`${vaultConfig.symbol} already updated in Params contract`)
      return
    }
    printBlue(`Updating ${vaultConfig.symbol}...`)

    const msg: ParamsExecuteMsg = {
      update_vault_config: {
        add_or_update: {
          config: {
            addr: vaultConfig.addr,
            deposit_cap: vaultConfig.deposit_cap,
            liquidation_threshold: vaultConfig.liquidation_threshold,
            whitelisted: vaultConfig.whitelisted,
            max_loan_to_value: vaultConfig.max_loan_to_value,
          },
        },
      },
    }

    await this.client.execute(this.deployerAddress, this.storage.addresses['params']!, msg, 'auto')

    printYellow(`${vaultConfig.symbol} updated.`)
  }
  async setRoutes() {
    printBlue('Setting Swapper Routes')
    for (const route of this.config.swapRoutes) {
      printBlue(`Setting route: ${route.denom_in} -> ${route.denom_out}`)
      if (isAstroportRoute(route.route)) {
        route.route.oracle = this.storage.addresses.oracle!
      }

      await this.client.execute(
        this.deployerAddress,
        this.storage.addresses.swapper!,
        {
          set_route: route,
        } satisfies SwapperExecuteMsg,
        'auto',
      )
    }

    printYellow(`${this.config.chainId} :: Swapper Routes have been set`)
  }

  async saveDeploymentAddrsToFile() {
    const addressesDir = resolve(join(__dirname, '../../../deploy/addresses'))
    await writeFile(
      `${addressesDir}/${this.config.chainId}.json`,
      JSON.stringify(this.storage.addresses),
    )
  }

  async updateAddressProvider() {
    printBlue('Updating addresses in Address Provider...')
    const addressesToSet = [
      {
        address_type: 'rewards_collector',
        address: this.storage.addresses['rewards-collector'],
      },
      {
        address_type: 'incentives',
        address: this.storage.addresses.incentives,
      },
      {
        address_type: 'oracle',
        address: this.storage.addresses.oracle,
      },
      {
        address_type: 'red_bank',
        address: this.storage.addresses['red-bank'],
      },
      {
        address_type: 'fee_collector',
        address: this.config.feeCollectorAddr,
      },
      {
        address_type: 'safety_fund',
        address: this.config.safetyFundAddr,
      },
      {
        address_type: 'protocol_admin',
        address: this.config.protocolAdminAddr,
      },
      {
        address_type: 'swapper',
        address: this.storage.addresses.swapper,
      },
    ]

    for (const addrObj of addressesToSet) {
      if (this.storage.execute.addressProviderUpdated[addrObj.address_type]) {
        printBlue(`Address already updated for ${addrObj.address_type}.`)
        continue
      }
      printBlue(`Setting ${addrObj.address_type} to ${addrObj.address}`)
      await this.client.execute(
        this.deployerAddress,
        this.storage.addresses['address-provider']!,
        { set_address: addrObj },
        'auto',
      )
      this.storage.execute.addressProviderUpdated[addrObj.address_type] = true
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

    await this.client.execute(this.deployerAddress, this.storage.addresses.oracle!, msg, 'auto')

    printYellow(`Twap snapshots recorded for denoms: ${denoms.join(',')}.`)
  }
  async setOracle(oracleConfig: OracleConfig) {
    const msg = {
      set_price_source: oracleConfig,
    }
    await this.client.execute(this.deployerAddress, this.storage.addresses.oracle!, msg, 'auto')

    printYellow('Oracle Price is set.')

    this.storage.execute.oraclePriceSet = true

    try {
      const oracleResult = (await this.client.queryContractSmart(this.storage.addresses.oracle!, {
        price: { denom: oracleConfig.denom },
      })) as { price: number; denom: string }

      printGreen(
        `${this.config.chainId} :: ${oracleConfig.denom} oracle price :  ${JSON.stringify(
          oracleResult,
        )}`,
      )
    } catch (e) {
      // Querying astroport TWAP can fail if enough TWAP snapshots have not been recorded yet
      if (!Object.keys(oracleConfig.price_source).includes('astroport_twap')) {
        throw e
      }
    }
  }

  async executeDeposit() {
    const msg = { deposit: {} }
    const coins = [
      {
        denom: this.config.atomDenom,
        amount: '1000000',
      },
    ]

    await this.client.execute(
      this.deployerAddress,
      this.storage.addresses['red-bank']!,
      msg,
      'auto',
      undefined,
      coins,
    )
    printYellow('Deposit Executed.')

    printYellow('Querying user position:')
    const msgTwo: RedBankQueryMsg = { user_position: { user: this.deployerAddress } }
    console.log(await this.client.queryContractSmart(this.storage.addresses['red-bank']!, msgTwo))
  }

  async executeBorrow() {
    const msg = {
      borrow: {
        denom: this.config.atomDenom,
        amount: '300000',
      },
    }

    await this.client.execute(
      this.deployerAddress,
      this.storage.addresses['red-bank']!,
      msg,
      'auto',
    )
    printYellow('Borrow executed:')

    const msgTwo = { user_position: { user: this.deployerAddress } }
    console.log(await this.client.queryContractSmart(this.storage.addresses['red-bank']!, msgTwo))
  }

  async executeRepay() {
    const msg = { repay: {} }
    const coins = [
      {
        denom: this.config.atomDenom,
        amount: '300005',
      },
    ]

    await this.client.execute(
      this.deployerAddress,
      this.storage.addresses['red-bank']!,
      msg,
      'auto',
      undefined,
      coins,
    )
    printYellow('Repay executed:')

    const msgTwo = { user_position: { user: this.deployerAddress } }
    console.log(await this.client.queryContractSmart(this.storage.addresses['red-bank']!, msgTwo))
  }

  async executeWithdraw() {
    const msg = {
      withdraw: {
        denom: this.config.atomDenom,
        amount: '1000000',
      },
    }

    await this.client.execute(
      this.deployerAddress,
      this.storage.addresses['red-bank']!,
      msg,
      'auto',
    )
    printYellow('Withdraw executed:')

    const msgTwo = { user_position: { user: this.deployerAddress } }
    console.log(await this.client.queryContractSmart(this.storage.addresses['red-bank']!, msgTwo))
  }

  async executeRewardsSwap() {
    // Send some coins to the contract
    const coins = [
      {
        denom: this.config.atomDenom,
        amount: '2000000',
      },
    ]
    await this.client.sendTokens(
      this.deployerAddress,
      this.storage.addresses['rewards-collector']!,
      coins,
      'auto',
    )

    // Check contract balance before swap
    const atomBalanceBefore = await this.client.getBalance(
      this.storage.addresses['rewards-collector']!,
      this.config.atomDenom,
    )
    const baseAssetBalanceBefore = await this.client.getBalance(
      this.storage.addresses['rewards-collector']!,
      this.config.baseAssetDenom,
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
    await this.client.execute(
      this.deployerAddress,
      this.storage.addresses['rewards-collector']!,
      msg,
      'auto',
    )
    // Check contract balance after swap
    const atomBalanceAfter = await this.client.getBalance(
      this.storage.addresses['rewards-collector']!,
      this.config.atomDenom,
    )
    const baseAssetBalanceAfter = await this.client.getBalance(
      this.storage.addresses['rewards-collector']!,
      this.config.baseAssetDenom,
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

  async updateIncentivesContractOwner() {
    const msg = {
      update_owner: {
        propose_new_owner: {
          proposed: this.storage.owner,
        },
      },
    }
    await this.client.execute(this.deployerAddress, this.storage.addresses.incentives!, msg, 'auto')
    printYellow('Owner updated to Mutlisig for Incentives')
    const incentivesConfig = (await this.client.queryContractSmart(
      this.storage.addresses.incentives!,
      {
        config: {},
      },
    )) as { proposed_new_owner: string; prefix: string }

    printRed(`${incentivesConfig.proposed_new_owner}`)
    assert.equal(incentivesConfig.proposed_new_owner, this.config.multisigAddr)
  }

  async updateRedBankContractOwner() {
    const msg = {
      update_owner: {
        propose_new_owner: {
          proposed: this.storage.owner,
        },
      },
    }
    await this.client.execute(
      this.deployerAddress,
      this.storage.addresses['red-bank']!,
      msg,
      'auto',
    )
    printYellow('Owner updated to Mutlisig for Red Bank')
    const redbankConfig = (await this.client.queryContractSmart(
      this.storage.addresses['red-bank']!,
      {
        config: {},
      },
    )) as { proposed_new_owner: string; prefix: string }

    assert.equal(redbankConfig.proposed_new_owner, this.config.multisigAddr)
  }

  async updateOracleContractOwner() {
    const msg = {
      update_owner: {
        propose_new_owner: {
          proposed: this.storage.owner,
        },
      },
    }
    await this.client.execute(this.deployerAddress, this.storage.addresses.oracle!, msg, 'auto')
    printYellow('Owner updated to Mutlisig for Oracle')
    const oracleConfig = (await this.client.queryContractSmart(this.storage.addresses.oracle!, {
      config: {},
    })) as { proposed_new_owner: string; prefix: string }

    assert.equal(oracleConfig.proposed_new_owner, this.config.multisigAddr)
  }

  async updateRewardsContractOwner() {
    const msg = {
      update_owner: {
        propose_new_owner: {
          proposed: this.storage.owner,
        },
      },
    }
    await this.client.execute(
      this.deployerAddress,
      this.storage.addresses['rewards-collector']!,
      msg,
      'auto',
    )
    printYellow('Owner updated to Mutlisig for Rewards Collector')
    const rewardsConfig = (await this.client.queryContractSmart(
      this.storage.addresses['rewards-collector']!,
      {
        config: {},
      },
    )) as { proposed_new_owner: string; prefix: string }

    assert.equal(rewardsConfig.proposed_new_owner, this.config.multisigAddr)
  }

  async updateAddressProviderContractOwner() {
    const msg = {
      update_owner: {
        propose_new_owner: {
          proposed: this.storage.owner,
        },
      },
    }
    await this.client.execute(
      this.deployerAddress,
      this.storage.addresses['address-provider']!,
      msg,
      'auto',
    )
    printYellow('Owner updated to Mutlisig for Rewards Collector')
    const addressProviderConfig = (await this.client.queryContractSmart(
      this.storage.addresses['address-provider']!,
      {
        config: {},
      },
    )) as { proposed_new_owner: string; prefix: string }

    assert.equal(addressProviderConfig.proposed_new_owner, this.config.multisigAddr)
  }
}
