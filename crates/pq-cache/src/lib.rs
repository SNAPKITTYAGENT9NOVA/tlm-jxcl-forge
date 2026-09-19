//! A Redis-backed cache whose values are sealed with post-quantum
//! envelope encryption (`pq-crypto`) before they ever leave the process.
//!
//! Redis itself never sees plaintext. A value that fails to decrypt —
//! whether because it's foreign data, corrupted, or was written under a
//! previous process's (now-discarded) key pair — is treated as a cache
//! miss rather than an error: a cache is an optimization, and losing an
//! entry is always safe, but surfacing decryption failures as request
//! errors would not be.
#![forbid(unsafe_code)]

use pq_crypto::{DecapsulationKey, EncapsulationKey, Envelope, KeyPair};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::fmt;

pub struct EncryptedCache {
    conn: ConnectionManager,
    encapsulation_key: EncapsulationKey,
    decapsulation_key: DecapsulationKey,
}

#[derive(Debug)]
pub enum CacheError {
    Redis(redis::RedisError),
    Seal(pq_crypto::Error),
}

impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CacheError::Redis(e) => write!(f, "redis error: {}", e),
            CacheError::Seal(e) => write!(f, "envelope sealing failed: {}", e),
        }
    }
}
impl std::error::Error for CacheError {}

impl From<redis::RedisError> for CacheError {
    fn from(e: redis::RedisError) -> Self {
        CacheError::Redis(e)
    }
}

impl EncryptedCache {
    /// Connect to Redis at `redis_url` and take ownership of `keypair`
    /// for sealing/opening cache entries.
    pub async fn connect(redis_url: &str, keypair: KeyPair) -> Result<Self, CacheError> {
        let client = redis::Client::open(redis_url)?;
        let conn = client.get_connection_manager().await?;
        Ok(EncryptedCache {
            conn,
            encapsulation_key: keypair.encapsulation_key,
            decapsulation_key: keypair.decapsulation_key,
        })
    }

    /// Fetch and decrypt a cache entry. Returns `Ok(None)` both for a
    /// genuine cache miss and for an entry that could not be decoded or
    /// decrypted (logged at `warn` level in the latter case).
    pub async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        let raw: Option<Vec<u8>> = self.conn.get(key).await?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let envelope = match Envelope::from_bytes(&raw) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(cache_key = key, error = %e, "cache entry is not a valid envelope; treating as miss");
                return Ok(None);
            }
        };
        match pq_crypto::open(&self.decapsulation_key, &envelope) {
            Ok(plaintext) => Ok(Some(plaintext)),
            Err(e) => {
                tracing::warn!(cache_key = key, error = %e, "cache entry failed to decrypt; treating as miss");
                Ok(None)
            }
        }
    }

    /// Encrypt `value` and store it with a TTL of `ttl_seconds`.
    pub async fn set_with_ttl(
        &mut self,
        key: &str,
        value: &[u8],
        ttl_seconds: u64,
    ) -> Result<(), CacheError> {
        let envelope = pq_crypto::seal(&self.encapsulation_key, value).map_err(CacheError::Seal)?;
        let bytes = envelope.to_bytes();
        let _: () = self.conn.set_ex(key, bytes, ttl_seconds).await?;
        Ok(())
    }

    /// Liveness check for the Redis connection (used by health endpoints).
    pub async fn ping(&mut self) -> Result<(), CacheError> {
        let _: String = redis::cmd("PING").query_async(&mut self.conn).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Real integration tests against a locally spawned `redis-server`
    //! subprocess (spec-project convention carried over from `jxcl`:
    //! prefer genuine testing over mocks wherever it's actually
    //! feasible in the environment). See `tests/` for the harness.
}
