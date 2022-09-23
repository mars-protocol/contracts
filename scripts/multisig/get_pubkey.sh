#!/bin/sh

echo What is your wallet address 

read address 

curl https://osmosis-api.polkachu.com/cosmos/auth/v1beta1/accounts/$address
