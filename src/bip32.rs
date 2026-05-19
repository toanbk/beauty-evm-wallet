// Minimal BIP32 HD key derivation using libsecp256k1 C bindings.
// Replaces coins_bip32 + k256 (pure Rust) with direct secp256k1 calls
// for faster EC point multiplication in the hot path.

use hmac::{Hmac, Mac};
use secp256k1::{PublicKey, Scalar, Secp256k1, SecretKey, Signing};
use sha2::Sha512;

type HmacSha512 = Hmac<Sha512>;

const HARDENED_BIT: u32 = 0x8000_0000;

/// Extended private key: secret key + chain code for BIP32 derivation.
pub struct ExtendedKey {
    pub secret_key: SecretKey,
    pub chain_code: [u8; 32],
}

/// Derives BIP32 master key from 64-byte seed.
/// HMAC-SHA512 with key "Bitcoin seed".
pub fn master_from_seed(seed: &[u8]) -> ExtendedKey {
    let mut mac =
        HmacSha512::new_from_slice(b"Bitcoin seed").expect("HMAC can take key of any size");
    mac.update(seed);
    let result = mac.finalize().into_bytes();

    let secret_key =
        SecretKey::from_slice(&result[..32]).expect("seed should produce valid secret key");
    let mut chain_code = [0u8; 32];
    chain_code.copy_from_slice(&result[32..]);

    ExtendedKey {
        secret_key,
        chain_code,
    }
}

/// Derives hardened child key (index must have hardened bit set).
/// Uses: HMAC-SHA512(chain_code, 0x00 || parent_key || index).
pub fn derive_hardened(parent: &ExtendedKey, index: u32) -> ExtendedKey {
    debug_assert!(index & HARDENED_BIT != 0, "index must be hardened");

    let mut mac =
        HmacSha512::new_from_slice(&parent.chain_code).expect("HMAC can take key of any size");
    mac.update(&[0x00]);
    mac.update(&parent.secret_key.secret_bytes());
    mac.update(&index.to_be_bytes());
    let result = mac.finalize().into_bytes();

    let mut il = [0u8; 32];
    il.copy_from_slice(&result[..32]);

    let tweak = Scalar::from_be_bytes(il).expect("HMAC output should be valid scalar");
    let child_key = parent
        .secret_key
        .add_tweak(&tweak)
        .expect("key addition should not overflow");

    let mut chain_code = [0u8; 32];
    chain_code.copy_from_slice(&result[32..]);

    ExtendedKey {
        secret_key: child_key,
        chain_code,
    }
}

/// Derives normal (non-hardened) child key.
/// Uses: HMAC-SHA512(chain_code, compressed_pubkey || index).
/// Requires secp256k1 context for public key derivation.
pub fn derive_normal<C: Signing>(
    secp: &Secp256k1<C>,
    parent: &ExtendedKey,
    index: u32,
) -> ExtendedKey {
    debug_assert!(index & HARDENED_BIT == 0, "index must not be hardened");

    let parent_pubkey = PublicKey::from_secret_key(secp, &parent.secret_key);
    let compressed = parent_pubkey.serialize(); // 33 bytes

    let mut mac =
        HmacSha512::new_from_slice(&parent.chain_code).expect("HMAC can take key of any size");
    mac.update(&compressed);
    mac.update(&index.to_be_bytes());
    let result = mac.finalize().into_bytes();

    let mut il = [0u8; 32];
    il.copy_from_slice(&result[..32]);

    let tweak = Scalar::from_be_bytes(il).expect("HMAC output should be valid scalar");
    let child_key = parent
        .secret_key
        .add_tweak(&tweak)
        .expect("key addition should not overflow");

    let mut chain_code = [0u8; 32];
    chain_code.copy_from_slice(&result[32..]);

    ExtendedKey {
        secret_key: child_key,
        chain_code,
    }
}

/// Derives the intermediate key at m/44'/60'/0'/0 from seed.
/// The 3 hardened levels need no EC math; the final normal level does.
/// This is the expensive part that should be done once per mnemonic.
pub fn derive_eth_account_parent<C: Signing>(
    secp: &Secp256k1<C>,
    seed: &[u8],
) -> ExtendedKey {
    let master = master_from_seed(seed);

    // m/44' -> m/44'/60' -> m/44'/60'/0' (3 hardened derivations, no EC math)
    let level1 = derive_hardened(&master, 44 | HARDENED_BIT);
    let level2 = derive_hardened(&level1, 60 | HARDENED_BIT);
    let level3 = derive_hardened(&level2, HARDENED_BIT); // 0'

    // m/44'/60'/0'/0 (normal derivation, needs EC multiplication)
    derive_normal(secp, &level3, 0)
}

/// Derives child key at address index N from the parent at m/44'/60'/0'/0.
/// This is the cheap part done for each address in the batch.
pub fn derive_address_index<C: Signing>(
    secp: &Secp256k1<C>,
    parent: &ExtendedKey,
    index: u32,
) -> SecretKey {
    derive_normal(secp, parent, index).secret_key
}

#[cfg(test)]
mod tests {
    use super::*;
    use bip39::Mnemonic;
    use tiny_keccak::{Hasher, Keccak};

    #[test]
    fn test_known_vector_matches_reference() {
        // "abandon ... about" mnemonic should produce known ETH address
        let mnemonic: Mnemonic = "abandon abandon abandon abandon abandon abandon \
            abandon abandon abandon abandon abandon about"
            .parse()
            .unwrap();
        let seed = mnemonic.to_seed("");
        let secp = Secp256k1::new();

        let parent = derive_eth_account_parent(&secp, &seed);
        let child_key = derive_address_index(&secp, &parent, 0);

        // Derive ETH address from private key
        let pubkey = PublicKey::from_secret_key(&secp, &child_key);
        let pubkey_bytes = pubkey.serialize_uncompressed();
        let mut hasher = Keccak::v256();
        hasher.update(&pubkey_bytes[1..]);
        let mut hash = [0u8; 32];
        hasher.finalize(&mut hash);
        let address = hex::encode(&hash[12..]);

        assert_eq!(address, "9858effd232b4033e47d90003d41ec34ecaeda94");
    }
}
