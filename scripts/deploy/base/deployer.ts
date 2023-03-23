import { DeploymentConfig, OracleConfig } from '../../types/config'
import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate'
import * as fs from 'fs'
import { printBlue, printGreen, printRed, printYellow } from '../../utils/chalk'
import { ARTIFACTS_PATH, Storage } from './storage'
import { InstantiateMsgs } from '../../types/msg'
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

  setOwnerAddr() {
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

    const { contractAddress } = await this.client.instantiate(
      this.deployerAddress,
      codeId,
      msg,
      `mars-${name}`,
      'auto',
      { admin: this.storage.owner },
    )

    this.storage.addresses[name] = contractAddress
    printGreen(
      `${this.config.chainId} :: ${name} Contract Address : ${this.storage.addresses[name]}`,
    )
  }

  async instantiateOracle() {
    const msg = {
      owner: this.deployerAddress,
      base_denom: this.config.baseAssetDenom,
    }
    await this.instantiate('oracle', this.storage.codeIds.oracle!, msg)
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
}
