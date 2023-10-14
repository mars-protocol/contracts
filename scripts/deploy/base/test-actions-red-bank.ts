import { DeploymentConfig } from '../../types/config'
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import { printRed, printYellow } from '../../utils/chalk'
import { Storage } from './storage'
import assert from 'assert'

import { QueryMsg as RedBankQueryMsg } from '../../types/generated/mars-red-bank/MarsRedBank.types'

export class Deployer {
  constructor(
    public config: DeploymentConfig,
    public client: SigningCosmWasmClient,
    public deployerAddress: string,
    private storage: Storage,
  ) {}

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
      this.storage.addresses['redBank']!,
      msg,
      'auto',
      undefined,
      coins,
    )
    printYellow('Deposit Executed.')

    printYellow('Querying user position:')
    const msgTwo: RedBankQueryMsg = { user_position: { user: this.deployerAddress } }
    console.log(await this.client.queryContractSmart(this.storage.addresses['redBank']!, msgTwo))
  }

  async executeBorrow() {
    const msg = {
      borrow: {
        denom: this.config.atomDenom,
        amount: '300000',
      },
    }

    await this.client.execute(this.deployerAddress, this.storage.addresses['redBank']!, msg, 'auto')
    printYellow('Borrow executed:')

    const msgTwo = { user_position: { user: this.deployerAddress } }
    console.log(await this.client.queryContractSmart(this.storage.addresses['redBank']!, msgTwo))
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
      this.storage.addresses['redBank']!,
      msg,
      'auto',
      undefined,
      coins,
    )
    printYellow('Repay executed:')

    const msgTwo = { user_position: { user: this.deployerAddress } }
    console.log(await this.client.queryContractSmart(this.storage.addresses['redBank']!, msgTwo))
  }

  async executeWithdraw() {
    const msg = {
      withdraw: {
        denom: this.config.atomDenom,
        amount: '1000000',
      },
    }

    await this.client.execute(this.deployerAddress, this.storage.addresses['redBank']!, msg, 'auto')
    printYellow('Withdraw executed:')

    const msgTwo = { user_position: { user: this.deployerAddress } }
    console.log(await this.client.queryContractSmart(this.storage.addresses['redBank']!, msgTwo))
  }

  async executeRewardsSwap() {
    // Send some coins to the contract
    const coins = [
      {
        denom: this.config.atomDenom,
        amount: '20000',
      },
    ]

    const deployerAtomBalance = await this.client.getBalance(
      this.deployerAddress,
      this.config.atomDenom,
    )

    if (Number(deployerAtomBalance.amount) < Number(coins[0].amount)) {
      printRed(
        `not enough ATOM tokens to complete rewards-collector swap action, ${this.deployerAddress} has ${deployerAtomBalance.amount} ATOM but needs ${coins[0].amount}.`,
      )
    } else {
      await this.client.sendTokens(
        this.deployerAddress,
        this.storage.addresses['rewardsCollector']!,
        coins,
        'auto',
      )

      // Check contract balance before swap
      const atomBalanceBefore = await this.client.getBalance(
        this.storage.addresses['rewardsCollector']!,
        this.config.atomDenom,
      )
      const baseAssetBalanceBefore = await this.client.getBalance(
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
      await this.client.execute(
        this.deployerAddress,
        this.storage.addresses['rewardsCollector']!,
        msg,
        'auto',
      )
      // Check contract balance after swap
      const atomBalanceAfter = await this.client.getBalance(
        this.storage.addresses['rewardsCollector']!,
        this.config.atomDenom,
      )
      const baseAssetBalanceAfter = await this.client.getBalance(
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
}
