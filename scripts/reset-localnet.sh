#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
CHAIN_ID="localosmosis"
MONIKER="localnode"
CONFIG_FILE="./localnet-config.yaml"
OSMOSIS_HOME="$HOME/.osmosisd"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --chain-id)
      CHAIN_ID="$2"
      shift 2
      ;;
    --home)
      OSMOSIS_HOME="$2"
      shift 2
      ;;
    --config)
      CONFIG_FILE="$2"
      shift 2
      ;;
    --help)
      echo "Usage: $0 [options]"
      echo ""
      echo "Options:"
      echo "  --chain-id    Chain ID (default: localosmosis)"
      echo "  --home        Osmosis home directory (default: ~/.osmosisd)"
      echo "  --config      Config file path (default: ./localnet-config.yaml)"
      echo "  --help        Show this help message"
      exit 0
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}"
      exit 1
      ;;
  esac
done

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Osmosis Localnet Reset Script${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "Chain ID: ${YELLOW}$CHAIN_ID${NC}"
echo -e "Home Directory: ${YELLOW}$OSMOSIS_HOME${NC}"
echo -e "Config File: ${YELLOW}$CONFIG_FILE${NC}"
echo ""

# Check if osmosisd is installed
if ! command -v osmosisd &> /dev/null; then
    echo -e "${RED}Error: osmosisd is not installed or not in PATH${NC}"
    echo "Please install osmosisd first: https://docs.osmosis.zone/overview/osmosis"
    exit 1
fi

# Check if config file exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${YELLOW}Warning: Config file not found at $CONFIG_FILE${NC}"
    echo "Using default configuration..."
fi

# Stop any running osmosisd processes
echo -e "${YELLOW}Stopping any running osmosisd processes...${NC}"
pkill osmosisd || true
sleep 2

# Remove existing data
echo -e "${YELLOW}Removing existing chain data...${NC}"
rm -rf "$OSMOSIS_HOME"

# Initialize new chain
echo -e "${YELLOW}Initializing new chain...${NC}"
osmosisd init "$MONIKER" --chain-id "$CHAIN_ID" --home "$OSMOSIS_HOME"

# Parse config file and extract genesis account mnemonic (if using yaml config)
# For now, use a default mnemonic
GENESIS_MNEMONIC="bottom loan skill merry east cradle onion journey palm apology verb edit desert impose absurd oil bubble sweet glove shallow size build burst effort"

# Add genesis account
echo -e "${YELLOW}Adding genesis account...${NC}"
echo "$GENESIS_MNEMONIC" | osmosisd keys add validator --recover --keyring-backend test --home "$OSMOSIS_HOME"

# Get the genesis account address
GENESIS_ADDR=$(osmosisd keys show validator -a --keyring-backend test --home "$OSMOSIS_HOME")
echo -e "Genesis account address: ${GREEN}$GENESIS_ADDR${NC}"

# Add genesis account with initial balances
echo -e "${YELLOW}Adding genesis account balances...${NC}"
osmosisd add-genesis-account "$GENESIS_ADDR" \
    100000000000000000uosmo,10000000000000000uatom,10000000000000000uusdc \
    --home "$OSMOSIS_HOME"

# Update genesis to use uosmo as staking denom instead of stake
echo -e "${YELLOW}Updating genesis configuration...${NC}"
sed -i.bak 's/"stake"/"uosmo"/g' "$OSMOSIS_HOME/config/genesis.json"

# Create gentx
echo -e "${YELLOW}Creating genesis transaction...${NC}"
osmosisd gentx validator 500000000uosmo \
    --chain-id "$CHAIN_ID" \
    --keyring-backend test \
    --home "$OSMOSIS_HOME"

# Collect gentxs
echo -e "${YELLOW}Collecting genesis transactions...${NC}"
osmosisd collect-gentxs --home "$OSMOSIS_HOME"

# Update app.toml configuration for local development
echo -e "${YELLOW}Updating app.toml configuration...${NC}"

# Enable API and set to listen on all interfaces
sed -i.bak 's/enable = false/enable = true/g' "$OSMOSIS_HOME/config/app.toml"
sed -i.bak 's/swagger = false/swagger = true/g' "$OSMOSIS_HOME/config/app.toml"

# Set minimum gas prices to 0 for local testing
sed -i.bak 's/minimum-gas-prices = "0.0025uosmo"/minimum-gas-prices = "0uosmo"/g' "$OSMOSIS_HOME/config/app.toml"

# Update config for faster block times (good for local testing)
sed -i.bak 's/timeout_commit = "5s"/timeout_commit = "1s"/g' "$OSMOSIS_HOME/config/config.toml"
sed -i.bak 's/timeout_propose = "3s"/timeout_propose = "1s"/g' "$OSMOSIS_HOME/config/config.toml"

# Enable unsafe CORS for local development
sed -i.bak 's/enabled-unsafe-cors = false/enabled-unsafe-cors = true/g' "$OSMOSIS_HOME/config/app.toml"
sed -i.bak 's/cors_allowed_origins = \[\]/cors_allowed_origins = ["*"]/g' "$OSMOSIS_HOME/config/config.toml"

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Localnet reset complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "To start the chain, run:"
echo -e "${YELLOW}osmosisd start --home $OSMOSIS_HOME${NC}"
echo ""
echo -e "In another terminal, run the pool setup script:"
echo -e "${YELLOW}cd scripts && npm run setup:localnet${NC}"
echo ""
echo -e "Validator address: ${GREEN}$GENESIS_ADDR${NC}"
echo -e "Keyring backend: ${YELLOW}test${NC}"
echo ""
