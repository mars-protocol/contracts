#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Osmosis Localnet Quick Start${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Check if osmosisd is installed
if ! command -v osmosisd &> /dev/null; then
    echo -e "${RED}Error: osmosisd is not installed or not in PATH${NC}"
    echo "Please install osmosisd first: https://docs.osmosis.zone/overview/osmosis"
    exit 1
fi

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Parse arguments
SKIP_RESET=false
SKIP_SETUP=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --skip-reset)
      SKIP_RESET=true
      shift
      ;;
    --skip-setup)
      SKIP_SETUP=true
      shift
      ;;
    --help)
      echo "Usage: $0 [options]"
      echo ""
      echo "Options:"
      echo "  --skip-reset   Skip chain reset (use existing data)"
      echo "  --skip-setup   Skip pool creation and address seeding"
      echo "  --help         Show this help message"
      exit 0
      ;;
    *)
      echo -e "${RED}Unknown option: $1${NC}"
      exit 1
      ;;
  esac
done

# Step 1: Reset chain (unless skipped)
if [ "$SKIP_RESET" = false ]; then
  echo -e "${YELLOW}Step 1: Resetting chain to genesis state...${NC}"
  "$SCRIPT_DIR/reset-localnet.sh"
  echo ""
else
  echo -e "${BLUE}Step 1: Skipping chain reset${NC}"
fi

# Step 2: Start chain in background
echo -e "${YELLOW}Step 2: Starting osmosisd...${NC}"
echo -e "${BLUE}Starting chain in background...${NC}"

# Start osmosisd in background
osmosisd start --home ~/.osmosisd > /tmp/osmosisd.log 2>&1 &
OSMOSIS_PID=$!

echo -e "${GREEN}Chain started with PID: $OSMOSIS_PID${NC}"
echo -e "${BLUE}Logs: tail -f /tmp/osmosisd.log${NC}"

# Wait for chain to be ready
echo -e "${YELLOW}Waiting for chain to produce blocks...${NC}"
sleep 8

# Check if process is still running
if ! ps -p $OSMOSIS_PID > /dev/null; then
  echo -e "${RED}Error: osmosisd process died${NC}"
  echo -e "${YELLOW}Check logs: cat /tmp/osmosisd.log${NC}"
  exit 1
fi

# Try to query the chain
MAX_RETRIES=10
RETRY_COUNT=0
while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
  if curl -s http://localhost:26657/status > /dev/null 2>&1; then
    echo -e "${GREEN}Chain is ready!${NC}"
    break
  fi
  RETRY_COUNT=$((RETRY_COUNT + 1))
  echo -e "${BLUE}Waiting for RPC to be ready... ($RETRY_COUNT/$MAX_RETRIES)${NC}"
  sleep 2
done

if [ $RETRY_COUNT -eq $MAX_RETRIES ]; then
  echo -e "${RED}Error: Chain failed to start after $MAX_RETRIES attempts${NC}"
  kill $OSMOSIS_PID 2>/dev/null || true
  exit 1
fi

echo ""

# Step 3: Setup pools and seed addresses (unless skipped)
if [ "$SKIP_SETUP" = false ]; then
  echo -e "${YELLOW}Step 3: Creating pools and seeding addresses...${NC}"
  cd "$SCRIPT_DIR"
  npm run setup:localnet
  echo ""
else
  echo -e "${BLUE}Step 3: Skipping pool setup${NC}"
fi

echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Localnet is ready!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo -e "${BLUE}Chain PID: ${YELLOW}$OSMOSIS_PID${NC}"
echo -e "${BLUE}RPC Endpoint: ${YELLOW}http://localhost:26657${NC}"
echo -e "${BLUE}LCD Endpoint: ${YELLOW}http://localhost:1317${NC}"
echo -e "${BLUE}gRPC Endpoint: ${YELLOW}localhost:9090${NC}"
echo ""
echo -e "${BLUE}To view logs:${NC}"
echo -e "${YELLOW}  tail -f /tmp/osmosisd.log${NC}"
echo ""
echo -e "${BLUE}To stop the chain:${NC}"
echo -e "${YELLOW}  kill $OSMOSIS_PID${NC}"
echo -e "${YELLOW}  # or${NC}"
echo -e "${YELLOW}  pkill osmosisd${NC}"
echo ""
echo -e "${BLUE}Genesis account:${NC}"
osmosisd keys show validator -a --keyring-backend test --home ~/.osmosisd
echo ""
echo -e "${GREEN}Happy testing!${NC}"
