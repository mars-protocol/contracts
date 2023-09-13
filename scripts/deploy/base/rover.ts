import { Storage } from './storage'
import { DeploymentConfig, TestActions, VaultInfo } from '../../types/config'
import { difference } from 'lodash'
import assert from 'assert'
import { printBlue, printGreen } from '../../utils/chalk'
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import {
  MarsCreditManagerClient,
  MarsCreditManagerQueryClient,
} from '../../types/generated/mars-credit-manager/MarsCreditManager.client'
import { MarsAccountNftQueryClient } from '../../types/generated/mars-account-nft/MarsAccountNft.client'
import {
  Action,
  Coin,
  ConfigUpdates,
  ExecuteMsg,
} from '../../types/generated/mars-credit-manager/MarsCreditManager.types'
import { MarsMockVaultQueryClient } from '../../types/generated/mars-mock-vault/MarsMockVault.client'
import { VaultConfigBaseForString } from '../../types/generated/mars-params/MarsParams.types'

export class Rover {
  private exec: MarsCreditManagerClient
  private query: MarsCreditManagerQueryClient
  private nft: MarsAccountNftQueryClient
  private accountId?: string

  constructor(
    private userAddr: string,
    private storage: Storage,
    private config: DeploymentConfig,
    private cwClient: SigningCosmWasmClient,
    private actions: TestActions,
  ) {
    this.exec = new MarsCreditManagerClient(cwClient, userAddr, storage.addresses.creditManager!)
    this.query = new MarsCreditManagerQueryClient(cwClient, storage.addresses.creditManager!)
    this.nft = new MarsAccountNftQueryClient(cwClient, storage.addresses.accountNft!)
  }

  async updateConfig(updates: ConfigUpdates) {
    await this.exec.updateConfig({ updates })
  }

  async createCreditAccount() {
    const before = await this.nft.tokens({ owner: this.userAddr })
    const executeMsg = { create_credit_account: 'default' } satisfies ExecuteMsg
    await this.cwClient.execute(
      this.userAddr,
      this.storage.addresses.creditManager!,
      executeMsg,
      'auto',
    )
    const after = await this.nft.tokens({ owner: this.userAddr })
    const diff = difference(after.tokens, before.tokens)
    assert.equal(diff.length, 1)
    this.accountId = diff[0]
    printGreen(`Newly created credit account id: #${diff[0]}`)
  }

