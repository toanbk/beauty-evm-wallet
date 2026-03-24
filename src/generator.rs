// BIP39 mnemonic -> seed -> HD derivation -> ETH address pipeline

use bip39::Mnemonic;
use coins_bip32::prelude::*;
use k256::ecdsa::VerifyingKey;
use tiny_keccak::{Hasher, Keccak};

pub struct WalletInfo {
    pub mnemonic: String,
    pub private_key: String, // hex, no 0x prefix
    pub address: String,     // hex, no 0x prefix, lowercase
}

/// Generates a random BIP39 wallet and derives the ETH address at m/44'/60'/0'/0/0.
pub fn generate_wallet() -> Result<WalletInfo, Box<dyn std::error::Error + Send + Sync>> {
    // 1. Generate random 12-word mnemonic (uses OsRng internally)
    let mnemonic = Mnemonic::generate(12)
        .map_err(|e| format!("Mnemonic generation failed: {}", e))?;

    // 2. Derive 64-byte seed (empty passphrase)
    let seed = mnemonic.to_seed("");

    // 3. Create BIP32 root key from seed
    let root = XPriv::root_from_seed(&seed, None)
        .map_err(|e| format!("Root key derivation failed: {}", e))?;

    // 4. Derive ETH path m/44'/60'/0'/0/0
    let derived = root
        .derive_path("m/44'/60'/0'/0/0")
        .map_err(|e| format!("Path derivation failed: {}", e))?;

    // 5. Extract signing key (private key)
    let signing_key: &k256::ecdsa::SigningKey = derived.as_ref();
    let private_key_bytes = signing_key.to_bytes();

    // 6. Get uncompressed public key (65 bytes: 04 || x || y)
    let verifying_key = VerifyingKey::from(signing_key);
    let pubkey_point = verifying_key.to_encoded_point(false);
    let pubkey_bytes = pubkey_point.as_bytes();

    // 7. Keccak256 of pubkey bytes WITHOUT the 0x04 prefix (64 bytes)
    let mut hasher = Keccak::v256();
    hasher.update(&pubkey_bytes[1..]); // skip 0x04 prefix
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    // 8. ETH address = last 20 bytes of hash
    let address = hex::encode(&hash[12..]);
    let private_key = hex::encode(private_key_bytes);

    Ok(WalletInfo {
        mnemonic: mnemonic.to_string(),
        private_key,
        address,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_wallet_produces_valid_output() {
        let wallet = generate_wallet().unwrap();
        // Mnemonic should be 12 words
        assert_eq!(wallet.mnemonic.split_whitespace().count(), 12);
        // Private key should be 64 hex chars (32 bytes)
        assert_eq!(wallet.private_key.len(), 64);
        assert!(wallet.private_key.chars().all(|c| c.is_ascii_hexdigit()));
        // Address should be 40 hex chars (20 bytes)
        assert_eq!(wallet.address.len(), 40);
        assert!(wallet.address.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_wallet_unique() {
        let w1 = generate_wallet().unwrap();
        let w2 = generate_wallet().unwrap();
        assert_ne!(w1.address, w2.address);
        assert_ne!(w1.mnemonic, w2.mnemonic);
    }

    #[test]
    fn test_known_mnemonic_produces_known_address() {
        // "abandon" x11 + "about" is the standard BIP39 test vector
        // Expected ETH address at m/44'/60'/0'/0/0 (verified via iancoleman.io/bip39):
        // 0x9858EfFD232B4033E47d90003D41EC34EcaEda94
        let mnemonic: Mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
            .parse()
            .unwrap();
        let seed = mnemonic.to_seed("");
        let root = XPriv::root_from_seed(&seed, None).unwrap();
        let derived = root.derive_path("m/44'/60'/0'/0/0").unwrap();

        let signing_key: &k256::ecdsa::SigningKey = derived.as_ref();
        let verifying_key = VerifyingKey::from(signing_key);
        let pubkey = verifying_key.to_encoded_point(false);

        let mut hasher = Keccak::v256();
        hasher.update(&pubkey.as_bytes()[1..]);
        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);
        let address = hex::encode(&hash[12..]);

        assert_eq!(address, "9858effd232b4033e47d90003d41ec34ecaeda94");
    }
}
