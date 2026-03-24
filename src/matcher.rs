// Suffix validation and case-insensitive hex matching

/// Validates suffix is hex-only, returns normalized lowercase.
/// Rejects empty, non-hex chars, and length > 40 (full address).
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

/// Checks if address (lowercase hex, no 0x prefix) ends with suffix.
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
}