  async deposit() {
    const amount = this.actions.depositAmount
    await this.updateCreditAccount(
      [{ deposit: { amount, denom: this.config.chain.baseDenom } }],
      [{ amount, denom: this.config.chain.baseDenom }],
    )
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.deposits.length, 1)
    assert.equal(positions.deposits[0].amount, amount)
    assert.equal(positions.deposits[0].denom, this.config.chain.baseDenom)
    printGreen(`Deposited into credit account: ${amount} ${this.config.chain.baseDenom}`)
  }

  async lend() {
    const amount = this.actions.lendAmount
    await this.updateCreditAccount(
      [{ lend: { amount: { exact: amount }, denom: this.config.chain.baseDenom } }],
      [],
    )
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.lends.length, 1)
    assert.equal(positions.lends[0].denom, this.config.chain.baseDenom)
    printGreen(`Lent to Red Bank: ${amount} ${this.config.chain.baseDenom}`)
  }

  async withdraw() {
    const amount = this.actions.withdrawAmount
    const positionsBefore = await this.query.positions({ accountId: this.accountId! })
    const beforeWithdraw = parseFloat(
      positionsBefore.deposits.find((c) => c.denom === this.config.chain.baseDenom)!.amount,
    )
    await this.updateCreditAccount([
      { withdraw: { amount: { exact: amount }, denom: this.config.chain.baseDenom } },
    ])
    const positionsAfter = await this.query.positions({ accountId: this.accountId! })
    const afterWithdraw = parseFloat(
      positionsAfter.deposits.find((c) => c.denom === this.config.chain.baseDenom)!.amount,
    )
    assert.equal(beforeWithdraw - afterWithdraw, amount)
    printGreen(`Withdrew: ${amount} ${this.config.chain.baseDenom}`)
  }

  async borrow() {
    const amount = this.actions.borrowAmount
    await this.updateCreditAccount([{ borrow: { amount, denom: this.config.chain.baseDenom } }])
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.debts.length, 1)
    assert.equal(positions.debts[0].denom, this.config.chain.baseDenom)
    printGreen(`Borrowed from RedBank: ${amount} ${this.config.chain.baseDenom}`)
  }

  async repay() {
    const amount = this.actions.repayAmount
    await this.updateCreditAccount([
      { repay: { coin: { amount: { exact: amount }, denom: this.config.chain.baseDenom } } },
    ])
    const positions = await this.query.positions({ accountId: this.accountId! })
    printGreen(
      `Repaid to RedBank: ${amount} ${
        this.config.chain.baseDenom
      }. Debt remaining: ${JSON.stringify(positions.debts)}`,
    )
  }

  async reclaim() {
    const positions = await this.query.positions({ accountId: this.accountId! })

    const amount = this.actions.reclaimAmount
    await this.updateCreditAccount([
      { reclaim: { amount: { exact: amount }, denom: this.config.chain.baseDenom } },
    ])
    printGreen(
      `User reclaimed: ${amount} ${
        this.config.chain.baseDenom
      }. Lent amount remaining: ${JSON.stringify(positions.lends)}`,
    )
  }

  async swap() {
    const amount = this.actions.swap.amount
    printBlue(
      `Swapping ${amount} ${this.config.chain.baseDenom} for ${this.actions.secondaryDenom}`,
    )
    const prevPositions = await this.query.positions({ accountId: this.accountId! })
    printBlue(`Previous account balance: ${JSON.stringify(prevPositions.deposits)}`)
    await this.updateCreditAccount([
      {
        swap_exact_in: {
          coin_in: { amount: { exact: amount }, denom: this.config.chain.baseDenom },
          denom_out: this.actions.secondaryDenom,
          slippage: this.actions.swap.slippage,
        },
      },
    ])
    printGreen(`Swap successful`)
    const newPositions = await this.query.positions({ accountId: this.accountId! })
    printGreen(`New account balance: ${JSON.stringify(newPositions.deposits)}`)
  }

  async zap(lp_token_out: string) {
    await this.updateCreditAccount([
      {
        provide_liquidity: {
          coins_in: this.actions.zap.coinsIn.map((c) => ({
            denom: c.denom,
            amount: { exact: c.amount },
          })),
          lp_token_out,
          slippage: '0.05',
        },
      },
    ])
    const positions = await this.query.positions({ accountId: this.accountId! })
    const lp_balance = positions.deposits.find((c) => c.denom === lp_token_out)!.amount
    printGreen(
      `Zapped ${this.actions.zap.coinsIn
        .map((c) => c.denom)
        .join(', ')} for LP token: ${lp_balance} ${lp_token_out}`,
    )
  }

  async unzap(lp_token_in: string) {
    const lpToken = {
      denom: lp_token_in,
      amount: this.actions.unzapAmount,
    }
    await this.updateCreditAccount([
      {
        withdraw_liquidity: {
          lp_token: { amount: { exact: lpToken.amount }, denom: lpToken.denom },
          slippage: '0.05',
        },
      },
    ])
    const underlying = await this.query.estimateWithdrawLiquidity({ lpToken })
    printGreen(
      `Unzapped ${lp_token_in} ${this.actions.unzapAmount} for underlying: ${underlying
        .map((c) => `${c.amount} ${c.denom}`)
        .join(', ')}`,
    )
  }

  async vaultDeposit(v: VaultConfigBaseForString, info: VaultInfo) {
    const oldRoverBalance = await this.cwClient.getBalance(
      this.storage.addresses.creditManager!,
      info.tokens.vault_token,
    )
    printBlue('testing vault deposit')
    printGreen(v.addr)
    printGreen(this.actions.vault.depositAmount)
    printGreen(info.tokens.base_token)
    await this.updateCreditAccount([
      {
        enter_vault: {
          coin: {
            amount: { exact: this.actions.vault.depositAmount },
            denom: info.tokens.base_token,
          },
          vault: { address: v.addr },
        },
      },
    ])
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.vaults.length, 1)
    const state = await this.getVaultBalance(v.addr)
    assert(state.locked > 0 || state.unlocked > 0)
    const newRoverBalance = await this.cwClient.getBalance(
      this.storage.addresses.creditManager!,
      info.tokens.vault_token,
    )
    const newAmount = parseInt(newRoverBalance.amount) - parseInt(oldRoverBalance.amount)
    assert(newAmount === state.locked || newAmount === state.unlocked)

    printGreen(
      `Deposited ${this.actions.vault.depositAmount} ${
        info.tokens.base_token
      } in exchange for ${JSON.stringify(positions.vaults[0].amount)} vault tokens (${
        info.tokens.vault_token
      })`,
    )
  }

  async vaultWithdraw(v: VaultConfigBaseForString, info: VaultInfo) {
    const oldBalance = await this.getAccountBalance(info.tokens.base_token)
    await this.updateCreditAccount([
      {
        exit_vault: {
          amount: this.actions.vault.withdrawAmount,
          vault: { address: v.addr },
        },
      },
    ])
    const newBalance = await this.getAccountBalance(info.tokens.base_token)
    assert(newBalance > oldBalance)
    printGreen(
      `Withdrew ${newBalance - oldBalance} ${info.tokens.base_token} in exchange for ${
        this.actions.vault.withdrawAmount
      } ${info.tokens.vault_token} vault tokens`,
    )
  }

  async vaultRequestUnlock(v: VaultConfigBaseForString, info: VaultInfo) {
    const oldBalance = await this.getVaultBalance(v.addr)
    await this.updateCreditAccount([
      {
        request_vault_unlock: {
          amount: this.actions.vault.withdrawAmount,
          vault: { address: v.addr },
        },
      },
    ])
    const newBalance = await this.getVaultBalance(v.addr)
    assert(newBalance.locked < oldBalance.locked)
    assert.equal(newBalance.unlocking.length, 1)

    printGreen(
      `Requested unlock: ID #${newBalance.unlocking[0].id}, amount: ${
        newBalance.unlocking[0].coin.amount
      } ${newBalance.unlocking[0].coin.denom} in exchange for: ${
        oldBalance.locked - newBalance.locked
      } ${info.tokens.vault_token}`,
    )
  }

  async refundAllBalances() {
    await this.updateCreditAccount([{ refund_all_coin_balances: {} }])
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.deposits.length, 0)
    printGreen(`Withdrew all balances back to wallet`)
  }

  async getVaultInfo(v: VaultConfigBaseForString): Promise<VaultInfo> {
    const client = new MarsMockVaultQueryClient(this.cwClient, v.addr)
    return {
      tokens: await client.info(),
      lockup: await this.getLockup(v),
    }
  }

  private async getLockup(v: VaultConfigBaseForString): Promise<VaultInfo['lockup']> {
    try {
      return await this.cwClient.queryContractSmart(v.addr, {
        vault_extension: {
          lockup: {
            lockup_duration: {},
          },
        },
      })
    } catch (e) {
      return undefined
    }
  }

  private async getAccountBalance(denom: string) {
    const positions = await this.query.positions({ accountId: this.accountId! })
    const coin = positions.deposits.find((c) => c.denom === denom)
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
