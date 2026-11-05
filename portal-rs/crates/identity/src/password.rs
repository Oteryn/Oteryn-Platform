//! Argon2id policy and Laravel PHC compatibility.
use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::SaltString,
};
/// OWASP minimum Argon2id profile: 19 MiB, two iterations, one lane.
#[derive(Debug, Clone, Copy)]
pub struct PasswordConfig {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}
impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            memory_kib: 19456,
            iterations: 2,
            parallelism: 1,
        }
    }
}
impl PasswordConfig {
    fn engine(self) -> Result<Argon2<'static>, String> {
        let params = Params::new(self.memory_kib, self.iterations, self.parallelism, Some(32))
            .map_err(|e| e.to_string())?;
        Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
    }
    pub fn hash(self, password: &str) -> Result<String, String> {
        let mut salt = [0u8; 16];
        getrandom::fill(&mut salt).map_err(|_| "secure randomness unavailable".to_owned())?;
        let salt = SaltString::encode_b64(&salt).map_err(|e| e.to_string())?;
        self.engine()?
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| e.to_string())
    }
    pub fn needs_rehash(self, hash: &str) -> bool {
        let Ok(h) = PasswordHash::new(hash) else {
            return true;
        };
        h.algorithm.as_str() != "argon2id"
            || h.version != Some(19)
            || h.params.get_decimal("m") != Some(self.memory_kib)
            || h.params.get_decimal("t") != Some(self.iterations)
            || h.params.get_decimal("p") != Some(self.parallelism)
            || h.hash.is_none_or(|v| v.len() != 32)
    }
}
pub fn valid(password: &str) -> bool {
    password.chars().count() >= 10 && password.len() <= 256
}
pub fn hash(password: &str) -> Result<String, String> {
    PasswordConfig::default().hash(password)
}
pub fn verify(password: &str, hash: &str) -> bool {
    let Ok(hash) = PasswordHash::new(hash) else {
        return false;
    };
    hash.algorithm.as_str() == "argon2id"
        && Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
}
pub fn needs_rehash(hash: &str) -> bool {
    PasswordConfig::default().needs_rehash(hash)
}
