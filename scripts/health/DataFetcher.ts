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
import { MarsMockVaultQueryClient } from '../types/generated/mars-mock-vault/MarsMockVault.client'
import { MarsParamsQueryClient } from '../types/generated/mars-params/MarsParams.client'

export class DataFetcher {
  constructor(
    private healthComputer: (h: HealthComputer) => HealthResponse,
    private creditManagerAddr: string,
    private oracleAddr: string,
    private paramsAddr: string,
    private rpcEndpoint: string,
  ) {}

  getClient = async (): Promise<CosmWasmClient> => {
    return await CosmWasmClient.connect(this.rpcEndpoint)
  }

  fetchPositions = async (accountId: string): Promise<Positions> => {
    const cmQuery = new MarsCreditManagerQueryClient(await this.getClient(), this.creditManagerAddr)
    return await cmQuery.positions({ accountId })
  }

  fetchParams = async (denoms: string[]): Promise<DenomsData['params']> => {
    const pQuery = new MarsParamsQueryClient(await this.getClient(), this.paramsAddr)
    const promises = denoms.map(async (denom) => ({
      denom: denom,
      params: await pQuery.assetParams({ denom }),
    }))
    const responses = await Promise.all(promises)
    return responses.reduce((acc, curr) => {
      acc[curr.denom] = curr.params
      return acc
    }, {} as DenomsData['params'])
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
      params: await this.fetchParams(allDenoms),
      prices: await this.fetchPrices(allDenoms),
    }
  }

  fetchVaultsData = async (positions: Positions): Promise<VaultsData> => {
    const vaultsData = { vault_values: {}, vault_configs: {} } as VaultsData
    const cmQuery = new MarsCreditManagerQueryClient(await this.getClient(), this.creditManagerAddr)
    const pQuery = new MarsParamsQueryClient(await this.getClient(), this.paramsAddr)
    await Promise.all(
      positions.vaults.map(async (v) => {
        vaultsData.vault_values[v.vault.address] = await cmQuery.vaultPositionValue({
          vaultPosition: v,
        })

        vaultsData.vault_configs[v.vault.address] = await pQuery.vaultConfig({
          address: v.vault.address,
        })
      }),
    )
    return vaultsData
  }

  fetchHealth = async (accountId: string): Promise<HealthResponse> => {
    const positions = await this.fetchPositions(accountId)

    const [denoms_data, vaults_data] = await Promise.all([
      this.fetchDenomsData(positions),
      this.fetchVaultsData(positions),
    ])

    let data = {
      positions,
      denoms_data,
      vaults_data,
    }
    return this.healthComputer(data)
  }
}
