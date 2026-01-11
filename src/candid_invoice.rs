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
use candid::Nat;
use cklightning::ic_types::SignedCandidInvoice;
use lightning_invoice::{Bolt11Invoice, PaymentSecret};
use std::convert::TryInto;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Errors that can occur during Candid invoice conversions
#[derive(Debug)]
pub enum CandidConversionError {
    /// Failed to parse BOLT11 string
    InvalidBolt11(String),
    /// Field mismatch between Candid and Lightning types
    FieldMismatch(String),
    /// Invalid timestamp
    InvalidTimestamp(String),
    /// Missing required field
    MissingField(String),
}

impl std::fmt::Display for CandidConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CandidConversionError::InvalidBolt11(msg) => {
                write!(f, "Invalid BOLT11 invoice: {}", msg)
            }
            CandidConversionError::FieldMismatch(msg) => write!(f, "Field mismatch: {}", msg),
            CandidConversionError::InvalidTimestamp(msg) => write!(f, "Invalid timestamp: {}", msg),
            CandidConversionError::MissingField(msg) => write!(f, "Missing field: {}", msg),
        }
    }
}

impl std::error::Error for CandidConversionError {}

pub type ConversionResult<T> = Result<T, CandidConversionError>;

/// Convert SignedCandidInvoice to Bolt11Invoice
///
/// This function takes a Candid-encoded invoice (as received from the IC canister)
/// and converts it to a standard Lightning BOLT11 invoice.
///
/// # Arguments
/// * `candid_invoice` - The SignedCandidInvoice from the canister
///
/// # Returns
/// * `Ok(Bolt11Invoice)` - Successfully converted invoice
/// * `Err(CandidConversionError)` - Conversion failed
///
/// # Example
/// ```ignore
/// let bolt11 = candid_to_bolt11(&signed_candid_invoice)?;
/// // bolt11 can now be used with standard Lightning operations
/// ```
pub fn candid_to_bolt11(candid_invoice: &SignedCandidInvoice) -> ConversionResult<Bolt11Invoice> {
    // The SignedCandidInvoice contains a BOLT11 string
    // Parse it to get the actual Bolt11Invoice
    let invoice = Bolt11Invoice::from_str(&candid_invoice.invoice)
        .map_err(|e| CandidConversionError::InvalidBolt11(format!("{:?}", e)))?;

    // Verify that the Candid metadata matches the parsed invoice
    verify_candid_matches_bolt11(candid_invoice, &invoice)?;

    Ok(invoice)
}

/// Convert Bolt11Invoice to SignedCandidInvoice
///
/// This function takes a standard Lightning BOLT11 invoice and converts it
/// to the Candid format suitable for sending to the IC canister.
///
/// # Arguments
/// * `bolt11` - The Bolt11Invoice to convert
/// * `signature_bytes` - Optional signature bytes (65 bytes: recovery_id + 64-byte signature)
/// * `channel_id` - Optional channel ID (defaults to zeros)
///
/// # Returns
/// * `SignedCandidInvoice` - Candid-encoded invoice ready for IC transmission
///
/// # Example
/// ```ignore
/// let candid_invoice = bolt11_to_candid(&invoice, Some(signature), Some(channel_id));
/// // candid_invoice can now be sent to IC canister via Candid encoding
/// ```
pub fn bolt11_to_candid(
    bolt11: &Bolt11Invoice,
    signature_bytes: Option<Vec<u8>>,
    channel_id: Option<Vec<u8>>,
) -> SignedCandidInvoice {
    // BOLT11 string
    let invoice_str = bolt11.to_string();

    // Amount in msat
    let amount_msat = bolt11.amount_milli_satoshis().map(Nat::from);

    // Payment hash (32 bytes)
    let payment_hash = bolt11.payment_hash().to_byte_array().to_vec();

    // Payment secret (32 bytes)
    let payment_secret = bolt11.payment_secret().0.to_vec();

    // Timestamp: get from invoice creation time
    // BOLT11 invoices store timestamp as seconds since epoch
    let timestamp = bolt11.duration_since_epoch().as_secs();

    // Expiry: Duration -> seconds
    let expiry_secs = Some(bolt11.expiry_time().as_secs());

    // Currency string
    let currency = format!("{:?}", bolt11.currency());

    // Channel ID (default to zeros if not provided)
    let channel_id_bytes = channel_id.unwrap_or_else(|| vec![0u8; 32]);

    // Signature bytes (default to empty if not provided)
    let signature = signature_bytes.unwrap_or_else(Vec::new);

    SignedCandidInvoice {
        invoice: invoice_str,
        amount_msat,
        payment_hash,
        payment_secret,
        timestamp,
        expiry_secs,
        currency,
        channel_id: channel_id_bytes,
        signature,
    }
}

