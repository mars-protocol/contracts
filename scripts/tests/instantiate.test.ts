import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { toHex } from '@cosmjs/encoding';
import fs from 'fs';
import path from 'path';
import { Network, networks } from '../utils/config';
import { testWallet1 } from '../utils/test-wallets';
import { getCosmWasmClient } from '../utils/client';
import { sha256 } from '@cosmjs/crypto';
import { GetAllowListResponse, serializeAssetInfo } from '../utils/types';

describe('instantiating fields contract', () => {
  let client: SigningCosmWasmClient;
  let codeId: number;
  let contractAddr: string;

  beforeAll(async () => {
    client = await getCosmWasmClient(testWallet1);
  });

  afterAll(() => {
    client.disconnect();
  });

  test('can be uploaded', async () => {
    const wasm = fs.readFileSync(path.resolve(__dirname, '../../artifacts/credit_manager.wasm'));
    const {
      codeId: uploadCodeId,
      originalChecksum,
      originalSize,
      compressedChecksum,
      compressedSize,
    } = await client.upload(testWallet1.address, wasm, networks[Network.OSMOSIS].defaultSendFee);

    expect(originalChecksum).toEqual(toHex(sha256(wasm)));
    expect(originalSize).toEqual(wasm.length);
    expect(compressedChecksum).toMatch(/^[0-9a-f]{64}$/);
    expect(compressedSize).toBeLessThan(wasm.length * 0.5);
    expect(uploadCodeId).toBeGreaterThanOrEqual(1);
    codeId = uploadCodeId;
    expect(codeId).toBeDefined();
  });

  test('can be instantiated', async () => {
    const owner = 'osmo105e4n2f2gr92x8pxvmhxj5v7e2m9j08zelxdnq';
    const allowed_vaults = [
      'osmo1r4c2g5wex39kcdeahgxjaxnr2wnv7jvxc5je0e',
      'osmo1av54qcmavhjkqsd67cf6f4cedqjrdeh73k52l2',
      'osmo18zhhdrjd5qfvewnu5lkkgv6w7rtcmzh3hq7qes',
    ];
    const allowed_assets = [
      { cw20: 'osmo1ptlhw66xg7nznag8sy4mnlsj04xklxqjgqrpz4' },
      { native: 'uosmo' },
      { cw20: 'osmo1ewn73qp0aqrtya38p0nv5c2xsshdea7ad34qkc' },
    ];

    const { contractAddress } = await client.instantiate(
      testWallet1.address,
      codeId,
      { owner, allowed_vaults, allowed_assets },
      'test-instantiate-string-123',
      networks[Network.OSMOSIS].defaultSendFee,
    );
    contractAddr = contractAddress;
    expect(contractAddr).toBeDefined();

    const ownerFromQuery = await client.queryContractSmart(contractAddress, { get_owner: {} });
    expect(ownerFromQuery).toEqual({ owner });

    const allowListsFromQuery: GetAllowListResponse = await client.queryContractSmart(contractAddress, {
      get_allow_lists: {},
    });

    expect(allowListsFromQuery.vaults.length).toEqual(allowed_vaults.length);
    expect(allowListsFromQuery.vaults.every((v) => allowed_vaults.includes(v))).toBeTruthy();

    expect(allowListsFromQuery.assets.length).toEqual(allowed_assets.length);
    expect(
      allowListsFromQuery.assets
        .map(serializeAssetInfo)
        .every((asset_str) => allowed_assets.map(serializeAssetInfo).includes(asset_str)),
    ).toBeTruthy();
  });
});
