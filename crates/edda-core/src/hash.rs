use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

pub const HASH_BYTES: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Hash(pub [u8; HASH_BYTES]);

impl Hash {
    pub fn new(data: &[u8]) -> Self {
        let mut h = Sha256::new();
        h.update(data);
        Hash(h.finalize().into())
    }

    pub fn new_from_array(arr: [u8; HASH_BYTES]) -> Self {
        Hash(arr)
    }

    pub fn as_bytes(&self) -> &[u8; HASH_BYTES] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.to_hex()[..16])
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({}...)", &self.to_hex()[..16])
    }
}

/// Hash multiple byte slices together in one pass
pub fn hashv(parts: &[&[u8]]) -> Hash {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    Hash(h.finalize().into())
}
