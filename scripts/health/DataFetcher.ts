import { Positions } from '../types/generated/mars-credit-manager/MarsCreditManager.types'
import { MarsCreditManagerQueryClient } from '../types/generated/mars-credit-manager/MarsCreditManager.client'
import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate/build/cosmwasmclient'
import { HealthResponse } from '../types/generated/mars-rover-health-types/MarsRoverHealthTypes.types'
import {
  DenomsData,
  HealthComputer,
  VaultsData,
} from '../types/generated/mars-rover-health-computer/MarsRoverHealthComputer.types'
import { MarsMockOracleQueryClient } from '../types/generated/mars-mock-oracle/MarsMockOracle.client'
import { MarsMockRedBankQueryClient } from '../types/generated/mars-mock-red-bank/MarsMockRedBank.client'
import { MarsMockVaultQueryClient } from '../types/generated/mars-mock-vault/MarsMockVault.client'

export class DataFetcher {
  constructor(
    private healthComputer: (h: HealthComputer) => HealthResponse,
    private creditManagerAddr: string,
    private oracleAddr: string,
    private redBankAddr: string,
    private rpcEndpoint: string,
  ) {}

  getClient = async (): Promise<CosmWasmClient> => {
    return await CosmWasmClient.connect(this.rpcEndpoint)
  }

  fetchPositions = async (accountId: string): Promise<Positions> => {
    const cmQuery = new MarsCreditManagerQueryClient(await this.getClient(), this.creditManagerAddr)
    return await cmQuery.positions({ accountId })
  }

  fetchMarkets = async (denoms: string[]): Promise<DenomsData['markets']> => {
    const rQuery = new MarsMockRedBankQueryClient(await this.getClient(), this.redBankAddr)
    const promises = denoms.map(async (denom) => await rQuery.market({ denom }))
    const responses = await Promise.all(promises)
    return responses.reduce((acc, curr) => {
      acc[curr.denom] = curr
      return acc
    }, {} as DenomsData['markets'])
  }

  fetchPrices = async (denoms: string[]): Promise<DenomsData['prices']> => {
    const oQuery = new MarsMockOracleQueryClient(await this.getClient(), this.oracleAddr)
    const promises = denoms.map(async (denom) => await oQuery.price({ denom }))
    const responses = await Promise.all(promises)
    return responses.reduce((acc, curr) => {
      acc[curr.denom] = curr.price
      return acc
    }, {} as DenomsData['prices'])
  }

  fetchDenomsData = async (positions: Positions): Promise<DenomsData> => {
    const depositDenoms = positions.deposits.map((c) => c.denom)
    const debtDenoms = positions.debts.map((c) => c.denom)
    const vaultBaseTokenDenoms = await Promise.all(
      positions.vaults.map(async (v) => {
        const vQuery = new MarsMockVaultQueryClient(await this.getClient(), v.vault.address)
        const info = await vQuery.info()
        return info.base_token
      }),
    )

    const allDenoms = depositDenoms.concat(debtDenoms).concat(vaultBaseTokenDenoms)

    return {
      markets: await this.fetchMarkets(allDenoms),
      prices: await this.fetchPrices(allDenoms),
    }
  }

  fetchAllowedCoins = async (): Promise<string[]> => {
    const cmQuery = new MarsCreditManagerQueryClient(await this.getClient(), this.creditManagerAddr)
    return await cmQuery.allowedCoins({})
  }

  fetchVaultsData = async (positions: Positions): Promise<VaultsData> => {
    const vaultsData = { vault_values: {}, vault_configs: {} } as VaultsData
    const cmQuery = new MarsCreditManagerQueryClient(await this.getClient(), this.creditManagerAddr)
    await Promise.all(
      positions.vaults.map(async (v) => {
        const values = await cmQuery.vaultPositionValue({ vaultPosition: v })
        vaultsData.vault_values[v.vault.address] = values

        const info = await cmQuery.vaultInfo({ vault: v.vault })
        vaultsData.vault_configs[v.vault.address] = info.config
      }),
    )
    return vaultsData
  }

  fetchHealth = async (accountId: string): Promise<HealthResponse> => {
    const positions = await this.fetchPositions(accountId)

    const [denoms_data, vaults_data, allowed_coins] = await Promise.all([
      this.fetchDenomsData(positions),
      this.fetchVaultsData(positions),
      this.fetchAllowedCoins(),
    ])

    let data = {
      positions,
      denoms_data,
      allowed_coins,
      vaults_data,
    }
    return this.healthComputer(data)
  }
}
