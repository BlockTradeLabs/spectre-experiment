#!/bin/bash

# Exit on any error
set -e

# Always run the commands from the "test" dir
cd $(dirname $0)/..

mkdir -p specs
../target/release/spectre-node build-spec --disable-default-bootnode --add-bootnode "/ip4/127.0.0.1/tcp/33050/ws/p2p/12D3KooWFGaw1rxB6MSuN3ucuBm7hMq5pBFJbEoqTyth4cG483Cc" --parachain-id 2001 --raw > specs/spectre-node.json
./tmp/tanssi-node build-spec --chain dancebox-local --parachain-id 1000 --add-container-chain specs/spectre-node.json --invulnerable "Collator1000-01"  --invulnerable "Collator2001-01" > specs/tanssi-1000.json
