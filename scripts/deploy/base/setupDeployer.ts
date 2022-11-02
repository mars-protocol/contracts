import { DeploymentConfig } from '../../types/config'
import { SigningCosmWasmClient, SigningCosmWasmClientOptions } from '@cosmjs/cosmwasm-stargate'
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing'
import { GasPrice } from '@cosmjs/stargate'
import { Deployer } from './deployer'
import { Storage } from './storage'

const getWallet = async (config: DeploymentConfig) => {
  return await DirectSecp256k1HdWallet.fromMnemonic(config.deployerMnemonic, {
    prefix: config.chainPrefix,
  })
}

const getDeployer = async (wallet: DirectSecp256k1HdWallet) => {
  const accounts = await wallet.getAccounts()
  return accounts[0].address
}

const setupClient = async (config: DeploymentConfig, wallet: DirectSecp256k1HdWallet) => {
  const clientOption: SigningCosmWasmClientOptions = {
    gasPrice: GasPrice.fromString(`0.1${config.baseAssetDenom}`),
  }
  return await SigningCosmWasmClient.connectWithSigner(config.rpcEndpoint, wallet, clientOption)
}

export const setupDeployer = async (config: DeploymentConfig) => {
  const wallet = await getWallet(config)
  const client = await setupClient(config, wallet)
  const deployerAddr = await getDeployer(wallet)
  const storage = await Storage.load(config.chainId)
  return new Deployer(config, client, deployerAddr, storage)
}
