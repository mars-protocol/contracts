import { testWallet1 } from '../utils/test-wallets';
import { getOsmosisClient } from '../utils/osmosis-client';
import fs from 'fs';
import path from 'path';
import { Network, networks } from '../utils/config';
import { MsgStoreCode } from 'cosmjs-types/cosmwasm/wasm/v1/tx';

describe('example contract', () => {
  test('can deploy contract', async () => {
    const client = await getOsmosisClient(testWallet1);

    const contractCode = fs.readFileSync(path.resolve(__dirname, '../../artifacts/example-aarch64.wasm'));
    const storeCode = {
      typeUrl: '/cosmwasm.wasm.v1.MsgStoreCode',
      value: MsgStoreCode.fromPartial({
        sender: testWallet1.address,
        wasmByteCode: contractCode,
      }),
    };

    const result = await client.signAndBroadcast(
      testWallet1.address,
      [storeCode],
      networks[Network.OSMOSIS].defaultSendFee,
    );

    console.log(result);
  });
});
