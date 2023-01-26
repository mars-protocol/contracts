import { AssetConfig, DeploymentConfig, OracleConfig } from '../../types/config'
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import * as fs from 'fs'
import { printBlue, printGreen, printRed, printYellow } from '../../utils/chalk'
import { ARTIFACTS_PATH, Storage } from './storage'
import { InstantiateMsgs } from '../../types/msg'
import { writeFile } from 'fs/promises'
import { join, resolve } from 'path'
import assert from 'assert'
import { ExecuteMsg } from '../../types/generated/mars-rewards-collector-osmosis/MarsRewardsCollectorOsmosis.types'

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

  async setOwnerAddr() {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    printGreen(`Owner is set to: ${this.storage.owner}`)
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
    const msg = {
      owner: this.deployerAddress,
      prefix: this.config.chainPrefix,
    }
    await this.instantiate('address-provider', this.storage.codeIds['address-provider']!, msg)
  }

  async instantiateRedBank() {
    const msg = {
      owner: this.deployerAddress,
      emergency_owner: this.storage.owner!,
      config: {
        address_provider: this.storage.addresses['address-provider']!,
        close_factor: '0.5',
      },
    }
    await this.instantiate('red-bank', this.storage.codeIds['red-bank']!, msg)
  }

  async instantiateIncentives() {
    const msg = {
      owner: this.deployerAddress,
      address_provider: this.storage.addresses['address-provider']!,
      mars_denom: this.config.marsDenom,
    }
    await this.instantiate('incentives', this.storage.codeIds.incentives!, msg)
  }

  async instantiateOracle() {
    const msg = {
      owner: this.deployerAddress,
      base_denom: this.config.baseAssetDenom,
    }
    await this.instantiate('oracle', this.storage.codeIds.oracle!, msg)
  }

  async instantiateRewards() {
    const msg = {
      owner: this.deployerAddress,
      address_provider: this.storage.addresses['address-provider']!,
      safety_tax_rate: this.config.safetyFundFeeShare,
      safety_fund_denom: this.config.baseAssetDenom,
      fee_collector_denom: this.config.baseAssetDenom,
      channel_id: this.config.channelId,
      timeout_revision: this.config.timeoutRevision,
      timeout_blocks: this.config.rewardCollectorTimeoutBlocks,
      timeout_seconds: this.config.rewardCollectorTimeoutSeconds,
      slippage_tolerance: this.config.slippage_tolerance,
    }
    await this.instantiate('rewards-collector', this.storage.codeIds['rewards-collector']!, msg)
  }

  async setRoutes() {
    for (const route of this.config.swapRoutes!) {
      await this.client.execute(
        this.deployerAddress,
        this.storage.addresses['rewards-collector']!,
        {
          set_route: route,
        } satisfies ExecuteMsg,
        'auto',
      )
    }

    printYellow(`${this.config.chainId} :: Rewards Collector Routes have been set`)
  }

  async saveDeploymentAddrsToFile() {
    const addressesDir = resolve(join(__dirname, '../../../deploy/addresses'))
    await writeFile(
      `${addressesDir}/${this.config.chainId}.json`,
      JSON.stringify(this.storage.addresses),
    )
  }

  async updateAddressProvider() {
    if (this.storage.execute['address-provider-updated']) {
      printBlue('Addresses already updated.')
      return
    }
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
        address_type: 'protocol_admin',
        address: this.storage.owner!,
      },
      {
        address_type: 'red_bank',
        address: this.storage.addresses['red-bank'],
      },
    ]

    for (const addrObj of addressesToSet) {
      await this.client.execute(
        this.deployerAddress,
        this.storage.addresses['address-provider']!,
        { set_address: addrObj },
        'auto',
      )
    }
    printYellow('Address Provider update completed')
    this.storage.execute['address-provider-updated'] = true
  }

  async initializeAsset(assetConfig: AssetConfig) {
    if (this.storage.execute.assetsInitialized.includes(assetConfig.denom)) {
      printBlue(`${assetConfig.symbol} already initialized.`)
      return
    }

    const msg = {
      init_asset: {
        denom: assetConfig.denom,
        params: {
          max_loan_to_value: assetConfig.max_loan_to_value,
          reserve_factor: assetConfig.reserve_factor,
          liquidation_threshold: assetConfig.liquidation_threshold,
          liquidation_bonus: assetConfig.liquidation_bonus,
          interest_rate_model: {
            optimal_utilization_rate: assetConfig.interest_rate_model.optimal_utilization_rate,
            base: assetConfig.interest_rate_model.base,
            slope_1: assetConfig.interest_rate_model.slope_1,
            slope_2: assetConfig.interest_rate_model.slope_2,
          },
          deposit_cap: assetConfig.deposit_cap,
          deposit_enabled: assetConfig.deposit_enabled,
          borrow_enabled: assetConfig.borrow_enabled,
        },
      },
    }

    await this.client.execute(
      this.deployerAddress,
      this.storage.addresses['red-bank']!,
      msg,
      'auto',
    )

    printYellow(`${assetConfig.symbol} initialized`)

    this.storage.execute.assetsInitialized.push(assetConfig.denom)
  }

  async setOracle(oracleConfig: OracleConfig) {
    if (oracleConfig.price) {
      const msg = {
        set_price_source: {
          denom: oracleConfig.denom,
          price_source: {
            fixed: { price: oracleConfig.price },
          },
        },
      }
      await this.client.execute(this.deployerAddress, this.storage.addresses.oracle!, msg, 'auto')
    } else {
      const msg = {
        set_price_source: {
          denom: oracleConfig.denom,
          price_source: {
            geometric_twap: {
              pool_id: oracleConfig.pool_id,
              window_size: oracleConfig.window_size,
              downtime_detector: oracleConfig.downtime_detector,
            },
          },
        },
      }
      // see if we need fixed price for osmo - remove fixed price
      await this.client.execute(this.deployerAddress, this.storage.addresses.oracle!, msg, 'auto')
    }

    printYellow('Oracle Price is set.')

    this.storage.execute.oraclePriceSet = true

    const oracleResult = (await this.client.queryContractSmart(this.storage.addresses.oracle!, {
      price: { denom: oracleConfig.denom },
    })) as { price: number; denom: string }

    printGreen(
      `${this.config.chainId} :: ${oracleConfig.denom} oracle price :  ${JSON.stringify(
        oracleResult,
      )}`,
    )
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
    printYellow('Deposit Executed:')

    const msgTwo = { user_position: { user: this.deployerAddress } }
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
