import { DeploymentConfig } from '../../types/config'
import { SigningCosmWasmClient, SigningCosmWasmClientOptions } from '@cosmjs/cosmwasm-stargate'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { GasPrice } from '@cosmjs/stargate'
import { Deployer } from './deployer'
import { Storage } from './storage'

export const getWallet = async (config: DeploymentConfig, mnemonic: string) => {
  return await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, {
    prefix: config.chainPrefix,
  })
}

export const getDeployer = async (wallet: DirectSecp256k1HdWallet) => {
  const accounts = await wallet.getAccounts()
  return accounts[0].address
}

export const setupClient = async (config: DeploymentConfig, wallet: DirectSecp256k1HdWallet) => {
  const clientOption: SigningCosmWasmClientOptions = {
    gasPrice: GasPrice.fromString(config.gasPrice),
  }
  return await SigningCosmWasmClient.connectWithSigner(config.rpcEndpoint, wallet, clientOption)
}

export const setupDeployer = async (config: DeploymentConfig) => {
  const mnemonic = process.env.MNEMONIC

  if (!mnemonic) {
    throw new Error('The environment variable MNEMONIC is not set.')
  }

  const wallet = await getWallet(config, mnemonic)
  const client = await setupClient(config, wallet)
  const deployerAddr = await getDeployer(wallet)
  const storage = await Storage.load(config.chainId)
  return new Deployer(config, client, deployerAddr, storage)
}
