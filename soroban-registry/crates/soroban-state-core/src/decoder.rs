/// XDR decoder for Soroban contract state values
use crate::types::DecodedValue;
use anyhow::{anyhow, Result};
use base64::Engine;
use std::string::String as StdString;
use std::vec::Vec as StdVec;
use stellar_xdr::curr::{ReadXdr, ScVal};

/// Decode a base64-encoded XDR SCVal into a human-readable value
pub fn decode_scval(xdr_base64: &str) -> Result<DecodedValue> {
    let engine = base64::engine::general_purpose::STANDARD;
    let xdr_bytes = engine
        .decode(xdr_base64)
        .map_err(|e| anyhow!("Failed to decode base64 XDR: {}", e))?;

    decode_scval_bytes(&xdr_bytes)
}

/// Decode XDR bytes directly into a DecodedValue
pub fn decode_scval_bytes(bytes: &[u8]) -> Result<DecodedValue> {
    let scval: ScVal = ScVal::from_xdr(bytes, stellar_xdr::curr::Limits::none())
        .map_err(|e| anyhow!("Failed to parse XDR: {}", e))?;

    decode_scval_native(&scval)
}

/// Decode a native ScVal into DecodedValue
pub fn decode_scval_native(scval: &ScVal) -> Result<DecodedValue> {
    use stellar_xdr::curr::ScVal::*;

    match scval {
        Bool(b) => Ok(DecodedValue::Bool(*b)),
        Void => Ok(DecodedValue::Void),
        Error(_e) => Ok(DecodedValue::Error("error".to_string())),
        U32(n) => Ok(DecodedValue::Uint32(*n)),
        I32(n) => Ok(DecodedValue::Int32(*n)),
        U64(n) => Ok(DecodedValue::Uint64(*n)),
        I64(n) => Ok(DecodedValue::Int64(*n)),
        U128(parts) => {
            let value = ((parts.hi as u128) << 64) | (parts.lo as u128);
            Ok(DecodedValue::Uint128(value))
        }
        I128(parts) => {
            let value = ((parts.hi as i128) << 64) | (parts.lo as i128);
            Ok(DecodedValue::Int128(value))
        }
        Timepoint(t) => Ok(DecodedValue::Uint64(t.clone().into())),
        Duration(d) => Ok(DecodedValue::Uint64(d.clone().into())),
        U256(parts) => {
            let hex_str = format!(
                "{:016x}{:016x}{:016x}{:016x}",
                parts.hi_hi, parts.hi_lo, parts.lo_hi, parts.lo_lo
            );
            Ok(DecodedValue::Bytes(hex_str))
        }
        I256(parts) => {
            let hex_str = format!(
                "{:016x}{:016x}{:016x}{:016x}",
                parts.hi_hi, parts.hi_lo, parts.lo_hi, parts.lo_lo
            );
            Ok(DecodedValue::Bytes(hex_str))
        }
        Bytes(b) => Ok(DecodedValue::Bytes(hex::encode(b.as_slice()))),
        String(s) => {
            let s_str = StdString::from_utf8_lossy(s.as_slice()).to_string();
            Ok(DecodedValue::String(s_str))
        }
        Symbol(s) => {
            let sym_str = StdString::from_utf8_lossy(s.as_slice()).to_string();
            Ok(DecodedValue::Symbol(sym_str))
        }
        Vec(Some(vec_items)) => {
            let mut decoded = StdVec::new();
            for item in vec_items.0.iter() {
                decoded.push(decode_scval_native(item)?);
            }
            Ok(DecodedValue::Vec(decoded))
        }
        Vec(None) => Ok(DecodedValue::Vec(StdVec::new())),
        Map(Some(map_items)) => {
            let mut decoded = StdVec::new();
            for entry in map_items.0.iter() {
                let key = decode_scval_native(&entry.key)?;
                let val = decode_scval_native(&entry.val)?;
                decoded.push((Box::new(key), Box::new(val)));
            }
            Ok(DecodedValue::Map(decoded))
        }
        Map(None) => Ok(DecodedValue::Map(StdVec::new())),
        Address(addr) => {
            let addr_str = format_address(addr);
            Ok(DecodedValue::Address(addr_str))
        }
        ContractInstance(_) => Ok(DecodedValue::String("[contract instance]".to_string())),
        LedgerKeyContractInstance => Ok(DecodedValue::String("[ledger key]".to_string())),
        LedgerKeyNonce(_) => Ok(DecodedValue::String("[nonce key]".to_string())),
    }
}

/// Format an address in Stellar strkey format
fn format_address(addr: &stellar_xdr::curr::ScAddress) -> StdString {
    use stellar_xdr::curr::ScAddress::*;
    match addr {
        Account(a) => match &a.0 {
            stellar_xdr::curr::PublicKey::PublicKeyTypeEd25519(key) => {
                format!("Account({})", hex::encode(key.0))
            }
        },
        // FIX: clone() the inner Hash value since it does not implement Copy
        Contract(c) => format!("Contract({})", hex::encode(c.0.clone())),
        MuxedAccount(m) => format!("MuxedAccount({})", hex::encode(m.ed25519.0)),
        ClaimableBalance(b) => match b {
            stellar_xdr::curr::ClaimableBalanceId::ClaimableBalanceIdTypeV0(hash) => {
                format!("ClaimableBalance({})", hex::encode(hash.0))
            }
        },
        // FIX: clone() the inner Hash value since it does not implement Copy
        LiquidityPool(p) => format!("LiquidityPool({})", hex::encode(p.0.clone())),
    }
}

/// Format a decoded value with optional indentation
pub fn format_decoded(value: &DecodedValue, indent: usize) -> StdString {
    let prefix = "  ".repeat(indent);
    match value {
        DecodedValue::Map(entries) => {
            let mut result = StdString::from("{\n");
            for (k, v) in entries {
                result.push_str(&format!(
                    "{}  {}: {},\n",
                    prefix,
                    format_decoded(k, indent + 1),
                    format_decoded(v, indent + 1)
                ));
            }
            result.push_str(&format!("{}}}", prefix));
            result
        }
        DecodedValue::Vec(items) => {
            let mut result = StdString::from("[\n");
            for item in items {
                result.push_str(&format!(
                    "{}  {},\n",
                    prefix,
                    format_decoded(item, indent + 1)
                ));
            }
            result.push_str(&format!("{}]", prefix));
            result
        }
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_decoded_simple() {
        let val = DecodedValue::Bool(true);
        let formatted = format_decoded(&val, 0);
        assert_eq!(formatted, "true");
    }

    #[test]
    fn test_format_decoded_vec() {
        let val = DecodedValue::Vec(vec![DecodedValue::Uint32(1), DecodedValue::Uint32(2)]);
        let formatted = format_decoded(&val, 0);
        assert!(formatted.contains("["));
        assert!(formatted.contains("]"));
    }

    #[test]
    fn test_decode_bool() {
        let scval = ScVal::Bool(true);
        let decoded = decode_scval_native(&scval).unwrap();
        assert!(matches!(decoded, DecodedValue::Bool(true)));
    }

    #[test]
    fn test_decode_uint128() {
        let scval = ScVal::U128(stellar_xdr::curr::UInt128Parts { hi: 0, lo: 1000 });
        let decoded = decode_scval_native(&scval).unwrap();
        assert!(matches!(decoded, DecodedValue::Uint128(1000)));
    }
}
