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

use bitcoin::hashes::Hash;
use bitcoin::hashes::sha256;
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use candid::Principal;
use candid::{CandidType, Deserialize, Nat};
use lightning_invoice::{Bolt11Invoice, Currency, InvoiceBuilder, PaymentSecret};
use lnp::p2p::bolt::ChannelId;
use lnp::p2p::bolt::TempChannelId;
use lnp_rpc::PayInvoice;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct CandidInvoice {
    pub invoice: String, // bech32 BOLT11 string
    pub amount_msat: Option<Nat>,
    pub payment_hash: Vec<u8>,   // 32 bytes
    pub payment_secret: Vec<u8>, // 32 bytes
    pub timestamp: u64,
    pub expiry_secs: Option<u64>,
    pub currency: String,
    pub channel_id: Vec<u8>, // 32 bytes from ChannelId
}

#[derive(Clone, CandidType, Deserialize)]
pub struct LnInvoiceRequest {
    pub caller_principal: Principal, // for derivation
    pub btc_address: String,         // deposit address verification
    pub amount_msat: u64,            // invoice amount
}

#[derive(Clone, Debug, CandidType, Deserialize)]
pub struct SignedCandidInvoice {
    pub invoice: String, // Signed BOLT11 string
    pub amount_msat: Option<Nat>,
    pub payment_hash: Vec<u8>,   // 32 bytes
    pub payment_secret: Vec<u8>, // 32 bytes
    pub timestamp: u64,
    pub expiry_secs: Option<u64>,
    pub currency: String,
    pub channel_id: Vec<u8>, // 32 bytes
    pub signature: Vec<u8>,  // ✅ NEW: Invoice signature bytes
}

// build_signed_invoice() goes here – omitted for brevity

pub fn build_signed_invoice() -> Bolt11Invoice {
    let payment_secret = PaymentSecret([0u8; 32]);

    let privkey = SecretKey::from_slice(&[41; 32]).expect("valid privkey");
    let secp_ctx = Secp256k1::new();

    let dummy_bytes = [0u8; 32];
    let payment_hash = sha256::Hash::from_slice(&dummy_bytes).expect("slice length correct");
    let amount_to_invoice = 1000;
    let signed_invoice = InvoiceBuilder::new(Currency::Bitcoin)
        .description("Test".into())
        .payment_hash(payment_hash)
        .payment_secret(payment_secret)
        .duration_since_epoch(Duration::from_secs(1234567))
        .amount_milli_satoshis(amount_to_invoice) //
        .build_raw()
        .expect("build invoice")
        .sign::<_, ()>(|hash| Ok(secp_ctx.sign_ecdsa_recoverable(hash, &privkey)))
        .expect("sign invoice");

    let inv_from_signed = Bolt11Invoice::from_signed(signed_invoice).expect("parse signed invoice");

    return inv_from_signed;
}

// TODO: PayInvoice uses old Invoice type, incompatible with lightning-invoice 0.33
// Commented out until lnp_rpc is updated or we remove dependency
// pub fn build_pay_invoice() -> PayInvoice {
//     let invoice = build_signed_invoice();
//
//     let temp_chan_id = TempChannelId::random();
//     let chan_id = ChannelId::from(temp_chan_id);
//     let amount_msat = 1000;
//
//     PayInvoice {
//         invoice,
//         channel_id: chan_id,
//         amount_msat: Some(amount_msat),
//     }
// }
//
// pub fn print_pay_invoice() {
//     let pay_invoice = build_pay_invoice();
//     println!("PayInvoice debug:\n{:?}", pay_invoice);
//     println!("PayInvoice display:\n{}", pay_invoice);
// }

impl CandidInvoice {
    pub fn into_invoice(self) -> Bolt11Invoice {
        // You are storing the canonical BOLT11 string already,
        // so just parse it back.
        Bolt11Invoice::from_str(&self.invoice).expect("invalid BOLT11 in CandidInvoice")
    }
}

