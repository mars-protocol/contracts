import { AssetConfig, DeploymentConfig, OracleConfig } from '../../types/config'
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import * as fs from 'fs'
import { printBlue, printGreen, printRed, printYellow } from '../../utils/chalk'
import { ARTIFACTS_PATH, Storage } from './storage'
import { InstantiateMsgs } from '../../types/msg'
import { writeFile } from 'fs/promises'
import { join, resolve } from 'path'
import assert from 'assert'

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

  async instantiate(name: keyof Storage['addresses'], codeId: number, msg: InstantiateMsgs) {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    if (this.storage.addresses[name]) {
      printBlue(`Contract already instantiated :: ${name} :: ${this.storage.addresses[name]}`)
      return
    }

    const { contractAddress: redBankContractAddress } = await this.client.instantiate(
      this.deployerAddress,
      codeId,
      // @ts-expect-error msg expecting too general of a type
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
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    const msg = {
      owner: this.storage.owner,
      prefix: this.config.chainPrefix,
    }
    await this.instantiate('addressProvider', this.storage.codeIds.addressProvider!, msg)
  }

  async instantiateRedBank() {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    const msg = {
      config: {
        owner: this.storage.owner,
        address_provider: this.storage.addresses.addressProvider!,
        close_factor: '0.5',
      },
    }
    await this.instantiate('redBank', this.storage.codeIds.redBank!, msg)
  }

  async instantiateIncentives() {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    const msg = {
      owner: this.storage.owner,
      address_provider: this.storage.addresses.addressProvider!,
      mars_denom: this.config.marsDenom,
    }
    await this.instantiate('incentives', this.storage.codeIds.incentives!, msg)
  }

  async instantiateOracle() {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    const msg = {
      owner: this.storage.owner,
      base_denom: this.config.baseAssetDenom,
    }
    await this.instantiate('oracle', this.storage.codeIds.oracle!, msg)
  }

  async instantiateRewards() {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    const msg = {
      owner: this.storage.owner,
      address_provider: this.storage.addresses.addressProvider!,
      safety_tax_rate: this.config.safetyFundFeeShare,
      safety_fund_denom: this.config.baseAssetDenom,
      fee_collector_denom: this.config.baseAssetDenom,
      channel_id: this.config.channelId,
      timeout_revision: this.config.timeoutRevision,
      timeout_blocks: this.config.rewardCollectorTimeoutBlocks,
      timeout_seconds: this.config.rewardCollectorTimeoutSeconds,
      slippage_tolerance: this.config.slippage_tolerance,
    }
    await this.instantiate('rewardsCollector', this.storage.codeIds.rewardsCollector!, msg)

    await this.client.execute(
      this.deployerAddress,
      this.storage.addresses.rewardsCollector!,
      {
        set_route: {
          denom_in: this.config.baseAssetDenom,
          denom_out: this.config.atomDenom,
          route: [{ token_out_denom: this.config.atomDenom, pool_id: 1 }],
        },
      },
      'auto',
    )

    printGreen(
      `${this.config.chainId} :: Rewards Collector Contract Address : ${this.storage.addresses.rewardsCollector}`,
    )
  }

  async saveDeploymentAddrsToFile() {
    const addressesDir = resolve(join(__dirname, '../../../deploy/addresses'))
    await writeFile(
      `${addressesDir}/${this.config.chainId}.json`,
      JSON.stringify(this.storage.addresses),
    )
  }

  async updateAddressProvider() {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    if (this.storage.execute.addressProviderUpdated) {
      printBlue('Addresses already updated.')
      return
    }
    const addressesToSet = [
      {
        contract: 'rewards_collector',
        address: this.storage.addresses.rewardsCollector,
      },
      {
        contract: 'incentives',
        address: this.storage.addresses.incentives,
      },
      {
        contract: 'oracle',
        address: this.storage.addresses.oracle,
      },
      {
        contract: 'protocol_admin',
        address: this.storage.owner,
      },
      {
        contract: 'red_bank',
        address: this.storage.addresses.redBank,
      },
    ]
    // When executeMultiple is released to npm, switch to that
    for (const addrObj of addressesToSet) {
      await this.client.execute(
        this.deployerAddress,
        this.storage.addresses.addressProvider!,
        { set_address: addrObj },
        'auto',
      )
    }
    printYellow('Address Provider update completed')
    this.storage.execute.addressProviderUpdated = true
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
          initial_borrow_rate: assetConfig.initial_borrow_rate,
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

    await this.client.execute(this.deployerAddress, this.storage.addresses.redBank!, msg, 'auto')

    printYellow(`${assetConfig.symbol} initialized`)

    this.storage.execute.assetsInitialized.push(assetConfig.denom)
  }

  async setOraclePrice(oracleConfig: OracleConfig) {
    if (this.storage.execute.oraclePriceSet) {
      printBlue(`${this.config.second_asset_symbol} Oracle Price already set`)
      return
    }

    const msg = {
      set_price_source: {
        denom: oracleConfig.denom,
        price_source: {
          fixed: { price: oracleConfig.price },
        },
      },
    }

    await this.client.execute(this.deployerAddress, this.storage.addresses.oracle!, msg, 'auto')

    printYellow('Oracle Price is set.')

    this.storage.execute.oraclePriceSet = true

    const oracleResult = (await this.client.queryContractSmart(this.storage.addresses.oracle!, {
      price: { denom: this.config.atomDenom },
    })) as { price: number; denom: string }

    console.log(`${this.config.chainId} :: uosmo oracle price :  ${JSON.stringify(oracleResult)}`)
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
      this.storage.addresses.redBank!,
      msg,
      'auto',
      undefined,
      coins,
    )
    printYellow('Deposit Executed:')

    const msgTwo = { user_position: { user: this.deployerAddress } }
    console.log(await this.client.queryContractSmart(this.storage.addresses.redBank!, msgTwo))
  }

  async executeBorrow() {
    const msg = {
      borrow: {
        denom: this.config.atomDenom,
        amount: '300000',
      },
    }

    await this.client.execute(this.deployerAddress, this.storage.addresses.redBank!, msg, 'auto')
    printYellow('Borrow executed:')

    const msgTwo = { user_position: { user: this.deployerAddress } }
    console.log(await this.client.queryContractSmart(this.storage.addresses.redBank!, msgTwo))
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
      this.storage.addresses.redBank!,
      msg,
      'auto',
      undefined,
      coins,
    )
    printYellow('Repay executed:')

    const msgTwo = { user_position: { user: this.deployerAddress } }
    console.log(await this.client.queryContractSmart(this.storage.addresses.redBank!, msgTwo))
  }

  async executeWithdraw() {
    const msg = {
      withdraw: {
        denom: this.config.atomDenom,
        amount: '1000000',
      },
    }

    await this.client.execute(this.deployerAddress, this.storage.addresses.redBank!, msg, 'auto')
    printYellow('Withdraw executed:')

    const msgTwo = { user_position: { user: this.deployerAddress } }
    console.log(await this.client.queryContractSmart(this.storage.addresses.redBank!, msgTwo))

    printGreen('ALL TESTS HAVE BEEN SUCCESSFUL')
  }

  async updateIncentivesContractOwner() {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    const msg = {
      update_config: {
        owner: this.storage.owner,
      },
    }
    await this.client.execute(this.deployerAddress, this.storage.addresses.incentives!, msg, 'auto')
    printYellow('Owner updated to Mutlisig for Incentives')
    const incentivesConfig = (await this.client.queryContractSmart(
      this.storage.addresses.incentives!,
      {
        config: {},
      },
    )) as { owner: string; prefix: string }

    assert.equal(incentivesConfig.owner, this.storage.owner)
  }

  async updateRedBankContractOwner() {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    const msg = {
      update_config: {
        config: {
          owner: this.storage.owner,
        },
      },
    }
    await this.client.execute(this.deployerAddress, this.storage.addresses.redBank!, msg, 'auto')
    printYellow('Owner updated to Mutlisig for Red Bank')
    const redbankConfig = (await this.client.queryContractSmart(this.storage.addresses.redBank!, {
      config: {},
    })) as { owner: string; prefix: string }

    assert.equal(redbankConfig.owner, this.storage.owner)
  }

  async updateOracleContractOwner() {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    const msg = {
      update_config: {
        owner: this.storage.owner,
      },
    }
    await this.client.execute(this.deployerAddress, this.storage.addresses.oracle!, msg, 'auto')
    printYellow('Owner updated to Mutlisig for Oracle')
    const oracleConfig = (await this.client.queryContractSmart(this.storage.addresses.oracle!, {
      config: {},
    })) as { owner: string; prefix: string }

    assert.equal(oracleConfig.owner, this.storage.owner,)
  }

  async updateRewardsContractOwner() {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    const msg = {
      update_config: {
        new_cfg: {
          owner: this.storage.owner,
        },
      },
    }
    await this.client.execute(
      this.deployerAddress,
      this.storage.addresses.rewardsCollector!,
      msg,
      'auto',
    )
    printYellow('Owner updated to Mutlisig for Rewards Collector')
    const rewardsConfig = (await this.client.queryContractSmart(
      this.storage.addresses.rewardsCollector!,
      {
        config: {},
      },
    )) as { owner: string; prefix: string }

    assert.equal(rewardsConfig.owner, this.storage.owner)
  }

  async updateAddressProviderContractOwner() {
    if (this.config.multisigAddr) {
      this.storage.owner = this.config.multisigAddr
    } else {
      this.storage.owner = this.deployerAddress
    }
    const msg = {
      transfer_ownership: {
        new_owner: this.storage.owner,
      },
    }
    await this.client.execute(
      this.deployerAddress,
      this.storage.addresses.addressProvider!,
      msg,
      'auto',
    )
    printYellow('Owner updated to Mutlisig for Rewards Collector')
    const addressProviderConfig = (await this.client.queryContractSmart(
      this.storage.addresses.addressProvider!,
      {
        config: {},
      },
    )) as { owner: string; prefix: string }

    assert.equal(addressProviderConfig.owner, this.storage.owner)
    printGreen('It is confirmed that all contracts have transferred ownership to the Multisig')
  }
}
