// BIP39 mnemonic -> seed -> HD derivation -> ETH address pipeline
// Optimized: raw byte output, libsecp256k1 C binding for EC math

use bip39::Mnemonic;
use coins_bip32::prelude::*;
use secp256k1::{PublicKey, SecretKey, Secp256k1};
use tiny_keccak::{Hasher, Keccak};

/// High-level wallet info with formatted strings (only created on match).
pub struct WalletInfo {
    pub mnemonic: String,
    pub private_key: String, // hex, no 0x prefix
    pub address: String,     // hex, no 0x prefix, lowercase
}

/// Raw wallet data with zero-copy bytes (used in hot loop).
pub struct RawWallet {
    pub mnemonic: Mnemonic,
    pub private_key: [u8; 32],
    pub address: [u8; 20], // raw address bytes, no hex encoding
}

impl RawWallet {
    /// Convert to WalletInfo with string formatting (only call on match).
    pub fn to_wallet_info(&self) -> WalletInfo {
        WalletInfo {
            mnemonic: self.mnemonic.to_string(),
            private_key: hex::encode(self.private_key),
            address: hex::encode(self.address),
        }
    }
}

/// Generates a random BIP39 wallet, returns raw bytes to avoid allocations in hot loop.
/// Uses libsecp256k1 C binding for faster EC point multiplication.
#[inline]
pub fn generate_raw() -> Result<RawWallet, Box<dyn std::error::Error + Send + Sync>> {
    // 1. Generate random 12-word mnemonic (uses OsRng internally)
    let mnemonic = Mnemonic::generate(12)
        .map_err(|e| format!("Mnemonic generation failed: {}", e))?;

    // 2. Derive 64-byte seed via PBKDF2 (empty passphrase)
    let seed = mnemonic.to_seed("");

    // 3. BIP32 HD derivation to get private key bytes
    let root = XPriv::root_from_seed(&seed, None)
        .map_err(|e| format!("Root key derivation failed: {}", e))?;
    let derived = root
        .derive_path("m/44'/60'/0'/0/0")
        .map_err(|e| format!("Path derivation failed: {}", e))?;

    // Extract raw private key bytes from k256 signing key
    let signing_key: &k256::ecdsa::SigningKey = derived.as_ref();
    let private_key_bytes: [u8; 32] = signing_key.to_bytes().into();

    // 4. Use libsecp256k1 C binding for faster pubkey derivation
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&private_key_bytes)
        .map_err(|e| format!("SecretKey creation failed: {}", e))?;
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let pubkey_bytes = public_key.serialize_uncompressed(); // 65 bytes: 04 || x || y

    // 5. Keccak256 of pubkey WITHOUT 0x04 prefix -> ETH address = last 20 bytes
    let mut hasher = Keccak::v256();
    hasher.update(&pubkey_bytes[1..]); // skip 0x04 prefix
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..]);

    Ok(RawWallet {
        mnemonic,
        private_key: private_key_bytes,
        address,
    })
}

/// Legacy API for backward compatibility with tests.
pub fn generate_wallet() -> Result<WalletInfo, Box<dyn std::error::Error + Send + Sync>> {
    generate_raw().map(|raw| raw.to_wallet_info())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_wallet_produces_valid_output() {
        let wallet = generate_wallet().unwrap();
        assert_eq!(wallet.mnemonic.split_whitespace().count(), 12);
        assert_eq!(wallet.private_key.len(), 64);
        assert!(wallet.private_key.chars().all(|c| c.is_ascii_hexdigit()));
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
        // Verify libsecp256k1 produces same result as k256
        let mnemonic: Mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
            .parse().unwrap();
        let seed = mnemonic.to_seed("");
        let root = XPriv::root_from_seed(&seed, None).unwrap();
        let derived = root.derive_path("m/44'/60'/0'/0/0").unwrap();

        let signing_key: &k256::ecdsa::SigningKey = derived.as_ref();
        let private_key_bytes: [u8; 32] = signing_key.to_bytes().into();

        // Use libsecp256k1 path
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(&private_key_bytes).unwrap();
        let public_key = PublicKey::from_secret_key(&secp, &secret_key);
        let pubkey_bytes = public_key.serialize_uncompressed();

        let mut hasher = Keccak::v256();
        hasher.update(&pubkey_bytes[1..]);
        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);
        let address = hex::encode(&hash[12..]);

        assert_eq!(address, "9858effd232b4033e47d90003d41ec34ecaeda94");
    }

    #[test]
    fn test_raw_wallet_to_wallet_info() {
        let raw = generate_raw().unwrap();
        let info = raw.to_wallet_info();
        assert_eq!(info.address, hex::encode(raw.address));
        assert_eq!(info.private_key, hex::encode(raw.private_key));
    }
}
