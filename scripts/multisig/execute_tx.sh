#!/bin/sh

osmosisd tx wasm migrate osmo14wwk7raehxgzm0wuf6ycvuc7uff5ykxuqx7ywhdu5dlhsdqs0cxqmg9x4q 1861 '{}' 
  --from testnet-deployer 
  --gas-prices 0.1uosmo 
  --gas auto 
  --gas-adjustment 1.3 -y 
  --output json -b block
  --generate-only > unsignedTx.json