#!/bin/sh

# step 1: generate unsignedTx.json

echo what is your wallet address? 

read wallet 

echo what will you name the unsigned tx file? 

read unsignedTx

osmosisd tx wasm migrate <name of contract address>
  --from $wallet
  --gas-prices 0.1uosmo 
  --gas auto 
  --gas-adjustment 1.3 -y 
  --generate-only > `$unsignedTx`.json