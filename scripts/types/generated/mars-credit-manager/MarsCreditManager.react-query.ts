// @ts-nocheck
/**
 * This file was automatically generated by @cosmwasm/ts-codegen@0.35.3.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run the @cosmwasm/ts-codegen generate command to regenerate this file.
 */

import { UseQueryOptions, useQuery, useMutation, UseMutationOptions } from '@tanstack/react-query'
import { ExecuteResult } from '@cosmjs/cosmwasm-stargate'
import { StdFee } from '@cosmjs/amino'
import {
  HealthContractBaseForString,
  IncentivesUnchecked,
  Decimal,
  Uint128,
  OracleBaseForString,
  ParamsBaseForString,
  RedBankUnchecked,
  SwapperBaseForString,
  ZapperBaseForString,
  InstantiateMsg,
  ExecuteMsg,
  AccountKind,
  Action,
  ActionAmount,
  LiquidateRequestForVaultBaseForString,
  VaultPositionType,
  SwapperRoute,
  OsmosisRoute,
  AccountNftBaseForString,
  OwnerUpdate,
  Action2,
  Expiration,
  Timestamp,
  Uint64,
  CallbackMsg,
  Addr,
  HealthState,
  LiquidateRequestForVaultBaseForAddr,
  ChangeExpected,
  Coin,
  ActionCoin,
  VaultBaseForString,
  AstroportRoute,
  SwapOperation,
  SwapAmountInRoute,
  ConfigUpdates,
  NftConfigUpdates,
  VaultBaseForAddr,
  QueryMsg,
  VaultPositionAmount,
  VaultAmount,
  VaultAmount1,
  UnlockingPositions,
  VaultPosition,
  LockingVaultAmount,
  VaultUnlockingPosition,
  ArrayOfAccount,
  Account,
  ArrayOfCoinBalanceResponseItem,
  CoinBalanceResponseItem,
  ArrayOfSharesResponseItem,
  SharesResponseItem,
  ArrayOfDebtShares,
  DebtShares,
  ArrayOfVaultPositionResponseItem,
  VaultPositionResponseItem,
  ConfigResponse,
  OwnerResponse,
  RewardsCollector,
  ArrayOfCoin,
  Positions,
  DebtAmount,
  VaultPositionValue,
  CoinValue,
  VaultUtilizationResponse,
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
  accountKind: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsCreditManagerQueryKeys.address(contractAddress)[0], method: 'account_kind', args },
    ] as const,
  accounts: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsCreditManagerQueryKeys.address(contractAddress)[0], method: 'accounts', args },
    ] as const,
  config: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsCreditManagerQueryKeys.address(contractAddress)[0], method: 'config', args },
    ] as const,
  vaultUtilization: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      {
        ...marsCreditManagerQueryKeys.address(contractAddress)[0],
        method: 'vault_utilization',
        args,
      },
    ] as const,
  positions: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsCreditManagerQueryKeys.address(contractAddress)[0], method: 'positions', args },
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
  vaultPositionValue: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      {
        ...marsCreditManagerQueryKeys.address(contractAddress)[0],
        method: 'vault_position_value',
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
export interface MarsCreditManagerVaultPositionValueQuery<TData>
  extends MarsCreditManagerReactQuery<VaultPositionValue, TData> {
  args: {
    vaultPosition: VaultPosition
  }
}
export function useMarsCreditManagerVaultPositionValueQuery<TData = VaultPositionValue>({
  client,
  args,
  options,
}: MarsCreditManagerVaultPositionValueQuery<TData>) {
  return useQuery<VaultPositionValue, Error, TData>(
    marsCreditManagerQueryKeys.vaultPositionValue(client?.contractAddress, args),
    () =>
      client
        ? client.vaultPositionValue({
            vaultPosition: args.vaultPosition,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
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
export interface MarsCreditManagerVaultUtilizationQuery<TData>
  extends MarsCreditManagerReactQuery<VaultUtilizationResponse, TData> {
  args: {
    vault: VaultBaseForString
  }
}
export function useMarsCreditManagerVaultUtilizationQuery<TData = VaultUtilizationResponse>({
  client,
  args,
  options,
}: MarsCreditManagerVaultUtilizationQuery<TData>) {
  return useQuery<VaultUtilizationResponse, Error, TData>(
    marsCreditManagerQueryKeys.vaultUtilization(client?.contractAddress, args),
    () =>
      client
        ? client.vaultUtilization({
            vault: args.vault,
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
export interface MarsCreditManagerAccountsQuery<TData>
  extends MarsCreditManagerReactQuery<ArrayOfAccount, TData> {
  args: {
    limit?: number
    owner: string
    startAfter?: string
  }
}
export function useMarsCreditManagerAccountsQuery<TData = ArrayOfAccount>({
  client,
  args,
  options,
}: MarsCreditManagerAccountsQuery<TData>) {
  return useQuery<ArrayOfAccount, Error, TData>(
    marsCreditManagerQueryKeys.accounts(client?.contractAddress, args),
    () =>
      client
        ? client.accounts({
            limit: args.limit,
            owner: args.owner,
            startAfter: args.startAfter,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerAccountKindQuery<TData>
  extends MarsCreditManagerReactQuery<AccountKind, TData> {
  args: {
    accountId: string
  }
}
export function useMarsCreditManagerAccountKindQuery<TData = AccountKind>({
  client,
  args,
  options,
}: MarsCreditManagerAccountKindQuery<TData>) {
  return useQuery<AccountKind, Error, TData>(
    marsCreditManagerQueryKeys.accountKind(client?.contractAddress, args),
    () =>
      client
        ? client.accountKind({
            accountId: args.accountId,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsCreditManagerCallbackMutation {
  client: MarsCreditManagerClient
  msg: CallbackMsg
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
export interface MarsCreditManagerUpdateNftConfigMutation {
  client: MarsCreditManagerClient
  msg: {
    config?: NftConfigUpdates
    ownership?: Action2
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsCreditManagerUpdateNftConfigMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsCreditManagerUpdateNftConfigMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsCreditManagerUpdateNftConfigMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) =>
      client.updateNftConfig(msg, fee, memo, funds),
    options,
  )
}
export interface MarsCreditManagerUpdateOwnerMutation {
  client: MarsCreditManagerClient
  msg: OwnerUpdate
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsCreditManagerUpdateOwnerMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsCreditManagerUpdateOwnerMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsCreditManagerUpdateOwnerMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) => client.updateOwner(msg, fee, memo, funds),
    options,
  )
}
export interface MarsCreditManagerUpdateConfigMutation {
  client: MarsCreditManagerClient
  msg: {
    updates: ConfigUpdates
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
export interface MarsCreditManagerRepayFromWalletMutation {
  client: MarsCreditManagerClient
  msg: {
    accountId: string
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsCreditManagerRepayFromWalletMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsCreditManagerRepayFromWalletMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsCreditManagerRepayFromWalletMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) =>
      client.repayFromWallet(msg, fee, memo, funds),
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
    ({ client, msg, args: { fee, memo, funds } = {} }) =>
      client.createCreditAccount(msg, fee, memo, funds),
    options,
  )
}
