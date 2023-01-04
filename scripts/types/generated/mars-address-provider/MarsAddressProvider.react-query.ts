// @ts-nocheck
/**
 * This file was automatically generated by @cosmwasm/ts-codegen@0.16.5.
 * DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
 * and run the @cosmwasm/ts-codegen generate command to regenerate this file.
 */

import { UseQueryOptions, useQuery, useMutation, UseMutationOptions } from '@tanstack/react-query'
import { ExecuteResult } from '@cosmjs/cosmwasm-stargate'
import { StdFee, Coin } from '@cosmjs/amino'
import {
  InstantiateMsg,
  ExecuteMsg,
  MarsAddressType,
  OwnerUpdate,
  QueryMsg,
  AddressResponseItem,
  ArrayOfAddressResponseItem,
  ConfigResponse,
} from './MarsAddressProvider.types'
import {
  MarsAddressProviderQueryClient,
  MarsAddressProviderClient,
} from './MarsAddressProvider.client'
export const marsAddressProviderQueryKeys = {
  contract: [
    {
      contract: 'marsAddressProvider',
    },
  ] as const,
  address: (contractAddress: string | undefined) =>
    [{ ...marsAddressProviderQueryKeys.contract[0], address: contractAddress }] as const,
  config: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsAddressProviderQueryKeys.address(contractAddress)[0], method: 'config', args },
    ] as const,
  address: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsAddressProviderQueryKeys.address(contractAddress)[0], method: 'address', args },
    ] as const,
  addresses: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      { ...marsAddressProviderQueryKeys.address(contractAddress)[0], method: 'addresses', args },
    ] as const,
  allAddresses: (contractAddress: string | undefined, args?: Record<string, unknown>) =>
    [
      {
        ...marsAddressProviderQueryKeys.address(contractAddress)[0],
        method: 'all_addresses',
        args,
      },
    ] as const,
}
export interface MarsAddressProviderReactQuery<TResponse, TData = TResponse> {
  client: MarsAddressProviderQueryClient | undefined
  options?: Omit<
    UseQueryOptions<TResponse, Error, TData>,
    "'queryKey' | 'queryFn' | 'initialData'"
  > & {
    initialData?: undefined
  }
}
export interface MarsAddressProviderAllAddressesQuery<TData>
  extends MarsAddressProviderReactQuery<ArrayOfAddressResponseItem, TData> {
  args: {
    limit?: number
    startAfter?: MarsAddressType
  }
}
export function useMarsAddressProviderAllAddressesQuery<TData = ArrayOfAddressResponseItem>({
  client,
  args,
  options,
}: MarsAddressProviderAllAddressesQuery<TData>) {
  return useQuery<ArrayOfAddressResponseItem, Error, TData>(
    marsAddressProviderQueryKeys.allAddresses(client?.contractAddress, args),
    () =>
      client
        ? client.allAddresses({
            limit: args.limit,
            startAfter: args.startAfter,
          })
        : Promise.reject(new Error('Invalid client')),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsAddressProviderAddressesQuery<TData>
  extends MarsAddressProviderReactQuery<ArrayOfAddressResponseItem, TData> {}
export function useMarsAddressProviderAddressesQuery<TData = ArrayOfAddressResponseItem>({
  client,
  options,
}: MarsAddressProviderAddressesQuery<TData>) {
  return useQuery<ArrayOfAddressResponseItem, Error, TData>(
    marsAddressProviderQueryKeys.addresses(client?.contractAddress),
    () => (client ? client.addresses() : Promise.reject(new Error('Invalid client'))),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsAddressProviderAddressQuery<TData>
  extends MarsAddressProviderReactQuery<AddressResponseItem, TData> {}
export function useMarsAddressProviderAddressQuery<TData = AddressResponseItem>({
  client,
  options,
}: MarsAddressProviderAddressQuery<TData>) {
  return useQuery<AddressResponseItem, Error, TData>(
    marsAddressProviderQueryKeys.address(client?.contractAddress),
    () => (client ? client.address() : Promise.reject(new Error('Invalid client'))),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsAddressProviderConfigQuery<TData>
  extends MarsAddressProviderReactQuery<ConfigResponse, TData> {}
export function useMarsAddressProviderConfigQuery<TData = ConfigResponse>({
  client,
  options,
}: MarsAddressProviderConfigQuery<TData>) {
  return useQuery<ConfigResponse, Error, TData>(
    marsAddressProviderQueryKeys.config(client?.contractAddress),
    () => (client ? client.config() : Promise.reject(new Error('Invalid client'))),
    { ...options, enabled: !!client && (options?.enabled != undefined ? options.enabled : true) },
  )
}
export interface MarsAddressProviderUpdateOwnerMutation {
  client: MarsAddressProviderClient
  msg: OwnerUpdate
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsAddressProviderUpdateOwnerMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsAddressProviderUpdateOwnerMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsAddressProviderUpdateOwnerMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) => client.updateOwner(msg, fee, memo, funds),
    options,
  )
}
export interface MarsAddressProviderSetAddressMutation {
  client: MarsAddressProviderClient
  msg: {
    address: string
    addressType: MarsAddressType
  }
  args?: {
    fee?: number | StdFee | 'auto'
    memo?: string
    funds?: Coin[]
  }
}
export function useMarsAddressProviderSetAddressMutation(
  options?: Omit<
    UseMutationOptions<ExecuteResult, Error, MarsAddressProviderSetAddressMutation>,
    'mutationFn'
  >,
) {
  return useMutation<ExecuteResult, Error, MarsAddressProviderSetAddressMutation>(
    ({ client, msg, args: { fee, memo, funds } = {} }) => client.setAddress(msg, fee, memo, funds),
    options,
  )
}
