# ckLightning Client

This repository contains the ckLightning client. It is a to-be-production implementation of a backend to connect Lightning Network nodes to our Dfinity Internet Computer canister. It connects to the ic-lightning-relay node and also the ckLightning canister, which implements Lightning Network Payment Channels.

## Prerequisites

- Rust toolchain (stable)
- `dfx` as the Internet Computer environment (local) and PEM identities
- Access to the other ckLightning repositories (`ckLighting canister` and `ic-lightning-relay`)

## Build & Deploy

cargo build --release --bin main
./target/release/main

## Test

cargo test

## Usage 

### Set up Bitcoin Node

In your bitcoin directory, start the bitcoin daemon with the following command:

```bash
./bin/bitcoind -conf=~/regtest/bitcoin.conf -datadir=~/regtest/node1 -regtest -daemon
```

Your local bitcoin chain will be running with regtest local blockchain parameters that are stored inside bitcoin.conf. They should look like this:

```

# Enable regtest mode.
regtest=1

# Needed so bitcoind actually serves RPC.
server=1

# RPC credentials (your existing values)
rpcuser=ic-btc-integration
rpcpassword=QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E=
rpcauth=ic-btc-integration:cdf2741387f3a12438f69092f0fdad8e$62081498c98bee09a0dce2b30671123fa561932992ce377585e8e08bb0c11dfa

# Helpful for LDK / testing
txindex=1
acceptnonstdtxn=1

# Bind locally (optional but recommended)
rpcbind=127.0.0.1
rpcallowip=127.0.0.1

```

### Set up the Internet Computer and the necessary canisters locally via dfx

cd into the `ckLightning canister` repository and run:

```
cd ic/
./start_dfx.sh
sleep 10 seconds
./create_identities.sh
./deploy_contracts_devnet.sh
```

During the installation of the ckLightning canister, select regtest as the Bitcoin network and confirm.

### Set up the ic-lightning-relay

cd into the `ic-lightning-relay` repository and run:

```
cargo run -- 'ic-btc-integration:QPQiNaph19FqUsCrBRN0FII7lyM26B51fAMeBQzCb-E=@127.0.0.1:18443' ~/path/to/bitcoin_chaindata 9735 regtest nodeA 127.0.0.1:9735
```

### Make invocations to the ckLightning canister

For instance, request the Lightning Node Bitcoin address for payment:

```
ln-address
```
Then insert that address into the invocation of the invoice generation, along with a desired amount:

```ln-invoice <amount> <bitcoin_address>
```

### Stop the local processes after testing

When done testing, you can kill the processes with these commands:

```
./bin/bitcoin-cli -conf=$HOME/regtest/bitcoin.conf -datadir=$HOME/regtest/node1 -regtest stop
dfx killall
```

# Copyright

Copyright 2026 PolyCrypt GmbH. Use of the source code is governed by the Apache 2.0 license that can be found in the [LICENSE file](LICENSE).