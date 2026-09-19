//! The `SealedStore` trait both `pq-cache` and `pq-sql-vault` implement,
//! letting callers (e.g. `pq-object-store`) be storage-backend-agnostic.
//!
//! # Signature note
//!
//! `SealedStore`'s methods mirror `pq-cache::EncryptedCache`'s and
//! `pq-sql-vault::SqlVault`'s *existing*, already-tested inherent
//! methods: both take `&mut self` (each holds a live network
//! connection), both treat a decrypt failure as `Ok(None)` rather than
//! an error, and both already have a `ping` liveness check. This crate
//! does not change how either backend seals or opens a value -- that
//! stays entirely inside each implementor, via its own key material --
//! it only names the common shape so generic code can be written once
//! against either backend.
//!
//! `pq-sql-vault::SqlVault`'s inherent `set_with_ttl` takes a signed
//! `i64` TTL (zero or negative means "never expires", to allow that
//! convention). `SealedStore::set_with_ttl` uses `u64` -- a TTL can't
//! sensibly be negative -- and `SqlVault`'s trait impl converts (`0u64`
//! still maps to "never expires").
#![forbid(unsafe_code)]

/// A storage backend whose entries are opaque, already-sealed bytes:
/// callers never see or provide ciphertext directly (each implementor
/// seals/opens values against its own key material internally), only
/// plaintext in and plaintext (or `None`) out.
///
/// Implemented by [`pq_cache::EncryptedCache`] (Redis-backed) and
/// [`pq_sql_vault::SqlVault`] (SQL-Server-backed); consumed generically
/// by `pq-object-store`.
///
/// Uses plain `async fn` rather than a `-> impl Future<...> + Send`
/// desugaring: every implementor and caller in this workspace runs on
/// one Tokio runtime, so the `Send`-across-await-points guarantee that
/// desugaring would buy isn't needed here, and the plain `async fn`
/// form is far more readable at every call site.
#[allow(async_fn_in_trait)]
pub trait SealedStore {
    /// This backend's error type (e.g. `pq-cache::CacheError`,
    /// `pq-sql-vault::VaultError`).
    type Error;

    /// Fetch and open a stored value. `Ok(None)` covers both a genuine
    /// miss and (by convention, matching both existing implementors) an
    /// entry that failed to decrypt -- decryption failure is logged by
    /// the implementor, not surfaced as an error, so a cache/vault stays
    /// safe to treat as "just empty" on decrypt failure.
    async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, Self::Error>;

    /// Seal and store `value` under `key`, expiring in `ttl_seconds`
    /// seconds (`0` means "no expiry", matching both implementors'
    /// convention).
    async fn set_with_ttl(
        &mut self,
        key: &str,
        value: &[u8],
        ttl_seconds: u64,
    ) -> Result<(), Self::Error>;

    /// Liveness check for the underlying connection (used by health
    /// endpoints).
    async fn ping(&mut self) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A trivial in-memory double, real enough to prove the trait is
    /// actually usable through a generic bound (not just declarable).
    /// `pq-object-store`'s tests exercise a very similar double more
    /// thoroughly against real chunking logic.
    #[derive(Default)]
    struct MemoryStore {
        data: HashMap<String, Vec<u8>>,
        pings: u32,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct MemoryStoreError(&'static str);

    impl SealedStore for MemoryStore {
        type Error = MemoryStoreError;

        async fn get(&mut self, key: &str) -> Result<Option<Vec<u8>>, Self::Error> {
            Ok(self.data.get(key).cloned())
        }

        async fn set_with_ttl(
            &mut self,
            key: &str,
            value: &[u8],
            _ttl_seconds: u64,
        ) -> Result<(), Self::Error> {
            self.data.insert(key.to_string(), value.to_vec());
            Ok(())
        }

        async fn ping(&mut self) -> Result<(), Self::Error> {
            self.pings += 1;
            Ok(())
        }
    }

    /// Written against the trait bound alone (not `MemoryStore`
    /// directly), to prove `SealedStore` is usable as a generic
    /// constraint the way `pq-object-store` uses it.
    async fn generic_round_trip<S: SealedStore>(store: &mut S) -> Result<(), S::Error> {
        assert_eq!(store.get("missing").await?, None);
        store.set_with_ttl("k", b"hello", 60).await?;
        assert_eq!(store.get("k").await?, Some(b"hello".to_vec()));
        store.ping().await?;
        Ok(())
    }

    #[tokio::test]
    async fn memory_store_round_trips_through_the_trait() {
        let mut store = MemoryStore::default();
        generic_round_trip(&mut store).await.unwrap();
        assert_eq!(store.pings, 1);
    }

    #[tokio::test]
    async fn zero_ttl_is_stored_and_readable_like_any_other_value() {
        let mut store = MemoryStore::default();
        store.set_with_ttl("k", b"v", 0).await.unwrap();
        assert_eq!(store.get("k").await.unwrap(), Some(b"v".to_vec()));
    }

    #[tokio::test]
    async fn overwriting_a_key_replaces_its_value() {
        let mut store = MemoryStore::default();
        store.set_with_ttl("k", b"first", 60).await.unwrap();
        store.set_with_ttl("k", b"second", 60).await.unwrap();
        assert_eq!(store.get("k").await.unwrap(), Some(b"second".to_vec()));
    }
}
