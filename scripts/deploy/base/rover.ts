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
    cwClient: SigningCosmWasmClient,
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
    printGreen(`Deposited: ${amount} ${this.config.baseDenom}`)
  }

  async withdraw() {
    const amount = this.config.withdrawAmount.toString()
    const positionsBefore = await this.query.positions({ accountId: this.accountId! })
    const beforeWithdraw = parseFloat(positionsBefore.coins[0].amount)
    await this.updateCreditAccount([{ withdraw: { amount, denom: this.config.baseDenom } }])
    const positionsAfter = await this.query.positions({ accountId: this.accountId! })
    const afterWithdraw = parseFloat(positionsAfter.coins[0].amount)
    assert.equal(beforeWithdraw - afterWithdraw, amount)
    printGreen(`Withdrew: ${amount} ${this.config.baseDenom}`)
  }

  async borrow() {
    const amount = this.config.borrowAmount.toString()
    await this.updateCreditAccount([{ borrow: { amount, denom: this.config.baseDenom } }])
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.debt.length, 1)
    assert.equal(positions.debt[0].denom, this.config.baseDenom)
    printGreen(`Borrowed from RedBank: ${amount} ${this.config.baseDenom}`)
  }

  async repay() {
    const amount = this.config.repayAmount.toString()
    await this.updateCreditAccount([{ repay: { amount, denom: this.config.baseDenom } }])
    const positions = await this.query.positions({ accountId: this.accountId! })
    printGreen(
      `Repaid to RedBank: ${amount} ${this.config.baseDenom}. Debt remaining: ${positions.debt[0].amount} ${positions.debt[0].denom}`,
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

  async vaultDeposit() {
    const amount = '10'
    await this.updateCreditAccount([
      {
        vault_deposit: {
          coins: [{ amount, denom: this.config.baseDenom }],
          vault: this.storage.addresses.mockVault!,
        },
      },
    ])
    const positions = await this.query.positions({ accountId: this.accountId! })
    assert.equal(positions.vault_positions.length, 1)
    assert.equal(positions.vault_positions[0].addr, this.storage.addresses.mockVault)
    assert.equal(positions.vault_positions[0].position, this.config.baseDenom)
    printGreen(
      `Deposit into vault: ${amount} ${this.config.baseDenom}, Vault Postition: ${JSON.stringify(
        positions.vault_positions[0].position,
      )}`,
    )
  }

  private async updateCreditAccount(actions: Action[], funds?: Coin[]) {
    await this.exec.updateCreditAccount(
      { actions, accountId: this.accountId! },
      'auto',
      undefined,
      funds,
    )
  }
}
