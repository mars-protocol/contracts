import { calculateFee, GasPrice } from '@cosmjs/stargate';
import { StdFee } from '@cosmjs/amino';

export type NetworkConfig = {
  localRpcEndpoint: string;
  provider: string;
  transactionLink: (arg0: string) => string;
  walletLink: (arg0: string) => string;
  networkName: string;
  bech32Prefix: string;
  nativeDenom: string;
  defaultSendFee: StdFee;
};

export enum Network {
  OSMOSIS,
}

export const networks: Record<Network, NetworkConfig> = {
  [Network.OSMOSIS]: {
    localRpcEndpoint: 'tcp://localhost:26657',
    provider: 'https://rpc-osmosis.keplr.app/',
    transactionLink: (hash) => `https://www.mintscan.io/osmosis/txs/${hash}`,
    walletLink: (address) => `https://www.mintscan.io/osmosis/account/${address}`,
    networkName: 'osmosis',
    bech32Prefix: 'osmo',
    nativeDenom: 'uosmo',
    defaultSendFee: calculateFee(100_000, GasPrice.fromString('0.025uosmo')),
  },
};
