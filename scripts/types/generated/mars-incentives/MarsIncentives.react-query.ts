// @ts-nocheck
/**
 * This file was automatically generated by @cosmwasm/ts-codegen@0.30.0.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run the @cosmwasm/ts-codegen generate command to regenerate this file.
 */

import { UseQueryOptions, useQuery, useMutation, UseMutationOptions } from '@tanstack/react-query'
import { ExecuteResult } from '@cosmjs/cosmwasm-stargate'
import { StdFee } from '@cosmjs/amino'
import {
  InstantiateMsg,
  ExecuteMsg,
  Uint128,
  Addr,
  OwnerUpdate,
  WhitelistEntry,
  QueryMsg,
  ArrayOfActiveEmission,
  ActiveEmission,
  ConfigResponse,
  ArrayOfEmissionResponse,
  EmissionResponse,
  Decimal,
  IncentiveStateResponse,
  ArrayOfIncentiveStateResponse,
  ArrayOfCoin,
  Coin,
  ArrayOfWhitelistEntry,
} from './MarsIncentives.types'
import { MarsIncentivesQueryClient, MarsIncentivesClient } from './MarsIncentives.client'
export const marsIncentivesQueryKeys = {
  contract: [
    {
      contract: 'marsIncentives',
    },
  ] as const,
  address: (contractAddress: string | undefined) =>
    [{ ...marsIncentivesQueryKeys.contract[0], address: contractAddress }] as const,
  activeEmissions: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsIncentivesQueryKeys.address(contractAddress)[0], method: 'active_emissions', args },
    ] as const,
  config: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [{ ...marsIncentivesQueryKeys.address(contractAddress)[0], method: 'config', args }] as const,
  incentiveState: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsIncentivesQueryKeys.address(contractAddress)[0], method: 'incentive_state', args },
    ] as const,
  incentiveStates: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsIncentivesQueryKeys.address(contractAddress)[0], method: 'incentive_states', args },
    ] as const,
  emission: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [{ ...marsIncentivesQueryKeys.address(contractAddress)[0], method: 'emission', args }] as const,
  emissions: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsIncentivesQueryKeys.address(contractAddress)[0], method: 'emissions', args },
    ] as const,
  userUnclaimedRewards: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      {
        ...marsIncentivesQueryKeys.address(contractAddress)[0],
        method: 'user_unclaimed_rewards',
        args,
      },
    ] as const,
  whitelist: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsIncentivesQueryKeys.address(contractAddress)[0], method: 'whitelist', args },
    ] as const,
}
export interface MarsIncentivesReactQuery<TResponse, TData = TResponse> {
  client: MarsIncentivesQueryClient | undefined
  options?: Omit<
    UseQueryOptions<TResponse, Error, TData>,
    "'queryKey' | 'queryFn' | 'initialData'"
  > & {
    initialData?: undefined
  }
}
export interface MarsIncentivesWhitelistQuery<TData>
  extends MarsIncentivesReactQuery<ArrayOfWhitelistEntry, TData> {}
