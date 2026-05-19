// BIP39 mnemonic -> seed -> HD derivation -> ETH address pipeline
// Optimized: batch derivation amortizes PBKDF2 across multiple addresses,
// direct libsecp256k1 C binding for all EC math (no pure-Rust k256).

use crate::bip32;
use bip39::Mnemonic;
use secp256k1::{PublicKey, Secp256k1, Signing};
use tiny_keccak::{Hasher, Keccak};

/// High-level wallet info with formatted strings (only created on match).
pub struct WalletInfo {
    pub mnemonic: String,
    pub private_key: String, // hex, no 0x prefix
    pub address: String,     // hex, no 0x prefix, lowercase
    pub address_index: u32,
}

/// Raw wallet data with zero-copy bytes (used in hot loop).
pub struct RawWallet {
    pub mnemonic: Mnemonic,
    pub private_key: [u8; 32],
    pub address: [u8; 20],
    pub address_index: u32,
}

impl RawWallet {
    /// Convert to WalletInfo with string formatting (only call on match).
    pub fn to_wallet_info(&self) -> WalletInfo {
        WalletInfo {
            mnemonic: self.mnemonic.to_string(),
            private_key: hex::encode(self.private_key),
            address: hex::encode(self.address),
            address_index: self.address_index,
        }
    }
}

/// Computes the 20-byte ETH address from a secp256k1 secret key.
#[inline]
fn secret_key_to_address<C: Signing>(
    secp: &Secp256k1<C>,
    key: &secp256k1::SecretKey,
) -> [u8; 20] {
    let public_key = PublicKey::from_secret_key(secp, key);
    let pubkey_bytes = public_key.serialize_uncompressed(); // 65 bytes

    let mut hasher = Keccak::v256();
    hasher.update(&pubkey_bytes[1..]); // skip 0x04 prefix
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..]);
    address
}

/// Generates a batch of wallets from a single mnemonic.
/// Derives addresses at indices 0..batch_size from path m/44'/60'/0'/0/N.
/// PBKDF2 (the bottleneck) is done once; each additional index only costs
/// one HMAC-SHA512 + two EC multiplications.
///
/// Uses the global secp256k1 context (pre-initialized, zero per-call cost).
#[inline]
pub fn generate_batch(batch_size: u32) -> Result<Vec<RawWallet>, String> {
    let mnemonic = Mnemonic::generate(12).map_err(|e| format!("Mnemonic error: {}", e))?;

    // PBKDF2: the expensive step, done once per batch
    let seed = mnemonic.to_seed("");

    let secp = secp256k1::SECP256K1; // global context, no allocation

    // Derive m/44'/60'/0'/0 (once per mnemonic)
    let parent = bip32::derive_eth_account_parent(secp, &seed);

    // Derive addresses at each index (cheap: ~50µs each vs ~4ms for PBKDF2)
    let mut results = Vec::with_capacity(batch_size as usize);
    for i in 0..batch_size {
        let child_key = bip32::derive_address_index(secp, &parent, i);
        let address = secret_key_to_address(secp, &child_key);
        results.push(RawWallet {
            mnemonic: mnemonic.clone(),
            private_key: child_key.secret_bytes(),
            address,
            address_index: i,
        });
    }

    Ok(results)
}

/// Generates a single random wallet at index 0 (legacy API for tests).
pub fn generate_wallet() -> Result<WalletInfo, String> {
    let batch = generate_batch(1)?;
    Ok(batch.into_iter().next().unwrap().to_wallet_info())
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
        let mnemonic: Mnemonic = "abandon abandon abandon abandon abandon abandon \
            abandon abandon abandon abandon abandon about"
            .parse()
            .unwrap();
        let seed = mnemonic.to_seed("");
        let secp = secp256k1::SECP256K1;

        let parent = bip32::derive_eth_account_parent(secp, &seed);
        let child_key = bip32::derive_address_index(secp, &parent, 0);

        let address = secret_key_to_address(secp, &child_key);
        assert_eq!(
            hex::encode(address),
            "9858effd232b4033e47d90003d41ec34ecaeda94"
        );
    }

    #[test]
    fn test_batch_produces_unique_addresses() {
        let batch = generate_batch(5).unwrap();
        assert_eq!(batch.len(), 5);
        for (i, w) in batch.iter().enumerate() {
            assert_eq!(w.address_index, i as u32);
        }
        // All addresses from the same mnemonic should be different
        let addresses: Vec<_> = batch.iter().map(|w| w.address).collect();
        for i in 0..addresses.len() {
            for j in (i + 1)..addresses.len() {
                assert_ne!(addresses[i], addresses[j]);
            }
        }
    }

    #[test]
    fn test_batch_shares_mnemonic() {
        let batch = generate_batch(3).unwrap();
        let m = batch[0].mnemonic.to_string();
        for w in &batch {
            assert_eq!(w.mnemonic.to_string(), m);
        }
    }
}
