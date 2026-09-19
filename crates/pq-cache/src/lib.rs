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

use pq_crypto::{Envelope, KeyPair, KeyRing, KeyStatus};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::fmt;

pub struct EncryptedCache {
    conn: ConnectionManager,
    key_ring: KeyRing,
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
    /// Connect to Redis at `redis_url`, sealing/opening cache entries
    /// with `key_ring` (see [`pq_crypto::KeyRing`] for key rotation).
    pub async fn connect_with_ring(redis_url: &str, key_ring: KeyRing) -> Result<Self, CacheError> {
        let client = redis::Client::open(redis_url)?;
        let conn = client.get_connection_manager().await?;
        Ok(EncryptedCache { conn, key_ring })
    }

    /// Convenience for the common single-key-version case: wraps
    /// `keypair` in a one-entry, all-`Active` [`KeyRing`] under version
    /// `1`. Use [`EncryptedCache::connect_with_ring`] directly to manage
    /// multiple key versions / rotation.
    pub async fn connect(redis_url: &str, keypair: KeyPair) -> Result<Self, CacheError> {
        let mut ring = KeyRing::new();
        ring.insert(1, keypair, KeyStatus::Active);
        Self::connect_with_ring(redis_url, ring).await
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
        match self.key_ring.open(&envelope) {
            Ok(plaintext) => Ok(Some(plaintext)),
            Err(e) => {
                // Includes the ordinary post-rotation case of a
                // Retired key version -- still just a miss, never an
                // error: the cache will naturally repopulate under the
                // active key on the next write.
                tracing::warn!(cache_key = key, error = %e, "cache entry failed to decrypt; treating as miss");
                Ok(None)
            }
        }
    }

    /// Encrypt `value` under the ring's current active key version and
    /// store it with a TTL of `ttl_seconds`.
    pub async fn set_with_ttl(
        &mut self,
        key: &str,
        value: &[u8],
        ttl_seconds: u64,
    ) -> Result<(), CacheError> {
        let envelope = self.key_ring.seal(value).map_err(CacheError::Seal)?;
        let bytes = envelope.to_bytes();
        let _: () = self.conn.set_ex(key, bytes, ttl_seconds).await?;
        Ok(())
    }

    /// Mutable access to the underlying key ring, for performing
    /// rotation (`insert`/`set_active`/`retire`) against a live cache.
    pub fn key_ring_mut(&mut self) -> &mut KeyRing {
        &mut self.key_ring
    }

    /// Liveness check for the Redis connection (used by health endpoints).
    pub async fn ping(&mut self) -> Result<(), CacheError> {
        let _: String = redis::cmd("PING").query_async(&mut self.conn).await?;
        Ok(())
    }
}

/// [`pq_storage::SealedStore`] impl so generic code (e.g.
/// `pq-object-store`) can use an [`EncryptedCache`] without depending
/// on `pq-cache`/`redis` directly. This doesn't change how
/// `EncryptedCache` seals or opens a value -- every method here just
/// forwards to the existing, already-tested inherent method of the
/// same name.
impl pq_storage::SealedStore for EncryptedCache {
    type Error = CacheError;

    async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, Self::Error> {
        EncryptedCache::get(self, key).await
    }

    async fn set_with_ttl(
        &mut self,
        key: &str,
        value: &[u8],
        ttl_seconds: u64,
    ) -> Result<(), Self::Error> {
        EncryptedCache::set_with_ttl(self, key, value, ttl_seconds).await
    }

    async fn ping(&mut self) -> Result<(), Self::Error> {
        EncryptedCache::ping(self).await
    }
}

#[cfg(test)]
mod tests {
    //! Real integration tests against a locally spawned `redis-server`
    //! subprocess (spec-project convention carried over from `jxcl`:
    //! prefer genuine testing over mocks wherever it's actually
    //! feasible in the environment). See `tests/` for the harness.
}
