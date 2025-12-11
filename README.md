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


# Copyright

Copyright 2025 PolyCrypt GmbH. Use of the source code is governed by the Apache 2.0 license that can be found in the [LICENSE file](LICENSE).