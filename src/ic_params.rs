//  Copyright 2026 PolyCrypt GmbH
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

pub use ic_lightning_relay::ICAgent;

pub const PEM_MINTING_ACC_PATH: &str = ".config/dfx/identity/staging-deployer/identity.pem";
pub const PEM_NODE_ACC_PATH: &str = ".config/dfx/identity/staging-deployer/identity.pem";
pub const PEM_USER_ACC_PATH: &str = ".config/dfx/identity/staging-user/identity.pem";
pub const LEDGER_ID: &str = "mc6ru-gyaaa-aaaar-qaaaq-cai";
pub const BTC_LEDGER_ID: &str = "mc6ru-gyaaa-aaaar-qaaaq-cai";
pub const ICP_LEDGER_ID: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
pub const CKLIGHTNING_LEDGER_ID: &str = "5iwit-tiaaa-aaaau-aelaa-cai";
pub const BTC_MINTER_ID: &str = "ml52i-qqaaa-aaaar-qaaba-cai";
pub const DEVNET_BASIC_BITCOIN: &str = "g4xu7-jiaaa-aaaan-aaaaq-cai";

pub const BTC_LEDGER_DEFAULT_FEE: u64 = 1000;
pub const ICP_DDOS_FEE_E8S: u64 = 100_000_000; // 1 ICP in e8s
pub const ICP_TRANSFER_FEE_E8S: u64 = 10_000;     // 0.0001 ICP in e8s
