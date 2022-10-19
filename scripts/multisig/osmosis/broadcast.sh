#!/bin/sh

#step 4: broadcast tx 
echo what is the signed tx file name? 

read signedTx

osmosisd tx broadcast `$signedTx`.json \
    --chain-id=osmo-test-4 \
    --broadcast-mode=block