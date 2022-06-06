import { toUtf8 } from "@cosmjs/encoding";
import { Uint53 } from "@cosmjs/math";
import { assertIsDeliverTxSuccess, SigningStargateClient } from '@cosmjs/stargate';
import { findAttribute, parseRawLog } from '@cosmjs/stargate/build/logs';
import { MsgInstantiateContract, MsgStoreCode } from 'cosmjs-types/cosmwasm/wasm/v1/tx';
import fs from 'fs';
import Long from "long";
import path from 'path';
import { Network, networks } from '../utils/config';
import { getOsmosisClient, getQueryClient } from '../utils/osmosis-client';
import { testWallet1 } from '../utils/test-wallets';

const INSTANTIATE_STR = "test-instantiate-string-123"

describe('example contract', () => {
  let client: SigningStargateClient;
  let codeId: number;
  let contractAddr: string;

  beforeAll(async () => {
    client = await getOsmosisClient(testWallet1);
  })

  afterAll(() => {
    client.disconnect();
  })

  test('can be deployed', async () => {
    const contractCode = fs.readFileSync(path.resolve(__dirname, '../../artifacts/example.wasm'));
    const storeCode = {
      typeUrl: '/cosmwasm.wasm.v1.MsgStoreCode',
      value: MsgStoreCode.fromPartial({
        sender: testWallet1.address,
        wasmByteCode: contractCode,
      }),
    };

    const uploadResult = await client.signAndBroadcast(
      testWallet1.address,
      [storeCode],
      networks[Network.OSMOSIS].defaultSendFee,
    );

    assertIsDeliverTxSuccess(uploadResult);

    const parsedLog = parseRawLog(uploadResult.rawLog);
    const codeIdAttr = findAttribute(parsedLog, "store_code", "code_id");
    codeId = Number.parseInt(codeIdAttr.value, 10);

    expect(codeId).toBeDefined();
  });

  test('can be instantiated', async () => {
    const instantiateContractMsg = {
      typeUrl: "/cosmwasm.wasm.v1.MsgInstantiateContract",
      value: MsgInstantiateContract.fromPartial({
        sender: testWallet1.address,
        codeId: Long.fromString(new Uint53(codeId).toString()),
        label: "instantiate-example-contract",
        msg: toUtf8(JSON.stringify({ some_string: INSTANTIATE_STR })),
      }),
    };

    const instantiateResult = await client.signAndBroadcast(
      testWallet1.address,
      [instantiateContractMsg],
      networks[Network.OSMOSIS].defaultSendFee,
    );

    assertIsDeliverTxSuccess(instantiateResult);

    const parsedLogs = parseRawLog(instantiateResult.rawLog);
    const contractAddressAttr = findAttribute(parsedLogs, "instantiate", "_contract_address");
    contractAddr = contractAddressAttr.value;

    expect(contractAddr).toBeDefined();
  });

  test.skip('can save item', async () => {
    const queryClient = await getQueryClient();
    const beforeRes: { str: string } = await queryClient.queryContractSmart(contractAddr, { get_stored_string: {} })
    expect(beforeRes.str).toBe(INSTANTIATE_STR)

    const updatedString = "spiderman123"

    const afterRes: { str: string } = await queryClient.queryContractSmart(contractAddr, { get_stored_string: {} })
    expect(afterRes.str).toBe(updatedString)
    console.log('afterRes', afterRes)

    queryClient.disconnect()
  });
})
