import { assertIsDeliverTxSuccess } from '@cosmjs/stargate';
import { Network, networks } from '../utils/config';
import { getOsmosisClient } from '../utils/osmosis-client';
import { testWallet1, testWallet2 } from '../utils/test-wallets';

describe('example client test', () => {
  test('can get client and transfer tokens', async () => {
    const client = await getOsmosisClient(testWallet1);

    const result = await client.sendTokens(
      testWallet1.address,
      testWallet2.address,
      [
        {
          denom: 'uosmo',
          amount: '12345',
        },
      ],
      networks[Network.OSMOSIS].defaultSendFee,
    );

    assertIsDeliverTxSuccess(result);

    client.disconnect()
  });
});