export function useMarsIncentivesWhitelistQuery<TData = ArrayOfWhitelistEntry>({
  client,
  options,
}: MarsIncentivesWhitelistQuery<TData>) {
  return useQuery<ArrayOfWhitelistEntry, Error, TData>(
    marsIncentivesQueryKeys.whitelist(client?.contractAddress),
    () => (client ? client.whitelist() : Promise.reject(new Error('Invalid client'))),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsIncentivesUserUnclaimedRewardsQuery<TData>
  extends MarsIncentivesReactQuery<ArrayOfCoin, TData> {
  args: {
    accountId?: string
    limit?: number
    startAfterCollateralDenom?: string
    startAfterIncentiveDenom?: string
    user: string
  }
}
export function useMarsIncentivesUserUnclaimedRewardsQuery<TData = ArrayOfCoin>({
  client,
  args,
  options,
}: MarsIncentivesUserUnclaimedRewardsQuery<TData>) {
  return useQuery<ArrayOfCoin, Error, TData>(
    marsIncentivesQueryKeys.userUnclaimedRewards(client?.contractAddress, args),
    () =>
      client
        ? client.userUnclaimedRewards({
            accountId: args.accountId,
            limit: args.limit,
            startAfterCollateralDenom: args.startAfterCollateralDenom,
            startAfterIncentiveDenom: args.startAfterIncentiveDenom,
            user: args.user,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsIncentivesEmissionsQuery<TData>
  extends MarsIncentivesReactQuery<ArrayOfEmissionResponse, TData> {
  args: {
    collateralDenom: string
    incentiveDenom: string
    limit?: number
    startAfterTimestamp?: number
  }
}
export function useMarsIncentivesEmissionsQuery<TData = ArrayOfEmissionResponse>({
  client,
  args,
  options,
}: MarsIncentivesEmissionsQuery<TData>) {
  return useQuery<ArrayOfEmissionResponse, Error, TData>(
    marsIncentivesQueryKeys.emissions(client?.contractAddress, args),
    () =>
      client
        ? client.emissions({
            collateralDenom: args.collateralDenom,
            incentiveDenom: args.incentiveDenom,
            limit: args.limit,
            startAfterTimestamp: args.startAfterTimestamp,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsIncentivesEmissionQuery<TData>
  extends MarsIncentivesReactQuery<Uint128, TData> {
  args: {
    collateralDenom: string
    incentiveDenom: string
    timestamp: number
  }
}
export function useMarsIncentivesEmissionQuery<TData = Uint128>({
  client,
  args,
  options,
}: MarsIncentivesEmissionQuery<TData>) {
  return useQuery<Uint128, Error, TData>(
    marsIncentivesQueryKeys.emission(client?.contractAddress, args),
    () =>
      client
        ? client.emission({
            collateralDenom: args.collateralDenom,
            incentiveDenom: args.incentiveDenom,
            timestamp: args.timestamp,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsIncentivesIncentiveStatesQuery<TData>
  extends MarsIncentivesReactQuery<ArrayOfIncentiveStateResponse, TData> {
  args: {
    limit?: number
    startAfterCollateralDenom?: string
    startAfterIncentiveDenom?: string
  }
}
export function useMarsIncentivesIncentiveStatesQuery<TData = ArrayOfIncentiveStateResponse>({
  client,
  args,
  options,
}: MarsIncentivesIncentiveStatesQuery<TData>) {
  return useQuery<ArrayOfIncentiveStateResponse, Error, TData>(
    marsIncentivesQueryKeys.incentiveStates(client?.contractAddress, args),
    () =>
      client
        ? client.incentiveStates({
            limit: args.limit,
            startAfterCollateralDenom: args.startAfterCollateralDenom,
            startAfterIncentiveDenom: args.startAfterIncentiveDenom,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsIncentivesIncentiveStateQuery<TData>
  extends MarsIncentivesReactQuery<IncentiveStateResponse, TData> {
  args: {
    collateralDenom: string
    incentiveDenom: string
  }
}
export function useMarsIncentivesIncentiveStateQuery<TData = IncentiveStateResponse>({
  client,
  args,
  options,
}: MarsIncentivesIncentiveStateQuery<TData>) {
  return useQuery<IncentiveStateResponse, Error, TData>(
    marsIncentivesQueryKeys.incentiveState(client?.contractAddress, args),
    () =>
      client
        ? client.incentiveState({
            collateralDenom: args.collateralDenom,
            incentiveDenom: args.incentiveDenom,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsIncentivesConfigQuery<TData>
  extends MarsIncentivesReactQuery<ConfigResponse, TData> {}
export function useMarsIncentivesConfigQuery<TData = ConfigResponse>({
  client,
  options,
}: MarsIncentivesConfigQuery<TData>) {
  return useQuery<ConfigResponse, Error, TData>(
    marsIncentivesQueryKeys.config(client?.contractAddress),
    () => (client ? client.config() : Promise.reject(new Error('Invalid client'))),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsIncentivesActiveEmissionsQuery<TData>
  extends MarsIncentivesReactQuery<ArrayOfActiveEmission, TData> {
  args: {
    collateralDenom: string
  }
}
export function useMarsIncentivesActiveEmissionsQuery<TData = ArrayOfActiveEmission>({
  client,
  args,
  options,
}: MarsIncentivesActiveEmissionsQuery<TData>) {
  return useQuery<ArrayOfActiveEmission, Error, TData>(
    marsIncentivesQueryKeys.activeEmissions(client?.contractAddress, args),
    () =>
      client
        ? client.activeEmissions({
            collateralDenom: args.collateralDenom,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsIncentivesUpdateOwnerMutation {
  client: MarsIncentivesClient
  msg: OwnerUpdate
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsIncentivesUpdateOwnerMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsIncentivesUpdateOwnerMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsIncentivesUpdateOwnerMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) => client.updateOwner(msg, fee, memo, funds),
    options,
  )
}
export interface MarsIncentivesUpdateConfigMutation {
  client: MarsIncentivesClient
  msg: {
    addressProvider?: string
    maxWhitelistedDenoms?: number
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsIncentivesUpdateConfigMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsIncentivesUpdateConfigMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsIncentivesUpdateConfigMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) =>
      client.updateConfig(msg, fee, memo, funds),
    options,
  )
}
export interface MarsIncentivesClaimRewardsMutation {
  client: MarsIncentivesClient
  msg: {
    accountId?: string
    limit?: number
    startAfterCollateralDenom?: string
    startAfterIncentiveDenom?: string
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsIncentivesClaimRewardsMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsIncentivesClaimRewardsMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsIncentivesClaimRewardsMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) =>
      client.claimRewards(msg, fee, memo, funds),
    options,
  )
}
export interface MarsIncentivesBalanceChangeMutation {
  client: MarsIncentivesClient
  msg: {
    accountId?: string
    denom: string
    totalAmountScaledBefore: Uint128
    userAddr: Addr
    userAmountScaledBefore: Uint128
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsIncentivesBalanceChangeMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsIncentivesBalanceChangeMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsIncentivesBalanceChangeMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) =>
      client.balanceChange(msg, fee, memo, funds),
    options,
  )
}
export interface MarsIncentivesSetAssetIncentiveMutation {
  client: MarsIncentivesClient
  msg: {
    collateralDenom: string
    duration: number
    emissionPerSecond: Uint128
    incentiveDenom: string
    startTime: number
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsIncentivesSetAssetIncentiveMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsIncentivesSetAssetIncentiveMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsIncentivesSetAssetIncentiveMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) =>
      client.setAssetIncentive(msg, fee, memo, funds),
    options,
  )
}
export interface MarsIncentivesUpdateWhitelistMutation {
  client: MarsIncentivesClient
  msg: {
    addDenoms: WhitelistEntry[]
    removeDenoms: string[]
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsIncentivesUpdateWhitelistMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsIncentivesUpdateWhitelistMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsIncentivesUpdateWhitelistMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) =>
      client.updateWhitelist(msg, fee, memo, funds),
    options,
  )
}
