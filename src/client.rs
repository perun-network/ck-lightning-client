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
use anyhow::Result;
use candid::{Nat, Principal};
use cklightning::ic_types::SignedCandidInvoice;
use ldk_sample::ICAgent;

pub struct CkLightningClient {
    pub agent: ICAgent,
}

impl CkLightningClient {
    pub fn new(agent: ICAgent) -> Self {
        Self { agent }
    }

    pub async fn icrc1_balance_of(&self, pr: Principal) -> Result<Nat, Box<dyn std::error::Error>> {
        let bal_res = self.agent.icrc1_balance_of(pr).await?; //self.icrc1_balance_of(pr).await?;
        Ok(bal_res)
    }

    pub async fn query_ln_invoice(
        &self,
        amount: u64,
        btc_address: String,
    ) -> Result<SignedCandidInvoice, Box<dyn std::error::Error>> {
        let resp = self.agent.req_ln_invoice(amount, btc_address).await?;
        Ok(resp)
    }

    pub async fn get_ln_address(&self) -> Result<String, Box<dyn std::error::Error>> {
        let resp = self.agent.get_ln_address().await?;
        Ok(resp)
    }
}