// TODO: Commented out until PayInvoice compatibility is resolved
// impl From<PayInvoice> for CandidInvoice {
//     fn from(p: PayInvoice) -> Self {
//         let invoice: Bolt11Invoice = p.invoice;
//
//         // BOLT11 string
//         let invoice_str = invoice.to_string();
//
//         // Amount in msat
//         let amount_msat_opt = invoice.amount_milli_satoshis();
//
//         // Payment hash (32 bytes)
//         let payment_hash = invoice.payment_hash().to_byte_array().to_vec();
//
//         // Payment secret (PaymentSecret([u8; 32]))
//         let payment_secret = invoice.payment_secret().0.to_vec();
//
//         // Timestamp: seconds since UNIX_EPOCH
//         let timestamp = invoice.duration_since_epoch().as_secs();
//
//         // Expiry: Duration -> seconds
//         // let expiry_secs = invoice.expiry_time().as_secs();
//         let expiry_secs = Some(invoice.expiry_time().as_secs());
//         // Currency string
//         let currency = format!("{:?}", invoice.currency());
//
//         // ChannelId -> bytes
//         let channel_id_bytes: Vec<u8> = p.channel_id.as_slice().to_vec();
//
//         CandidInvoice {
//             invoice: invoice_str,
//             amount_msat: amount_msat_opt.map(Nat::from),
//             payment_hash,
//             payment_secret,
//             timestamp,
//             expiry_secs,
//             currency,
//             channel_id: channel_id_bytes,
//         }
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;
    use candid::{Decode, Encode};
    use strict_encoding::{StrictDecode, StrictEncode};
    // TODO: Commented out until PayInvoice compatibility is resolved
    // #[test]
    // fn test_payinvoice_encode_decode() {
    //     let pay_invoice = build_pay_invoice();
    //
    //     let mut encoded_bytes = Vec::new();
    //     pay_invoice
    //         .strict_encode(&mut encoded_bytes)
    //         .expect("Failed to encode PayInvoice");
    //
    //     let decoded_invoice =
    //         PayInvoice::strict_decode(&encoded_bytes[..]).expect("Failed to decode PayInvoice");
    //
    //     assert_eq!(pay_invoice, decoded_invoice);
    //
    //     print_pay_invoice();
    // }

    // TODO: Commented out until PayInvoice compatibility is resolved
    // #[test]
    // fn test_invoice_candid_roundtrip() {
    //         // 1. build original PayInvoice + Invoice
    //         let original_pay = build_pay_invoice();
    //         let original_invoice = original_pay.invoice.clone();
    //
    //         // 2. map to CandidInvoice
    //         let candid_inv: CandidInvoice = original_pay.into();
    //
    //         // basic field checks on the DTO itself
    //         assert_eq!(candid_inv.invoice, original_invoice.to_string());
    //         assert_eq!(
    //             candid_inv.amount_msat,
    //             original_invoice.amount_milli_satoshis().map(Nat::from)
    //         );
    //         assert_eq!(
    //             candid_inv.payment_hash,
    //             original_invoice.payment_hash().to_byte_array().to_vec()
    //         );
    //         assert_eq!(
    //             candid_inv.payment_secret,
    //             original_invoice.payment_secret().0.to_vec()
    //         );
    //
    //         // 3. Candid encode + decode
    //         let bytes = Encode!(&candid_inv).expect("candid encode");
    //         // let decoded: CandidInvoice = Decode!(&bytes).expect("candid decode");
    //         let decoded = Decode!(&bytes, CandidInvoice).expect("candid decode");
    //
    //         // 4. Decode back to Invoice
    //         let decoded_invoice = decoded.into_invoice();
    //
    //         // 5. Compare original vs decoded invoices
    //         assert_eq!(original_invoice.to_string(), decoded_invoice.to_string());
    //         assert_eq!(
    //             original_invoice.amount_milli_satoshis(),
    //             decoded_invoice.amount_milli_satoshis()
    //         );
    //         assert_eq!(
    //             original_invoice.payment_hash().to_byte_array().to_vec(),
    //             decoded_invoice.payment_hash().to_byte_array().to_vec()
    //         );
    //         assert_eq!(
    //             original_invoice.payment_secret().0.to_vec(),
    //             decoded_invoice.payment_secret().0.to_vec()
    //         );
    //         assert_eq!(
    //             original_invoice.expiry_time().as_secs(),
    //             decoded_invoice.expiry_time().as_secs()
    //         );
    //         assert_eq!(original_invoice.currency(), decoded_invoice.currency());
    //     }
}
