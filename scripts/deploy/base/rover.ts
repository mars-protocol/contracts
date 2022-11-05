import {
  CreditManagerClient,
  CreditManagerQueryClient,
} from '../../types/generated/credit-manager/CreditManager.client'
import { AccountNftQueryClient } from '../../types/generated/account-nft/AccountNft.client'
import { Storage } from './storage'
import { DeploymentConfig } from '../../types/config'
import { difference } from 'lodash'
import assert from 'assert'
import { printBlue, printGreen } from '../../utils/chalk'
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import {
  Action,
  Coin,
  ConfigUpdates,
} from '../../types/generated/credit-manager/CreditManager.types'

export class Rover {
  private exec: CreditManagerClient
  private query: CreditManagerQueryClient
  private nft: AccountNftQueryClient
  private accountId?: string

  constructor(
    private userAddr: string,
    private storage: Storage,
    private config: DeploymentConfig,
    private cwClient: SigningCosmWasmClient,
  ) {
    this.exec = new CreditManagerClient(cwClient, userAddr, storage.addresses.creditManager!)
    this.query = new CreditManagerQueryClient(cwClient, storage.addresses.creditManager!)
    this.nft = new AccountNftQueryClient(cwClient, storage.addresses.accountNft!)
  }

  async updateConfig(newConfig: ConfigUpdates) {
    await this.exec.updateConfig({ newConfig })
  }

  async createCreditAccount() {
    const before = await this.nft.tokens({ owner: this.userAddr })
    await this.exec.createCreditAccount()
    const after = await this.nft.tokens({ owner: this.userAddr })
    const diff = difference(after.tokens, before.tokens)
    assert.equal(diff.length, 1)
    this.accountId = diff[0]
    printGreen(`Newly created credit account id: #${diff[0]}`)
  }

