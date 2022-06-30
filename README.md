# Fields of Mars: Credit Manager

## Bug bounty

## Overview

## Development

### Dependencies

### Environment Setup

Docker
https://docs.docker.com/get-docker/
v8

Osmosisd
Select option 3 (localosmosis), the installer will configure everything for you.
The osmosisd dameon on your local computer is used to communicate with the localosmosis daemin running inside the Docker container.
https://get.osmosis.zone/

install localosmosis
https://docs.osmosis.zone/developing/tools/localosmosis.html#install-localosmosis

cd localOsmosis
make start

now creating blocks


osmosjs?


cargo wasm?

docker run --rm -v "$(pwd)":/code \
--mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
cosmwasm/workspace-optimizer:0.12.6


SCRIPTS
npm install


### Test

### Deploy

### Notes

## Deployment

### Mainnet

### Testnet

## License
