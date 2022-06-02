import { testWallet1 } from '../utils/test-wallets';
import { getOsmosisClient } from '../utils/osmosis-client';
import fs from 'fs';
import path from 'path';
import { EncodeObject } from '@cosmjs/proto-signing';
import { AccessType } from 'cosmjs-types/cosmwasm/wasm/v1/types';
import { Network, networks } from '../utils/config';

describe('example contract', () => {
  test('can deploy contract', async () => {
    const client = await getOsmosisClient(testWallet1);

    const contractCode = fs.readFileSync(path.resolve(__dirname, '../../artifacts/example-aarch64.wasm'));
    const storeCode: EncodeObject = {
      typeUrl: '/cosmwasm.wasm.v1.MsgStoreCode',
      value: {
        instantiate_permission: {
          address: testWallet1.address,
          permission: AccessType.ACCESS_TYPE_UNSPECIFIED,
        },
        sender: testWallet1.address,
        wasm_byte_code: contractCode,
      },
    };

    const result = await client.signAndBroadcast(
      testWallet1.address,
      [storeCode],
      networks[Network.OSMOSIS].defaultSendFee,
    );

    console.log(result);
  });
});
