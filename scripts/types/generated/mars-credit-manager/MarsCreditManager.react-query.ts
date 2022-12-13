// @ts-nocheck
/**
 * This file was automatically generated by @cosmwasm/ts-codegen@0.24.0.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run the @cosmwasm/ts-codegen generate command to regenerate this file.
 */

import { UseQueryOptions, useQuery, useMutation, UseMutationOptions } from '@tanstack/react-query'
import { ExecuteResult } from '@cosmjs/cosmwasm-stargate'
import { StdFee } from '@cosmjs/amino'
import {
  Decimal,
  Uint128,
  OracleBaseForString,
  RedBankBaseForString,
  SwapperBaseForString,
  ZapperBaseForString,
  InstantiateMsg,
  VaultInstantiateConfig,
  VaultConfig,
  Coin,
  VaultBaseForString,
  ExecuteMsg,
  Action,
  VaultPositionType,
  AdminUpdate,
  CallbackMsg,
  Addr,
  ConfigUpdates,
  VaultBaseForAddr,
  QueryMsg,
  ArrayOfCoinBalanceResponseItem,
  CoinBalanceResponseItem,
  ArrayOfSharesResponseItem,
  SharesResponseItem,
  ArrayOfDebtShares,
  DebtShares,
  ArrayOfVaultWithBalance,
  VaultWithBalance,
  VaultPositionAmount,
  VaultAmount,
  VaultAmount1,
  UnlockingPositions,
  ArrayOfVaultPositionResponseItem,
  VaultPositionResponseItem,
  VaultPosition,
  LockingVaultAmount,
  VaultUnlockingPosition,
  ArrayOfString,
  ConfigResponse,
  ArrayOfCoin,
  HealthResponse,
  Positions,
  DebtAmount,
  ArrayOfVaultInstantiateConfig,
} from './MarsCreditManager.types'
import { MarsCreditManagerQueryClient, MarsCreditManagerClient } from './MarsCreditManager.client'
export const marsCreditManagerQueryKeys = {
  contract: [
    {
      contract: 'marsCreditManager',
    },
  ] as const,
  address: (contractAddress: string | undefined) =>
    [{ ...marsCreditManagerQueryKeys.contract[0], address: contractAddress }] as const,
  config: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsCreditManagerQueryKeys.address(contractAddress)[0], method: 'config', args },
    ] as const,
  vaultConfigs: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsCreditManagerQueryKeys.address(contractAddress)[0], method: 'vault_configs', args },
    ] as const,
  allowedCoins: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsCreditManagerQueryKeys.address(contractAddress)[0], method: 'allowed_coins', args },
    ] as const,
  positions: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsCreditManagerQueryKeys.address(contractAddress)[0], method: 'positions', args },
    ] as const,
  health: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsCreditManagerQueryKeys.address(contractAddress)[0], method: 'health', args },
    ] as const,
  allCoinBalances: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      {
        ...marsCreditManagerQueryKeys.address(contractAddress)[0],
        method: 'all_coin_balances',
        args,
      },
    ] as const,
  allDebtShares: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      {
        ...marsCreditManagerQueryKeys.address(contractAddress)[0],
        method: 'all_debt_shares',
        args,
      },
    ] as const,
  totalDebtShares: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      {
        ...marsCreditManagerQueryKeys.address(contractAddress)[0],
        method: 'total_debt_shares',
        args,
      },
    ] as const,
  allTotalDebtShares: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      {
        ...marsCreditManagerQueryKeys.address(contractAddress)[0],
        method: 'all_total_debt_shares',
        args,
      },
    ] as const,
  allVaultPositions: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      {
        ...marsCreditManagerQueryKeys.address(contractAddress)[0],
        method: 'all_vault_positions',
        args,
      },
    ] as const,
  totalVaultCoinBalance: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      {
        ...marsCreditManagerQueryKeys.address(contractAddress)[0],
        method: 'total_vault_coin_balance',
        args,
      },
    ] as const,
  allTotalVaultCoinBalances: (
    contractAddress: string | undefined,
    args?: Record<string, unknown>,
  ) =>
    [
      {
        ...marsCreditManagerQueryKeys.address(contractAddress)[0],
        method: 'all_total_vault_coin_balances',
        args,
      },
    ] as const,
  estimateProvideLiquidity: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      {
        ...marsCreditManagerQueryKeys.address(contractAddress)[0],
        method: 'estimate_provide_liquidity',
        args,
      },
    ] as const,
  estimateWithdrawLiquidity: (
    contractAddress: string | undefined,
    args?: Record<string, unknown>,
  ) =>
    [
      {
        ...marsCreditManagerQueryKeys.address(contractAddress)[0],
        method: 'estimate_withdraw_liquidity',
        args,
      },
    ] as const,
}
export interface MarsCreditManagerReactQuery<TResponse, TData = TResponse> {
  client: MarsCreditManagerQueryClient | undefined
  options?: Omit<
    UseQueryOptions<TResponse, Error, TData>,
    "'queryKey' | 'queryFn' | 'initialData'"
  > & {
    initialData?: undefined
  }
}
export interface MarsCreditManagerEstimateWithdrawLiquidityQuery<TData>
  extends MarsCreditManagerReactQuery<ArrayOfCoin, TData> {
  args: {
    lpToken: Coin
  }
}
export function useMarsCreditManagerEstimateWithdrawLiquidityQuery<TData = ArrayOfCoin>({
  client,
  args,
  options,
}: MarsCreditManagerEstimateWithdrawLiquidityQuery<TData>) {
  return useQuery<ArrayOfCoin, Error, TData>(
    marsCreditManagerQueryKeys.estimateWithdrawLiquidity(client?.contractAddress, args),
    () =>
      client
        ? client.estimateWithdrawLiquidity({
            lpToken: args.lpToken,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerEstimateProvideLiquidityQuery<TData>
  extends MarsCreditManagerReactQuery<Uint128, TData> {
  args: {
    coinsIn: Coin[]
    lpTokenOut: string
  }
}
export function useMarsCreditManagerEstimateProvideLiquidityQuery<TData = Uint128>({
  client,
  args,
  options,
}: MarsCreditManagerEstimateProvideLiquidityQuery<TData>) {
  return useQuery<Uint128, Error, TData>(
    marsCreditManagerQueryKeys.estimateProvideLiquidity(client?.contractAddress, args),
    () =>
      client
        ? client.estimateProvideLiquidity({
            coinsIn: args.coinsIn,
            lpTokenOut: args.lpTokenOut,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerAllTotalVaultCoinBalancesQuery<TData>
  extends MarsCreditManagerReactQuery<ArrayOfVaultWithBalance, TData> {
  args: {
    limit?: number
    startAfter?: VaultBaseForString
  }
}
export function useMarsCreditManagerAllTotalVaultCoinBalancesQuery<
  TData = ArrayOfVaultWithBalance,
>({ client, args, options }: MarsCreditManagerAllTotalVaultCoinBalancesQuery<TData>) {
  return useQuery<ArrayOfVaultWithBalance, Error, TData>(
    marsCreditManagerQueryKeys.allTotalVaultCoinBalances(client?.contractAddress, args),
    () =>
      client
        ? client.allTotalVaultCoinBalances({
            limit: args.limit,
            startAfter: args.startAfter,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerTotalVaultCoinBalanceQuery<TData>
  extends MarsCreditManagerReactQuery<Uint128, TData> {
  args: {
    vault: VaultBaseForString
  }
}
export function useMarsCreditManagerTotalVaultCoinBalanceQuery<TData = Uint128>({
  client,
  args,
  options,
}: MarsCreditManagerTotalVaultCoinBalanceQuery<TData>) {
  return useQuery<Uint128, Error, TData>(
    marsCreditManagerQueryKeys.totalVaultCoinBalance(client?.contractAddress, args),
    () =>
      client
        ? client.totalVaultCoinBalance({
            vault: args.vault,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerAllVaultPositionsQuery<TData>
  extends MarsCreditManagerReactQuery<ArrayOfVaultPositionResponseItem, TData> {
  args: {
    limit?: number
    startAfter?: string[][]
  }
}
export function useMarsCreditManagerAllVaultPositionsQuery<
  TData = ArrayOfVaultPositionResponseItem,
>({ client, args, options }: MarsCreditManagerAllVaultPositionsQuery<TData>) {
  return useQuery<ArrayOfVaultPositionResponseItem, Error, TData>(
    marsCreditManagerQueryKeys.allVaultPositions(client?.contractAddress, args),
    () =>
      client
        ? client.allVaultPositions({
            limit: args.limit,
            startAfter: args.startAfter,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerAllTotalDebtSharesQuery<TData>
  extends MarsCreditManagerReactQuery<ArrayOfDebtShares, TData> {
  args: {
    limit?: number
    startAfter?: string
  }
}
export function useMarsCreditManagerAllTotalDebtSharesQuery<TData = ArrayOfDebtShares>({
  client,
  args,
  options,
}: MarsCreditManagerAllTotalDebtSharesQuery<TData>) {
  return useQuery<ArrayOfDebtShares, Error, TData>(
    marsCreditManagerQueryKeys.allTotalDebtShares(client?.contractAddress, args),
    () =>
      client
        ? client.allTotalDebtShares({
            limit: args.limit,
            startAfter: args.startAfter,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerTotalDebtSharesQuery<TData>
  extends MarsCreditManagerReactQuery<DebtShares, TData> {}
export function useMarsCreditManagerTotalDebtSharesQuery<TData = DebtShares>({
  client,
  options,
}: MarsCreditManagerTotalDebtSharesQuery<TData>) {
  return useQuery<DebtShares, Error, TData>(
    marsCreditManagerQueryKeys.totalDebtShares(client?.contractAddress),
    () => (client ? client.totalDebtShares() : Promise.reject(new Error('Invalid client'))),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerAllDebtSharesQuery<TData>
  extends MarsCreditManagerReactQuery<ArrayOfSharesResponseItem, TData> {
  args: {
    limit?: number
    startAfter?: string[][]
  }
}
export function useMarsCreditManagerAllDebtSharesQuery<TData = ArrayOfSharesResponseItem>({
  client,
  args,
  options,
}: MarsCreditManagerAllDebtSharesQuery<TData>) {
  return useQuery<ArrayOfSharesResponseItem, Error, TData>(
    marsCreditManagerQueryKeys.allDebtShares(client?.contractAddress, args),
    () =>
      client
        ? client.allDebtShares({
            limit: args.limit,
            startAfter: args.startAfter,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerAllCoinBalancesQuery<TData>
  extends MarsCreditManagerReactQuery<ArrayOfCoinBalanceResponseItem, TData> {
  args: {
    limit?: number
    startAfter?: string[][]
  }
}
export function useMarsCreditManagerAllCoinBalancesQuery<TData = ArrayOfCoinBalanceResponseItem>({
  client,
  args,
  options,
}: MarsCreditManagerAllCoinBalancesQuery<TData>) {
  return useQuery<ArrayOfCoinBalanceResponseItem, Error, TData>(
    marsCreditManagerQueryKeys.allCoinBalances(client?.contractAddress, args),
    () =>
      client
        ? client.allCoinBalances({
            limit: args.limit,
            startAfter: args.startAfter,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerHealthQuery<TData>
  extends MarsCreditManagerReactQuery<HealthResponse, TData> {
  args: {
    accountId: string
  }
}
export function useMarsCreditManagerHealthQuery<TData = HealthResponse>({
  client,
  args,
  options,
}: MarsCreditManagerHealthQuery<TData>) {
  return useQuery<HealthResponse, Error, TData>(
    marsCreditManagerQueryKeys.health(client?.contractAddress, args),
    () =>
      client
        ? client.health({
            accountId: args.accountId,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerPositionsQuery<TData>
  extends MarsCreditManagerReactQuery<Positions, TData> {
  args: {
    accountId: string
  }
}
export function useMarsCreditManagerPositionsQuery<TData = Positions>({
  client,
  args,
  options,
}: MarsCreditManagerPositionsQuery<TData>) {
  return useQuery<Positions, Error, TData>(
    marsCreditManagerQueryKeys.positions(client?.contractAddress, args),
    () =>
      client
        ? client.positions({
            accountId: args.accountId,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerAllowedCoinsQuery<TData>
  extends MarsCreditManagerReactQuery<ArrayOfString, TData> {
  args: {
    limit?: number
    startAfter?: string
  }
}
export function useMarsCreditManagerAllowedCoinsQuery<TData = ArrayOfString>({
  client,
  args,
  options,
}: MarsCreditManagerAllowedCoinsQuery<TData>) {
  return useQuery<ArrayOfString, Error, TData>(
    marsCreditManagerQueryKeys.allowedCoins(client?.contractAddress, args),
    () =>
      client
        ? client.allowedCoins({
            limit: args.limit,
            startAfter: args.startAfter,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerVaultConfigsQuery<TData>
  extends MarsCreditManagerReactQuery<ArrayOfVaultInstantiateConfig, TData> {
  args: {
    limit?: number
    startAfter?: VaultBaseForString
  }
}
export function useMarsCreditManagerVaultConfigsQuery<TData = ArrayOfVaultInstantiateConfig>({
  client,
  args,
  options,
}: MarsCreditManagerVaultConfigsQuery<TData>) {
  return useQuery<ArrayOfVaultInstantiateConfig, Error, TData>(
    marsCreditManagerQueryKeys.vaultConfigs(client?.contractAddress, args),
    () =>
      client
        ? client.vaultConfigs({
            limit: args.limit,
            startAfter: args.startAfter,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerConfigQuery<TData>
  extends MarsCreditManagerReactQuery<ConfigResponse, TData> {}
export function useMarsCreditManagerConfigQuery<TData = ConfigResponse>({
  client,
  options,
}: MarsCreditManagerConfigQuery<TData>) {
  return useQuery<ConfigResponse, Error, TData>(
    marsCreditManagerQueryKeys.config(client?.contractAddress),
    () => (client ? client.config() : Promise.reject(new Error('Invalid client'))),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerCallbackMutation {
  client: MarsCreditManagerClient
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsCreditManagerCallbackMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsCreditManagerCallbackMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsCreditManagerCallbackMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) => client.callback(msg, fee, memo, funds),
    options,
  )
}
export interface MarsCreditManagerUpdateAdminMutation {
  client: MarsCreditManagerClient
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsCreditManagerUpdateAdminMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsCreditManagerUpdateAdminMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsCreditManagerUpdateAdminMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) => client.updateAdmin(msg, fee, memo, funds),
    options,
  )
}
export interface MarsCreditManagerUpdateConfigMutation {
  client: MarsCreditManagerClient
  msg: {
    newConfig: ConfigUpdates
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsCreditManagerUpdateConfigMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsCreditManagerUpdateConfigMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsCreditManagerUpdateConfigMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) =>
      client.updateConfig(msg, fee, memo, funds),
    options,
  )
}
export interface MarsCreditManagerUpdateCreditAccountMutation {
  client: MarsCreditManagerClient
  msg: {
    accountId: string
    actions: Action[]
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsCreditManagerUpdateCreditAccountMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsCreditManagerUpdateCreditAccountMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsCreditManagerUpdateCreditAccountMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) =>
      client.updateCreditAccount(msg, fee, memo, funds),
    options,
  )
}
export interface MarsCreditManagerCreateCreditAccountMutation {
  client: MarsCreditManagerClient
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsCreditManagerCreateCreditAccountMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsCreditManagerCreateCreditAccountMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsCreditManagerCreateCreditAccountMutation>(
    ({ client, args: { fee, memo, funds } = {} }) => client.createCreditAccount(fee, memo, funds),
    options,
  )
}