/// Verify that Candid metadata matches the parsed BOLT11 invoice
///
/// This is a safety check to ensure data integrity during conversions.
fn verify_candid_matches_bolt11(
    candid: &SignedCandidInvoice,
    bolt11: &Bolt11Invoice,
) -> ConversionResult<()> {
    // Check payment hash
    let bolt11_payment_hash = bolt11.payment_hash().to_byte_array().to_vec();
    if candid.payment_hash != bolt11_payment_hash {
        return Err(CandidConversionError::FieldMismatch(format!(
            "Payment hash mismatch: Candid has {:?}, BOLT11 has {:?}",
            candid.payment_hash, bolt11_payment_hash
        )));
    }

    // Check payment secret
    let bolt11_payment_secret = bolt11.payment_secret().0.to_vec();
    if candid.payment_secret != bolt11_payment_secret {
        return Err(CandidConversionError::FieldMismatch(format!(
            "Payment secret mismatch: Candid has {:?}, BOLT11 has {:?}",
            candid.payment_secret, bolt11_payment_secret
        )));
    }

    // Check amount (if present in Candid)
    if let Some(ref candid_amount) = candid.amount_msat {
        if let Some(bolt11_amount) = bolt11.amount_milli_satoshis() {
            // Convert Nat to u64 for comparison
            let candid_amount_u64: u64 = candid_amount.0.clone().try_into().map_err(|_| {
                CandidConversionError::FieldMismatch("Amount too large to convert".to_string())
            })?;
            if candid_amount_u64 != bolt11_amount {
                return Err(CandidConversionError::FieldMismatch(format!(
                    "Amount mismatch: Candid has {}, BOLT11 has {}",
                    candid_amount_u64, bolt11_amount
                )));
            }
        }
    }

    // Check timestamp (with some tolerance for rounding)
    let bolt11_timestamp = bolt11.duration_since_epoch().as_secs();

    // Allow up to 1 second difference due to rounding
    if candid.timestamp.abs_diff(bolt11_timestamp) > 1 {
        return Err(CandidConversionError::FieldMismatch(format!(
            "Timestamp mismatch: Candid has {}, BOLT11 has {}",
            candid.timestamp, bolt11_timestamp
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::hashes::Hash as HashTrait;
    use bitcoin::secp256k1::{Secp256k1, SecretKey};
    use lightning_invoice::{Currency, InvoiceBuilder};

    /// Helper to create a test BOLT11 invoice
    fn create_test_bolt11(description: &str, amount_msat: Option<u64>) -> Bolt11Invoice {
        let secp_ctx = Secp256k1::new();
        let private_key = SecretKey::from_slice(&[42u8; 32]).unwrap();
        let public_key = bitcoin::secp256k1::PublicKey::from_secret_key(&secp_ctx, &private_key);

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let payment_hash = bitcoin::hashes::sha256::Hash::from_slice(&[1u8; 32]).unwrap();
        let payment_secret = lightning_invoice::PaymentSecret([2u8; 32]);

        let mut builder = InvoiceBuilder::new(Currency::Bitcoin)
            .description(description.to_string())
            .payment_hash(payment_hash)
            .payment_secret(payment_secret)
            .duration_since_epoch(Duration::from_secs(timestamp))
            .min_final_cltv_expiry_delta(144)
            .payee_pub_key(public_key);

        if let Some(amt) = amount_msat {
            builder = builder.amount_milli_satoshis(amt);
        }

        builder
            .build_signed(|hash| secp_ctx.sign_ecdsa_recoverable(hash, &private_key))
            .unwrap()
    }

    #[test]
    fn test_bolt11_to_candid_basic() {
        let bolt11 = create_test_bolt11("Test invoice", Some(50000));

        let candid = bolt11_to_candid(&bolt11, None, None);

        // Verify invoice string is preserved
        assert_eq!(candid.invoice, bolt11.to_string());

        // Verify amount
        assert_eq!(candid.amount_msat, Some(Nat::from(50000u64)));

        // Verify payment hash
        assert_eq!(
            candid.payment_hash,
            bolt11.payment_hash().to_byte_array().to_vec()
        );

        // Verify payment secret
        assert_eq!(candid.payment_secret, bolt11.payment_secret().0.to_vec());

        // Verify currency
        assert!(candid.currency.contains("Bitcoin"));
    }

    #[test]
    fn test_candid_to_bolt11_roundtrip() {
        let original_bolt11 = create_test_bolt11("Roundtrip test", Some(100000));

        // Convert to Candid
        let candid = bolt11_to_candid(&original_bolt11, None, None);

        // Convert back to BOLT11
        let recovered_bolt11 = candid_to_bolt11(&candid).unwrap();

        // Verify they match
        assert_eq!(original_bolt11.to_string(), recovered_bolt11.to_string());
        assert_eq!(
            original_bolt11.payment_hash().to_byte_array(),
            recovered_bolt11.payment_hash().to_byte_array()
        );
        assert_eq!(
            original_bolt11.payment_secret().0,
            recovered_bolt11.payment_secret().0
        );
        assert_eq!(
            original_bolt11.amount_milli_satoshis(),
            recovered_bolt11.amount_milli_satoshis()
        );
    }

    #[test]
    fn test_candid_to_bolt11_with_signature() {
        let bolt11 = create_test_bolt11("Signature test", Some(75000));

        // Mock signature bytes (65 bytes: 1 recovery_id + 64 signature)
        let signature_bytes = vec![0u8; 65];

        let candid = bolt11_to_candid(&bolt11, Some(signature_bytes.clone()), None);

        assert_eq!(candid.signature, signature_bytes);

        // Should still roundtrip correctly
        let recovered = candid_to_bolt11(&candid).unwrap();
        assert_eq!(bolt11.to_string(), recovered.to_string());
    }

    #[test]
    fn test_candid_to_bolt11_with_channel_id() {
        let bolt11 = create_test_bolt11("Channel ID test", Some(25000));

        let channel_id = vec![42u8; 32];

        let candid = bolt11_to_candid(&bolt11, None, Some(channel_id.clone()));

        assert_eq!(candid.channel_id, channel_id);

        // Should still roundtrip correctly
        let recovered = candid_to_bolt11(&candid).unwrap();
        assert_eq!(bolt11.to_string(), recovered.to_string());
    }

    #[test]
    fn test_zero_amount_invoice() {
        // BOLT11 allows zero-amount invoices (amount specified at payment time)
        let bolt11 = create_test_bolt11("Zero amount", None);

        let candid = bolt11_to_candid(&bolt11, None, None);

        assert_eq!(candid.amount_msat, None);

        // Roundtrip
        let recovered = candid_to_bolt11(&candid).unwrap();
        assert_eq!(bolt11.to_string(), recovered.to_string());
        assert_eq!(recovered.amount_milli_satoshis(), None);
    }

    #[test]
    fn test_ic_swap_invoice_description() {
        // Test with IC swap format
        let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let description = format!("ckBTC_SWAP:{}", principal);
        let bolt11 = create_test_bolt11(&description, Some(50000));

        let candid = bolt11_to_candid(&bolt11, None, None);

        // Verify description is preserved in BOLT11 string
        let recovered = candid_to_bolt11(&candid).unwrap();

        // Extract description from recovered invoice
        use lightning_invoice::Bolt11InvoiceDescriptionRef;
        let recovered_desc = match recovered.description() {
            Bolt11InvoiceDescriptionRef::Direct(d) => d.to_string(),
            _ => panic!("Expected direct description"),
        };

        assert_eq!(recovered_desc, description);
    }

    #[test]
    fn test_invalid_bolt11_string() {
        let candid = SignedCandidInvoice {
            invoice: "invalid_bolt11_string".to_string(),
            amount_msat: Some(Nat::from(1000u64)),
            payment_hash: vec![0u8; 32],
            payment_secret: vec![0u8; 32],
            timestamp: 1234567890,
            expiry_secs: Some(3600),
            currency: "Bitcoin".to_string(),
            channel_id: vec![0u8; 32],
            signature: vec![],
        };

        let result = candid_to_bolt11(&candid);
        assert!(result.is_err());
        match result {
            Err(CandidConversionError::InvalidBolt11(_)) => {}
            _ => panic!("Expected InvalidBolt11 error"),
        }
    }

    #[test]
    fn test_field_mismatch_detection() {
        let bolt11 = create_test_bolt11("Mismatch test", Some(50000));
        let mut candid = bolt11_to_candid(&bolt11, None, None);

        // Corrupt the payment hash
        candid.payment_hash[0] ^= 0xFF;

        let result = candid_to_bolt11(&candid);
        assert!(result.is_err());
        match result {
            Err(CandidConversionError::FieldMismatch(_)) => {}
            _ => panic!("Expected FieldMismatch error"),
        }
    }

    #[test]
    fn test_large_amount() {
        // Test with large amount (2^32 satoshis)
        let large_amount_msat = 4_294_967_296_000u64; // 2^32 sats in msat
        let bolt11 = create_test_bolt11("Large amount", Some(large_amount_msat));

        let candid = bolt11_to_candid(&bolt11, None, None);

        assert_eq!(candid.amount_msat, Some(Nat::from(large_amount_msat)));

        // Roundtrip
        let recovered = candid_to_bolt11(&candid).unwrap();
        assert_eq!(recovered.amount_milli_satoshis(), Some(large_amount_msat));
    }

    #[test]
    fn test_multiple_roundtrips() {
        // Verify that multiple roundtrips don't corrupt data
        let bolt11 = create_test_bolt11("Multi-roundtrip", Some(123456));

        let mut current_bolt11 = bolt11.clone();

        for _ in 0..5 {
            let candid = bolt11_to_candid(&current_bolt11, None, None);
            current_bolt11 = candid_to_bolt11(&candid).unwrap();
        }

        // Should still match original
        assert_eq!(bolt11.to_string(), current_bolt11.to_string());
    }
}
