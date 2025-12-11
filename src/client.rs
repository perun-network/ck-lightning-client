// Copyright 2025 - See NOTICE file for copyright holders.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::{Context, Result};
use candid::{Decode, Encode};
use ic_agent::export::Principal;
use lnp_node::ckl::agent::ICAgent;

pub mod client {
    use anyhow::{Context, Result};
    use candid::{Decode, Encode};
    use ic_agent::export::Principal;
    use lnp_node::ckl::agent::ICAgent;

    pub struct ck_lightning_client {
        pub agent: ICAgent,
    }

    impl ck_lightning_client {
        pub fn new(agent: ICAgent) -> Self {
            Self { agent }
        }
    }
}
