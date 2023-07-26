/* tslint:disable */
/* eslint-disable */
/**
 * @param {HealthComputer} c
 * @returns {HealthValuesResponse}
 */
export function compute_health_js(c: HealthComputer): HealthValuesResponse
/**
 * @param {HealthComputer} c
 * @param {string} withdraw_denom
 * @returns {string}
 */
export function max_withdraw_estimate_js(c: HealthComputer, withdraw_denom: string): string
/**
 * @param {HealthComputer} c
 * @param {string} borrow_denom
 * @param {BorrowTarget} target
 * @returns {string}
 */
export function max_borrow_estimate_js(
  c: HealthComputer,
  borrow_denom: string,
  target: BorrowTarget,
): string
export interface HealthComputer {
  kind: AccountKind
  positions: Positions
  denoms_data: DenomsData
  vaults_data: VaultsData
}

export interface HealthValuesResponse {
  total_debt_value: Uint128
  total_collateral_value: Uint128
  max_ltv_adjusted_collateral: Uint128
  liquidation_threshold_adjusted_collateral: Uint128
  max_ltv_health_factor: Decimal | null
  liquidation_health_factor: Decimal | null
  liquidatable: boolean
  above_max_ltv: boolean
}

export type BorrowTarget = 'deposit' | 'wallet'
