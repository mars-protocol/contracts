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
  OwnerUpdate,
  SwapOperation,
  AssetInfo,
  Addr,
  Uint128,
  Decimal,
  AstroportRoute,
  Coin,
  QueryMsg,
  EstimateExactInSwapResponse,
  OwnerResponse,
  RouteResponseForEmpty,
  Empty,
  ArrayOfRouteResponseForEmpty,
} from './MarsSwapperAstroport.types'
import {
  MarsSwapperAstroportQueryClient,
  MarsSwapperAstroportClient,
} from './MarsSwapperAstroport.client'
export const marsSwapperAstroportQueryKeys = {
  contract: [
    {
      contract: 'marsSwapperAstroport',
    },
  ] as const,
  address: (contractAddress: string | undefined) =>
    [{ ...marsSwapperAstroportQueryKeys.contract[0], address: contractAddress }] as const,
  owner: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsSwapperAstroportQueryKeys.address(contractAddress)[0], method: 'owner', args },
    ] as const,
  route: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsSwapperAstroportQueryKeys.address(contractAddress)[0], method: 'route', args },
    ] as const,
  routes: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsSwapperAstroportQueryKeys.address(contractAddress)[0], method: 'routes', args },
    ] as const,
  estimateExactInSwap: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      {
        ...marsSwapperAstroportQueryKeys.address(contractAddress)[0],
        method: 'estimate_exact_in_swap',
        args,
      },
    ] as const,
}
export interface MarsSwapperAstroportReactQuery<TResponse, TData = TResponse> {
  client: MarsSwapperAstroportQueryClient | undefined
  options?: Omit<
    UseQueryOptions<TResponse, Error, TData>,
    "'queryKey' | 'queryFn' | 'initialData'"
  > & {
    initialData?: undefined
  }
}
export interface MarsSwapperAstroportEstimateExactInSwapQuery<TData>
  extends MarsSwapperAstroportReactQuery<EstimateExactInSwapResponse, TData> {
  args: {
    coinIn: Coin
    denomOut: string
  }
}
export function useMarsSwapperAstroportEstimateExactInSwapQuery<
  TData = EstimateExactInSwapResponse,
>({ client, args, options }: MarsSwapperAstroportEstimateExactInSwapQuery<TData>) {
  return useQuery<EstimateExactInSwapResponse, Error, TData>(
    marsSwapperAstroportQueryKeys.estimateExactInSwap(client?.contractAddress, args),
    () =>
      client
        ? client.estimateExactInSwap({
            coinIn: args.coinIn,
            denomOut: args.denomOut,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsSwapperAstroportRoutesQuery<TData>
  extends MarsSwapperAstroportReactQuery<ArrayOfRouteResponseForEmpty, TData> {
  args: {
    limit?: number
    startAfter?: string[][]
  }
}
export function useMarsSwapperAstroportRoutesQuery<TData = ArrayOfRouteResponseForEmpty>({
  client,
  args,
  options,
}: MarsSwapperAstroportRoutesQuery<TData>) {
  return useQuery<ArrayOfRouteResponseForEmpty, Error, TData>(
    marsSwapperAstroportQueryKeys.routes(client?.contractAddress, args),
    () =>
      client
        ? client.routes({
            limit: args.limit,
            startAfter: args.startAfter,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsSwapperAstroportRouteQuery<TData>
  extends MarsSwapperAstroportReactQuery<RouteResponseForEmpty, TData> {
  args: {
    denomIn: string
    denomOut: string
  }
}
export function useMarsSwapperAstroportRouteQuery<TData = RouteResponseForEmpty>({
  client,
  args,
  options,
}: MarsSwapperAstroportRouteQuery<TData>) {
  return useQuery<RouteResponseForEmpty, Error, TData>(
    marsSwapperAstroportQueryKeys.route(client?.contractAddress, args),
    () =>
      client
        ? client.route({
            denomIn: args.denomIn,
            denomOut: args.denomOut,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsSwapperAstroportOwnerQuery<TData>
  extends MarsSwapperAstroportReactQuery<OwnerResponse, TData> {}
export function useMarsSwapperAstroportOwnerQuery<TData = OwnerResponse>({
  client,
  options,
}: MarsSwapperAstroportOwnerQuery<TData>) {
  return useQuery<OwnerResponse, Error, TData>(
    marsSwapperAstroportQueryKeys.owner(client?.contractAddress),
    () => (client ? client.owner() : Promise.reject(new Error('Invalid client'))),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsSwapperAstroportTransferResultMutation {
  client: MarsSwapperAstroportClient
  msg: {
    denomIn: string
    denomOut: string
    recipient: Addr
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsSwapperAstroportTransferResultMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsSwapperAstroportTransferResultMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsSwapperAstroportTransferResultMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) =>
      client.transferResult(msg, fee, memo, funds),
    options,
  )
}
export interface MarsSwapperAstroportSwapExactInMutation {
  client: MarsSwapperAstroportClient
  msg: {
    coinIn: Coin
    denomOut: string
    slippage: Decimal
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsSwapperAstroportSwapExactInMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsSwapperAstroportSwapExactInMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsSwapperAstroportSwapExactInMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) => client.swapExactIn(msg, fee, memo, funds),
    options,
  )
}
export interface MarsSwapperAstroportSetRouteMutation {
  client: MarsSwapperAstroportClient
  msg: {
    denomIn: string
    denomOut: string
    route: AstroportRoute
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsSwapperAstroportSetRouteMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsSwapperAstroportSetRouteMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsSwapperAstroportSetRouteMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) => client.setRoute(msg, fee, memo, funds),
    options,
  )
}
export interface MarsSwapperAstroportUpdateOwnerMutation {
  client: MarsSwapperAstroportClient
  msg: OwnerUpdate
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsSwapperAstroportUpdateOwnerMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsSwapperAstroportUpdateOwnerMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsSwapperAstroportUpdateOwnerMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) => client.updateOwner(msg, fee, memo, funds),
    options,
  )
}
