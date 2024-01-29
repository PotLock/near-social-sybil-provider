#!/bin/sh

if [ $? -ne 0 ]; then
  echo ">> Error building Near Social Sybil Provider contract"
  exit 1
fi

echo ">> Deploying Near Social Sybil Provider contract!"

# https://docs.near.org/tools/near-cli#near-dev-deploy
near dev-deploy --wasmFile ./out/main.wasm