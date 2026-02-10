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

pub use ldk_sample::ICAgent;

pub const PEM_MINTING_ACC_PATH: &str = ".config/dfx/identity/minting_ledger/identity.pem";
pub const PEM_NODE_ACC_PATH: &str = ".config/dfx/identity/node/identity.pem";
pub const PEM_USER_ACC_PATH: &str = ".config/dfx/identity/user/identity.pem";
pub const LEDGER_ID: &str = "by6od-j4aaa-aaaaa-qaadq-cai";
pub const BTC_LEDGER_ID: &str = "u6s2n-gx777-77774-qaaba-cai";
pub const ICP_LEDGER_ID: &str = "ufxgi-4p777-77774-qaadq-cai";
pub const CKLIGHTNING_LEDGER_ID: &str = "vizcg-th777-77774-qaaea-cai";
pub const BTC_MINTER_ID: &str = "uzt4z-lp777-77774-qaabq-cai";
pub const DEVNET_BASIC_BITCOIN: &str = "vpyes-67777-77774-qaaeq-cai";

pub const BTC_LEDGER_DEFAULT_FEE: u64 = 1000;
pub const ICP_DDOS_FEE_E8S: u64 = 100_000_000; // 1 ICP in e8s
pub const ICP_TRANSFER_FEE_E8S: u64 = 10_000;     // 0.0001 ICP in e8s