  async deposit() {
    const amount = this.config.depositAmount.toString()
    await this.updateCreditAccount(
      [{ deposit: { amount, denom: this.config.baseDenom } }],
      [{ amount, denom: this.config.baseDenom }],
    )
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.coins.length, 1)
    assert.equal(positions.coins[0].amount, amount)
    assert.equal(positions.coins[0].denom, this.config.baseDenom)
    printGreen(`Deposited into credit account: ${amount} ${this.config.baseDenom}`)
  }

  async withdraw() {
    const amount = this.config.withdrawAmount.toString()
    const positionsBefore = await this.query.positions({ accountId: this.accountId! })
    const beforeWithdraw = parseFloat(
      positionsBefore.coins.find((c) => c.denom === this.config.baseDenom)!.amount,
    )
    await this.updateCreditAccount([{ withdraw: { amount, denom: this.config.baseDenom } }])
    const positionsAfter = await this.query.positions({ accountId: this.accountId! })
    const afterWithdraw = parseFloat(
      positionsAfter.coins.find((c) => c.denom === this.config.baseDenom)!.amount,
    )
    assert.equal(beforeWithdraw - afterWithdraw, amount)
    printGreen(`Withdrew: ${amount} ${this.config.baseDenom}`)
  }

  async borrow() {
    const amount = this.config.borrowAmount.toString()
    await this.updateCreditAccount([{ borrow: { amount, denom: this.config.secondaryDenom } }])
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.debts.length, 1)
    assert.equal(positions.debts[0].denom, this.config.secondaryDenom)
    printGreen(`Borrowed from RedBank: ${amount} ${this.config.secondaryDenom}`)
  }

  async repay() {
    const amount = this.config.repayAmount.toString()
    await this.updateCreditAccount([{ repay: { amount, denom: this.config.secondaryDenom } }])
    const positions = await this.query.positions({ accountId: this.accountId! })
    printGreen(
      `Repaid to RedBank: ${amount} ${this.config.secondaryDenom}. Debt remaining: ${positions.debts[0].amount} ${positions.debts[0].denom}`,
    )
  }

  async swap() {
    const amount = this.config.swapAmount.toString()
    printBlue(`Swapping ${amount} ${this.config.baseDenom} for ${this.config.secondaryDenom}`)
    const prevPositions = await this.query.positions({ accountId: this.accountId! })
    printBlue(
      `Previous account balance: ${prevPositions.coins[0].amount} ${prevPositions.coins[0].denom}`,
    )
    await this.updateCreditAccount([
      {
        swap_exact_in: {
          coin_in: { amount, denom: this.config.baseDenom },
          denom_out: this.config.secondaryDenom,
          slippage: this.config.slippage.toString(),
        },
      },
    ])
    printGreen(`Swap successful`)
    const newPositions = await this.query.positions({ accountId: this.accountId! })
    printGreen(
      `New account balance: ${newPositions.coins[0].amount} ${newPositions.coins[0].denom}, ${newPositions.coins[1].amount} ${newPositions.coins[1].denom}`,
    )
  }

  async zap() {
    await this.updateCreditAccount([
      {
        provide_liquidity: {
          coins_in: this.config.zap.map((c) => ({ denom: c.denom, amount: c.amount.toString() })),
          lp_token_out: this.config.lpToken.denom,
          minimum_receive: '1',
        },
      },
    ])
    const positions = await this.query.positions({ accountId: this.accountId! })
    const lp_balance = positions.coins.find((c) => c.denom === this.config.lpToken.denom)!.amount
    printGreen(
      `Zapped ${this.config.zap.map((c) => c.denom).join(', ')} for LP token: ${lp_balance} ${
        this.config.lpToken.denom
      }`,
    )
  }

  async unzap() {
    const lpToken = { denom: this.config.lpToken.denom, amount: this.config.unzapAmount.toString() }
    await this.updateCreditAccount([
      {
        withdraw_liquidity: {
          lp_token: lpToken,
        },
      },
    ])
    const underlying = await this.query.estimateWithdrawLiquidity({ lpToken })
    printGreen(
      `Unzapped ${this.config.lpToken.denom} ${this.config.unzapAmount} for underlying: ${underlying
        .map((c) => `${c.amount} ${c.denom}`)
        .join(', ')}`,
    )
  }

  async vaultDeposit() {
    const oldRoverBalance = await this.cwClient.getBalance(
      this.storage.addresses.creditManager!,
      this.config.vaultTokenDenom,
    )
    await this.updateCreditAccount([
      {
        enter_vault: {
          amount: this.config.vaultDepositAmount.toString(),
          denom: this.config.baseDenom,
          vault: { address: this.storage.addresses.mockVault! },
        },
      },
    ])
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.vaults.length, 1)
    const state = await this.getVaultBalance(this.storage.addresses.mockVault!)
    assert(state.locked > 0 || state.unlocked > 0)
    const newRoverBalance = await this.cwClient.getBalance(
      this.storage.addresses.creditManager!,
      this.config.vaultTokenDenom,
    )
    const newAmount = parseInt(newRoverBalance.amount) - parseInt(oldRoverBalance.amount)
    assert(newAmount === state.locked || newAmount === state.unlocked)

    printGreen(
      `Deposited ${this.config.vaultDepositAmount} ${
        this.config.baseDenom
      } in exchange for vault tokens: ${JSON.stringify(positions.vaults[0])}`,
    )
  }

  async vaultWithdraw() {
    const oldBalance = await this.getAccountBalance(this.config.baseDenom)
    await this.updateCreditAccount([
      {
        exit_vault: {
          amount: this.config.vaultWithdrawAmount.toString(),
          vault: { address: this.storage.addresses.mockVault! },
        },
      },
    ])
    const newBalance = await this.getAccountBalance(this.config.baseDenom)
    assert(newBalance > oldBalance)
    printGreen(
      `Withdrew ${newBalance - oldBalance} ${this.config.baseDenom} in exchange for ${
        this.config.vaultWithdrawAmount
      } ${this.config.vaultTokenDenom}`,
    )
  }

  async vaultRequestUnlock() {
    const oldBalance = await this.getVaultBalance(this.storage.addresses.mockVault!)
    await this.updateCreditAccount([
      {
        request_vault_unlock: {
          amount: this.config.vaultWithdrawAmount.toString(),
          vault: { address: this.storage.addresses.mockVault! },
        },
      },
    ])
    const newBalance = await this.getVaultBalance(this.storage.addresses.mockVault!)
    assert(newBalance.locked < oldBalance.locked)
    assert.equal(newBalance.unlocking.length, 1)

    printGreen(
      `Requested unlock: ID #${newBalance.unlocking[0].id}, amount: ${
        newBalance.unlocking[0].coin.amount
      } in exchange for: ${oldBalance.locked - newBalance.locked} ${this.config.vaultTokenDenom}`,
    )
  }

  private async getAccountBalance(denom: string) {
    const positions = await this.query.positions({ accountId: this.accountId! })
    const coin = positions.coins.find((c) => c.denom === denom)
    if (!coin) throw new Error(`No balance of ${denom}`)
    return parseInt(coin.amount)
  }

  private async getVaultBalance(vaultAddr: string) {
    const positions = await this.query.positions({ accountId: this.accountId! })
    const vault = positions.vaults.find((p) => p.vault.address === vaultAddr)
    if (!vault) throw new Error(`No balance for ${vaultAddr}`)

    if ('unlocked' in vault.amount) {
      return {
        unlocked: parseInt(vault.amount.unlocked),
        locked: 0,
        unlocking: [],
      }
    } else {
      return {
        unlocked: 0,
        locked: parseInt(vault.amount.locking.locked),
        unlocking: vault.amount.locking.unlocking.map((lockup) => ({
          id: lockup.id,
          coin: { denom: lockup.coin.denom, amount: parseInt(lockup.coin.amount) },
        })),
      }
    }
  }

  private async updateCreditAccount(actions: Action[], funds?: Coin[]) {
    return await this.exec.updateCreditAccount(
      { actions, accountId: this.accountId! },
      'auto',
      undefined,
      funds,
    )
  }
}
