# Osmosis Localnet Setup

This directory contains scripts to set up and configure a local Osmosis blockchain environment for development and testing.

## Overview

The localnet setup consists of two main components:

1. **reset-localnet.sh** - Bash script that resets osmosisd to genesis state
2. **setup-localnet.ts** - TypeScript script that creates pools and seeds addresses
3. **localnet-config.yaml** - Configuration file for pools, assets, and addresses

## Prerequisites

1. **osmosisd** - Osmosis daemon must be installed and in your PATH
   - Installation guide: https://docs.osmosis.zone/overview/osmosis
   - Verify with: `osmosisd version`

2. **Node.js and npm/yarn** - For running the TypeScript setup script
   - Verify with: `node --version` and `npm --version`

## Quick Start

### 1. Reset the Local Chain

```bash
cd scripts
./reset-localnet.sh
```

This will:
- Stop any running osmosisd processes
- Remove existing chain data
- Initialize a new chain with chain ID `localosmosis`
- Create a genesis account with substantial balances
- Configure the chain for local development (fast blocks, CORS enabled, etc.)

### 2. Start the Chain

In a separate terminal:

```bash
osmosisd start --home ~/.osmosisd
```

Wait for the chain to start producing blocks (you'll see logs scrolling).

### 3. Create Pools and Seed Addresses

In another terminal:

```bash
cd scripts
npm run setup:localnet
```

Or with a custom config file:

```bash
npm run setup:localnet-custom path/to/custom-config.yaml
```

This will:
- Connect to your local Osmosis node
- Create all pools defined in the config
- Seed the specified addresses with tokens

## Configuration

### Config File Structure

The [localnet-config.yaml](./localnet-config.yaml) file defines:

```yaml
chain:
  chain_id: "localosmosis"
  denom: "uosmo"

assets:
  - denom: "uosmo"
    description: "Osmosis native token"
  # ... more assets

pools:
  - name: "OSMO/ATOM Pool"
    token1:
      denom: "uosmo"
      amount: "1000000000000"  # Amount in base units
    token2:
      denom: "uatom"
      amount: "100000000000"
    swap_fee: "0.002"  # 0.2%
    exit_fee: "0.000"
  # ... more pools

seed_addresses:
  - address: "osmo1..."
    name: "Test Account 1"
    balances:
      - denom: "uosmo"
        amount: "100000000000000"
      # ... more balances
  # ... more addresses

genesis_account:
  name: "validator"
  mnemonic: "your mnemonic here..."
  balances:
    - denom: "uosmo"
      amount: "100000000000000000"
    # ... more balances
```

### Understanding Pool Pricing

Pools use a constant product formula (x * y = k). The price is determined by the ratio of tokens:

```
Price of token1 in terms of token2 = token2_amount / token1_amount
```

Example from config:
```yaml
token1:
  denom: "uosmo"
  amount: "1000000000000"  # 1,000,000 OSMO
token2:
  denom: "uatom"
  amount: "100000000000"   # 100,000 ATOM
```

This creates a price of 0.1 ATOM per OSMO (or 10 OSMO per ATOM).

### Token Amounts

All amounts are in base units (micro units). For tokens with 6 decimals:
- 1 OSMO = 1,000,000 uosmo
- 1 ATOM = 1,000,000 uatom
- 1 USDC = 1,000,000 uusdc

## Script Options

### reset-localnet.sh

```bash
./reset-localnet.sh [options]

Options:
  --chain-id    Chain ID (default: localosmosis)
  --home        Osmosis home directory (default: ~/.osmosisd)
  --config      Config file path (default: ./localnet-config.yaml)
  --help        Show help message
```

### setup-localnet.ts

```bash
# Use default config (./localnet-config.yaml)
npm run setup:localnet

# Use custom config
npm run setup:localnet-custom path/to/config.yaml

# Set custom RPC endpoint
RPC_ENDPOINT=http://localhost:26657 npm run setup:localnet
```

## Environment Variables

- `RPC_ENDPOINT` - RPC endpoint for the local chain (default: `http://localhost:26657`)

## Common Workflows

### Reset and Restart Everything

```bash
# Terminal 1: Reset and start chain
cd scripts
./reset-localnet.sh
osmosisd start --home ~/.osmosisd

# Terminal 2: Wait for chain to start, then setup pools
cd scripts
sleep 5
npm run setup:localnet
```

### Using Different Chain Home

```bash
# Reset with custom home
./reset-localnet.sh --home /path/to/custom/home

# Start with custom home
osmosisd start --home /path/to/custom/home
```

### Creating Custom Pools

1. Copy `localnet-config.yaml` to `my-config.yaml`
2. Edit pool configurations
3. Run setup with custom config:
   ```bash
   npm run setup:localnet-custom my-config.yaml
   ```

## Accessing Test Accounts

The genesis account mnemonic is in the config file. Import it into your wallet:

```bash
# Add to osmosisd keyring
echo "bottom loan skill merry east cradle onion journey palm apology verb edit desert impose absurd oil bubble sweet glove shallow size build burst effort" | \
  osmosisd keys add mykey --recover --keyring-backend test

# Get address
osmosisd keys show mykey -a --keyring-backend test
```

For Keplr or other wallets, use the mnemonic directly.

## Troubleshooting

### osmosisd not found

Make sure osmosisd is installed and in your PATH:
```bash
which osmosisd
osmosisd version
```

### Port already in use

If you get port binding errors, check for existing processes:
```bash
lsof -i :26657
pkill osmosisd
```

### Pool creation fails

Common issues:
- Chain not running: Start with `osmosisd start --home ~/.osmosisd`
- Insufficient balance: Check genesis account has enough tokens
- Invalid amounts: Amounts must be strings representing integers in base units

### Connection refused

Make sure:
1. Chain is running: `osmosisd start --home ~/.osmosisd`
2. RPC endpoint is correct: Default is `http://localhost:26657`
3. Chain has produced blocks: Wait a few seconds after starting

## Development Tips

### Fast Iteration

For rapid testing, use shorter block times (already configured in reset script):
- `timeout_commit = "1s"`
- `timeout_propose = "1s"`

### Querying Pools

```bash
# List all pools
osmosisd query gamm pools --home ~/.osmosisd

# Get specific pool
osmosisd query gamm pool 1 --home ~/.osmosisd

# Get pool's spot price
osmosisd query gamm spot-price 1 uosmo uatom --home ~/.osmosisd
```

### Checking Balances

```bash
# Query balance
osmosisd query bank balances <address> --home ~/.osmosisd

# Get LP token balance after providing liquidity
osmosisd query bank balances <address> --home ~/.osmosisd --denom gamm/pool/1
```

### Swapping Tokens

```bash
osmosisd tx gamm swap-exact-amount-in \
  1000000uosmo \
  1 \
  --swap-route-pool-ids 1 \
  --swap-route-denoms uatom \
  --from validator \
  --keyring-backend test \
  --chain-id localosmosis \
  --home ~/.osmosisd \
  --yes
```

## Integration with Tests

You can use this setup for:

1. **Manual testing** - Create pools and test swaps manually
2. **Integration tests** - Point your tests at the local chain
3. **Smart contract testing** - Deploy and test contracts against real pools

## Files

- [reset-localnet.sh](./reset-localnet.sh) - Genesis reset script
- [setup-localnet.ts](./setup-localnet.ts) - Pool creation and seeding script
- [localnet-config.yaml](./localnet-config.yaml) - Configuration file
- [types/localnet-config.ts](./types/localnet-config.ts) - TypeScript types for config

## Further Reading

- [Osmosis Documentation](https://docs.osmosis.zone/)
- [Osmosis GitHub](https://github.com/osmosis-labs/osmosis)
- [CosmJS Documentation](https://cosmos.github.io/cosmjs/)
