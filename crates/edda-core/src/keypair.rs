use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;

use crate::account::Pubkey;

pub struct Keypair {
    signing_key: SigningKey,
}

impl Keypair {
    pub fn generate() -> Self {
        Self { signing_key: SigningKey::generate(&mut OsRng) }
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { signing_key: SigningKey::from_bytes(&bytes) }
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    pub fn pubkey(&self) -> Pubkey {
        Pubkey(self.signing_key.verifying_key().to_bytes())
    }

    /// Sign arbitrary bytes — returns 64-byte Ed25519 signature
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        self.signing_key.sign(data).to_bytes().to_vec()
    }

    /// Verify a signature against a public key
    pub fn verify(pubkey: &Pubkey, data: &[u8], signature: &[u8]) -> bool {
        let vk = match VerifyingKey::from_bytes(&pubkey.0) {
            Ok(k) => k,
            Err(_) => return false,
        };
        if signature.len() != 64 {
            return false;
        }
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(signature);
        let sig = Signature::from_bytes(&sig_bytes);
        vk.verify(data, &sig).is_ok()
    }
}
