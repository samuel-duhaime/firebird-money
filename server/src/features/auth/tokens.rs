//! Generating and hashing the random tokens behind magic links and sessions.

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// A fresh random token. UUID v4 is 122 bits from the OS CSPRNG, on par with a standard session
/// id, and avoids pulling in another crate just to draw random bytes.
pub fn generate() -> String {
    Uuid::new_v4().simple().to_string()
}

/// SHA-256 of a token, hex encoded. Only this ever reaches the database: the raw token lives in
/// the emailed link or the user's cookie, so a leaked dump can't be replayed as a login.
///
/// Unlike a password, a token is long and random, so a plain hash (no salt, no work factor) is
/// enough — there is nothing to brute force.
pub fn hash(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_a_different_token_every_time() {
        assert_ne!(generate(), generate());
    }

    #[test]
    fn hashes_are_stable_and_hex() {
        let token = "3f9a2b7c";
        assert_eq!(hash(token), hash(token));
        assert_eq!(hash(token).len(), 64);
        assert!(hash(token).chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn different_tokens_hash_differently() {
        assert_ne!(hash("one"), hash("two"));
    }
}
