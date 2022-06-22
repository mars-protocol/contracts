export type AssetInfo = { cw20: string } | { native: string };
export const serializeAssetInfo = (obj: AssetInfo) => Object.entries(obj).flat().join(':');

export interface GetAllowListResponse {
  vaults: string[];
  assets: AssetInfo[];
}
