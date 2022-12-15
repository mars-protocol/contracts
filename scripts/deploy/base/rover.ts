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
  VaultInstantiateConfig,
} from '../../types/generated/mars-credit-manager/MarsCreditManager.types'
import { MarsMockVaultQueryClient } from '../../types/generated/mars-mock-vault/MarsMockVault.client'

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
    const amount = this.actions.depositAmount
    await this.updateCreditAccount(
      [{ deposit: { amount, denom: this.config.chain.baseDenom } }],
      [{ amount, denom: this.config.chain.baseDenom }],
    )
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.coins.length, 1)
    assert.equal(positions.coins[0].amount, amount)
    assert.equal(positions.coins[0].denom, this.config.chain.baseDenom)
    printGreen(`Deposited into credit account: ${amount} ${this.config.chain.baseDenom}`)
  }

  async withdraw() {
    const amount = this.actions.withdrawAmount
    const positionsBefore = await this.query.positions({ accountId: this.accountId! })
    const beforeWithdraw = parseFloat(
      positionsBefore.coins.find((c) => c.denom === this.config.chain.baseDenom)!.amount,
    )
    await this.updateCreditAccount([{ withdraw: { amount, denom: this.config.chain.baseDenom } }])
    const positionsAfter = await this.query.positions({ accountId: this.accountId! })
    const afterWithdraw = parseFloat(
      positionsAfter.coins.find((c) => c.denom === this.config.chain.baseDenom)!.amount,
    )
    assert.equal(beforeWithdraw - afterWithdraw, amount)
    printGreen(`Withdrew: ${amount} ${this.config.chain.baseDenom}`)
  }

  async borrow() {
    const amount = this.actions.borrowAmount
    await this.updateCreditAccount([{ borrow: { amount, denom: this.actions.secondaryDenom } }])
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.debts.length, 1)
    assert.equal(positions.debts[0].denom, this.actions.secondaryDenom)
    printGreen(`Borrowed from RedBank: ${amount} ${this.actions.secondaryDenom}`)
  }

  async repay() {
    const amount = this.actions.repayAmount
    await this.updateCreditAccount([{ repay: { amount, denom: this.actions.secondaryDenom } }])
    const positions = await this.query.positions({ accountId: this.accountId! })
    printGreen(
      `Repaid to RedBank: ${amount} ${
        this.actions.secondaryDenom
      }. Debt remaining: ${JSON.stringify(positions.debts)}`,
    )
  }

  async swap() {
    const amount = this.actions.swap.amount
    printBlue(
      `Swapping ${amount} ${this.config.chain.baseDenom} for ${this.actions.secondaryDenom}`,
    )
    const prevPositions = await this.query.positions({ accountId: this.accountId! })
    printBlue(`Previous account balance: ${JSON.stringify(prevPositions.coins)}`)
    await this.updateCreditAccount([
      {
        swap_exact_in: {
          coin_in_amount: amount,
          coin_in_denom: this.config.chain.baseDenom,
          denom_out: this.actions.secondaryDenom,
          slippage: this.actions.swap.slippage,
        },
      },
    ])
    printGreen(`Swap successful`)
    const newPositions = await this.query.positions({ accountId: this.accountId! })
    printGreen(`New account balance: ${JSON.stringify(newPositions.coins)}`)
  }

  async zap(lp_token_out: string) {
    await this.updateCreditAccount([
      {
        provide_liquidity: {
          coins_in: this.actions.zap.map((c) => ({ denom: c.denom, amount: c.amount })),
          lp_token_out,
          minimum_receive: '1',
        },
      },
    ])
    const positions = await this.query.positions({ accountId: this.accountId! })
    const lp_balance = positions.coins.find((c) => c.denom === lp_token_out)!.amount
    printGreen(
      `Zapped ${this.actions.zap
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
          lp_token_denom: lpToken.denom,
          lp_token_amount: lpToken.amount,
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

  async vaultDeposit(v: VaultInstantiateConfig, info: VaultInfo) {
    const oldRoverBalance = await this.cwClient.getBalance(
      this.storage.addresses.creditManager!,
      info.tokens.vault_token,
    )
    await this.updateCreditAccount([
      {
        enter_vault: {
          amount: this.actions.vault.depositAmount,
          denom: info.tokens.base_token,
          vault: { address: v.vault.address },
        },
      },
    ])
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.vaults.length, 1)
    const state = await this.getVaultBalance(v.vault.address)
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

  async vaultWithdraw(v: VaultInstantiateConfig, info: VaultInfo) {
    const oldBalance = await this.getAccountBalance(info.tokens.base_token)
    await this.updateCreditAccount([
      {
        exit_vault: {
          amount: this.actions.vault.withdrawAmount,
          vault: { address: v.vault.address },
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

  async vaultRequestUnlock(v: VaultInstantiateConfig, info: VaultInfo) {
    const oldBalance = await this.getVaultBalance(v.vault.address)
    await this.updateCreditAccount([
      {
        request_vault_unlock: {
          amount: this.actions.vault.withdrawAmount,
          vault: { address: v.vault.address },
        },
      },
    ])
    const newBalance = await this.getVaultBalance(v.vault.address)
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
    assert.equal(positions.coins.length, 0)
    printGreen(`Withdrew all balances back to wallet`)
  }

  async getVaultInfo(v: VaultInstantiateConfig): Promise<VaultInfo> {
    const client = new MarsMockVaultQueryClient(this.cwClient, v.vault.address)
    return {
      tokens: await client.info(),
      lockup: await this.getLockup(v),
    }
  }

  async getLockup(v: VaultInstantiateConfig): Promise<VaultInfo['lockup']> {
    try {
      return await this.cwClient.queryContractSmart(v.vault.address, {
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
