// Suffix validation, hex matching, and optimized byte-level matching

/// Validates suffix is hex-only, returns normalized lowercase.
pub fn validate_suffix(suffix: &str) -> Result<String, String> {
    let lower = suffix.to_lowercase();
    if lower.is_empty() {
        return Err("Suffix cannot be empty".into());
    }
    if lower.len() > 40 {
        return Err("Suffix cannot exceed 40 hex chars (full ETH address)".into());
    }
    if !lower.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("Invalid hex in suffix: '{}'", suffix));
    }
    Ok(lower)
}

/// Converts validated hex suffix to raw bytes for byte-level matching.
/// Returns (bytes, is_nibble_aligned) where is_nibble_aligned means
/// the suffix has odd length and the first nibble needs special handling.
pub fn suffix_to_bytes(hex_suffix: &str) -> (Vec<u8>, bool) {
    let odd = hex_suffix.len() % 2 != 0;
    // Pad with leading zero if odd length for hex decode
    let padded = if odd {
        format!("0{}", hex_suffix)
    } else {
        hex_suffix.to_string()
    };
    let bytes = hex::decode(&padded).expect("suffix already validated as hex");
    (bytes, odd)
}

/// Fast byte-level suffix matching against raw 20-byte address.
/// Avoids hex encoding entirely — compares raw bytes in the hot loop.
#[inline]
pub fn matches_suffix_bytes(address: &[u8; 20], suffix_bytes: &[u8], odd_nibble: bool) -> bool {
    let suffix_len = suffix_bytes.len();
    if suffix_len == 0 || suffix_len > 20 {
        return false;
    }

    // Compare full bytes from the end
    let addr_start = 20 - suffix_len;
    let addr_tail = &address[addr_start..];

    if odd_nibble {
        // First byte: only compare lower nibble (suffix had odd hex length)
        if (addr_tail[0] & 0x0F) != (suffix_bytes[0] & 0x0F) {
            return false;
        }
        // Remaining bytes: exact match
        addr_tail[1..] == suffix_bytes[1..]
    } else {
        // All bytes: exact match
        addr_tail == suffix_bytes
    }
}

/// Legacy string-based matching (kept for tests).
#[inline]
pub fn matches_suffix(address: &str, suffix: &str) -> bool {
    address.ends_with(suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid() {
        assert_eq!(validate_suffix("1988").unwrap(), "1988");
        assert_eq!(validate_suffix("AbCd").unwrap(), "abcd");
        assert_eq!(validate_suffix("8888").unwrap(), "8888");
    }

    #[test]
    fn test_validate_invalid() {
        assert!(validate_suffix("").is_err());
        assert!(validate_suffix("xyz").is_err());
        assert!(validate_suffix("12g4").is_err());
    }

    #[test]
    fn test_validate_too_long() {
        let long = "a".repeat(41);
        assert!(validate_suffix(&long).is_err());
    }

    #[test]
    fn test_matches() {
        assert!(matches_suffix("abcdef1988", "1988"));
        assert!(matches_suffix("abcdef1988", "f1988"));
        assert!(!matches_suffix("abcdef1988", "1989"));
    }

    #[test]
    fn test_suffix_to_bytes_even() {
        let (bytes, odd) = suffix_to_bytes("1988");
        assert_eq!(bytes, vec![0x19, 0x88]);
        assert!(!odd);
    }

    #[test]
    fn test_suffix_to_bytes_odd() {
        let (bytes, odd) = suffix_to_bytes("abc");
        assert_eq!(bytes, vec![0x0a, 0xbc]);
        assert!(odd);
    }

    #[test]
    fn test_matches_suffix_bytes_even() {
        // Address ending in ...1988
        let mut addr = [0u8; 20];
        addr[18] = 0x19;
        addr[19] = 0x88;
        let (suffix, odd) = suffix_to_bytes("1988");
        assert!(matches_suffix_bytes(&addr, &suffix, odd));
        let (suffix2, odd2) = suffix_to_bytes("1989");
        assert!(!matches_suffix_bytes(&addr, &suffix2, odd2));
    }

    #[test]
    fn test_matches_suffix_bytes_odd() {
        // Address ending in ...0abc
        let mut addr = [0u8; 20];
        addr[18] = 0x0a;
        addr[19] = 0xbc;
        let (suffix, odd) = suffix_to_bytes("abc");
        assert!(matches_suffix_bytes(&addr, &suffix, odd));
        // "0abc" (even) should also match
        let (suffix2, odd2) = suffix_to_bytes("0abc");
        assert!(matches_suffix_bytes(&addr, &suffix2, odd2));
    }
}
