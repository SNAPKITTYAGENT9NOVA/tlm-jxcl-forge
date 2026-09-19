//! Chunked/streamed large-blob storage built on top of any
//! [`pq_storage::SealedStore`], splitting values above a size threshold
//! into several sealed chunks tied together by a small manifest.
//!
//! # Why chunk at all?
//!
//! Both existing [`pq_storage::SealedStore`] implementors have a
//! practical ceiling on a single value: Redis's default
//! `proto-max-bulk-len` is 512MB but individual large values still hurt
//! replication/latency long before that, and each `pq-sql-vault` row's
//! `VARBINARY(MAX)` columns, while technically unbounded, get
//! expensive to `MERGE`/transfer whole. Splitting a large object into
//! fixed-size chunks, each sealed and stored under its own key exactly
//! like any other [`SealedStore`] entry, keeps every individual write
//! small and bounded regardless of the backend or the object's total
//! size.
//!
//! # Threshold
//!
//! [`CHUNK_THRESHOLD_BYTES`] (512 KiB) is the cutoff: a value at or
//! under this size is stored inline in the manifest itself (one
//! `SealedStore` entry, no different from calling `set_with_ttl`
//! directly); a larger value is split into chunks of at most this many
//! bytes each, stored as separate `SealedStore` entries, with the
//! manifest recording only how many there are.
//!
//! # Manifest wire format
//!
//! The manifest is itself just a plaintext byte blob, sealed and stored
//! under the caller's own `key` like any other `SealedStore` entry (so
//! it benefits from the same at-rest encryption as everything else --
//! nothing here ever hands a backend unsealed bytes):
//!
//! ```text
//! [magic: 4 bytes = b"PQOS"][version: u8 = 1][mode: u8]
//! mode 0 (Inline): [len: u32 LE][data: len bytes]
//! mode 1 (Chunked): [total_len: u64 LE][chunk_size: u32 LE][chunk_count: u32 LE]
//! ```
//!
//! Chunk keys are derived deterministically from the object's own key
//! (`{key}::pq-object-store-chunk::{index:08}`) rather than stored
//! individually in the manifest -- one fewer thing that could drift out
//! of sync with `chunk_count`. This does mean an object key containing
//! that literal separator could collide with a chunk key; callers that
//! need namespace safety against adversarial keys should prefix their
//! own keys accordingly (the same caveat applies to any flat key-value
//! store layered with a naming convention).
//!
//! Owns: [`put_object`]/[`get_object`] and the chunk-manifest format.
#![forbid(unsafe_code)]

use pq_storage::SealedStore;
use std::fmt;

/// Values at or under this size are stored inline in the manifest
/// entry itself; larger values are split into chunks of at most this
/// many bytes each. 512 KiB balances "few enough round trips for a
/// large object" against "small enough that no single chunk write is
/// itself a large-value problem" for both existing backends (Redis,
/// SQL Server).
pub const CHUNK_THRESHOLD_BYTES: usize = 512 * 1024;

const MAGIC: [u8; 4] = *b"PQOS";
const VERSION: u8 = 1;
const MODE_INLINE: u8 = 0;
const MODE_CHUNKED: u8 = 1;

/// Errors from [`put_object`]/[`get_object`]. Generic over the
/// underlying [`SealedStore`]'s own error type `E`, since this crate
/// has no backend-specific error of its own to report beyond what the
/// store already reports.
#[derive(Debug)]
pub enum ObjectStoreError<E> {
    /// The underlying [`SealedStore`] returned an error.
    Store(E),
    /// The manifest entry existed but wasn't a well-formed manifest
    /// (wrong magic/version, or truncated) -- foreign or corrupted data
    /// under this key.
    MalformedManifest,
    /// The manifest named a chunk that [`SealedStore::get`] reported as
    /// missing (`Ok(None)`) -- a partially-written or since-evicted
    /// object.
    MissingChunk { key: String, index: u32 },
    /// All chunks were present, but their concatenated length didn't
    /// match the manifest's recorded total -- data corruption beyond
    /// what a plain missing chunk would produce.
    LengthMismatch { expected: u64, actual: u64 },
}

impl<E: fmt::Display> fmt::Display for ObjectStoreError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ObjectStoreError::Store(e) => write!(f, "underlying store error: {e}"),
            ObjectStoreError::MalformedManifest => {
                write!(f, "manifest entry is not a valid pq-object-store manifest")
            }
            ObjectStoreError::MissingChunk { key, index } => {
                write!(f, "chunk {index} (key {key:?}) is missing")
            }
            ObjectStoreError::LengthMismatch { expected, actual } => write!(
                f,
                "reassembled object length {actual} does not match manifest's recorded {expected}"
            ),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for ObjectStoreError<E> {}

impl<E> From<E> for ObjectStoreError<E> {
    fn from(e: E) -> Self {
        ObjectStoreError::Store(e)
    }
}

fn chunk_key(object_key: &str, index: u32) -> String {
    format!("{object_key}::pq-object-store-chunk::{index:08}")
}

fn encode_manifest(mode: u8, payload: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.push(VERSION);
    out.push(mode);
    payload(&mut out);
    out
}

enum Manifest {
    Inline(Vec<u8>),
    Chunked {
        total_len: u64,
        chunk_size: u32,
        chunk_count: u32,
    },
}

impl Manifest {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            Manifest::Inline(data) => encode_manifest(MODE_INLINE, |out| {
                out.extend_from_slice(&(data.len() as u32).to_le_bytes());
                out.extend_from_slice(data);
            }),
            Manifest::Chunked {
                total_len,
                chunk_size,
                chunk_count,
            } => encode_manifest(MODE_CHUNKED, |out| {
                out.extend_from_slice(&total_len.to_le_bytes());
                out.extend_from_slice(&chunk_size.to_le_bytes());
                out.extend_from_slice(&chunk_count.to_le_bytes());
            }),
        }
    }

    fn from_bytes<E>(bytes: &[u8]) -> Result<Self, ObjectStoreError<E>> {
        let malformed = || ObjectStoreError::MalformedManifest;

        if bytes.len() < 6 || bytes[0..4] != MAGIC {
            return Err(malformed());
        }
        if bytes[4] != VERSION {
            return Err(malformed());
        }
        let mode = bytes[5];
        let rest = &bytes[6..];

        match mode {
            MODE_INLINE => {
                let len_bytes: [u8; 4] = rest.get(0..4).ok_or_else(malformed)?.try_into().unwrap();
                let len = u32::from_le_bytes(len_bytes) as usize;
                let data = rest.get(4..4 + len).ok_or_else(malformed)?.to_vec();
                Ok(Manifest::Inline(data))
            }
            MODE_CHUNKED => {
                if rest.len() != 16 {
                    return Err(malformed());
                }
                let total_len = u64::from_le_bytes(rest[0..8].try_into().unwrap());
                let chunk_size = u32::from_le_bytes(rest[8..12].try_into().unwrap());
                let chunk_count = u32::from_le_bytes(rest[12..16].try_into().unwrap());
                Ok(Manifest::Chunked {
                    total_len,
                    chunk_size,
                    chunk_count,
                })
            }
            _ => Err(malformed()),
        }
    }
}

/// Seal and store `value` under `key`, transparently chunking it if
/// it's larger than [`CHUNK_THRESHOLD_BYTES`]. Every write this
/// produces (the manifest, and each chunk if any) shares the same
/// `ttl_seconds`, so an object's chunks and its manifest expire
/// together.
pub async fn put_object<S: SealedStore>(
    store: &mut S,
    key: &str,
    value: &[u8],
    ttl_seconds: u64,
) -> Result<(), ObjectStoreError<S::Error>> {
    if value.len() <= CHUNK_THRESHOLD_BYTES {
        let manifest = Manifest::Inline(value.to_vec());
        store
            .set_with_ttl(key, &manifest.to_bytes(), ttl_seconds)
            .await?;
        return Ok(());
    }

    let chunks: Vec<&[u8]> = value.chunks(CHUNK_THRESHOLD_BYTES).collect();
    for (index, chunk) in chunks.iter().enumerate() {
        store
            .set_with_ttl(&chunk_key(key, index as u32), chunk, ttl_seconds)
            .await?;
    }

    let manifest = Manifest::Chunked {
        total_len: value.len() as u64,
        chunk_size: CHUNK_THRESHOLD_BYTES as u32,
        chunk_count: chunks.len() as u32,
    };
    store
        .set_with_ttl(key, &manifest.to_bytes(), ttl_seconds)
        .await?;
    Ok(())
}

/// Fetch and reassemble the object stored under `key` by
/// [`put_object`]. `Ok(None)` means no manifest exists under `key` at
/// all (an ordinary miss, matching [`SealedStore::get`]'s own
/// convention). A manifest that exists but names a chunk that is
/// itself missing, or whose reassembled length doesn't match, is an
/// `Err` -- unlike a plain [`SealedStore`] miss, a *partially* present
/// multi-chunk object is data corruption worth surfacing, not silently
/// treating as absent.
pub async fn get_object<S: SealedStore>(
    store: &mut S,
    key: &str,
) -> Result<Option<Vec<u8>>, ObjectStoreError<S::Error>> {
    let Some(manifest_bytes) = store.get(key).await? else {
        return Ok(None);
    };
    let manifest = Manifest::from_bytes(&manifest_bytes)?;

    match manifest {
        Manifest::Inline(data) => Ok(Some(data)),
        Manifest::Chunked {
            total_len,
            chunk_count,
            ..
        } => {
            let mut out = Vec::with_capacity(total_len as usize);
            for index in 0..chunk_count {
                let key_for_chunk = chunk_key(key, index);
                let chunk =
                    store
                        .get(&key_for_chunk)
                        .await?
                        .ok_or(ObjectStoreError::MissingChunk {
                            key: key_for_chunk,
                            index,
                        })?;
                out.extend_from_slice(&chunk);
            }
            if out.len() as u64 != total_len {
                return Err(ObjectStoreError::LengthMismatch {
                    expected: total_len,
                    actual: out.len() as u64,
                });
            }
            Ok(Some(out))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// A real (if trivial) [`SealedStore`] test double: in-memory,
    /// unencrypted (sealing is each backend's own job, not this
    /// crate's or this double's), but a genuine implementor of the
    /// trait -- proving `put_object`/`get_object` work through the
    /// trait boundary itself, not against a special-cased backend.
    #[derive(Default)]
    struct MemoryStore {
        data: HashMap<String, Vec<u8>>,
    }

    #[derive(Debug, PartialEq, Eq)]
    struct MemoryStoreError;

    impl fmt::Display for MemoryStoreError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "memory store error")
        }
    }

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
            Ok(())
        }
    }

    #[tokio::test]
    async fn small_values_round_trip_inline() {
        let mut store = MemoryStore::default();
        put_object(&mut store, "k", b"small payload", 60)
            .await
            .unwrap();
        // Inline objects are a single SealedStore entry -- no chunk
        // keys were written.
        assert_eq!(store.data.len(), 1);
        assert_eq!(
            get_object(&mut store, "k").await.unwrap(),
            Some(b"small payload".to_vec())
        );
    }

    #[tokio::test]
    async fn a_value_exactly_at_the_threshold_stays_inline() {
        let mut store = MemoryStore::default();
        let value = vec![0xABu8; CHUNK_THRESHOLD_BYTES];
        put_object(&mut store, "k", &value, 60).await.unwrap();
        assert_eq!(store.data.len(), 1);
        assert_eq!(get_object(&mut store, "k").await.unwrap(), Some(value));
    }

    #[tokio::test]
    async fn a_value_larger_than_the_threshold_round_trips_through_real_chunking() {
        let mut store = MemoryStore::default();
        // A little over 2.5x the threshold, so it needs 3 chunks, the
        // last one partial -- exercises the boundary as well as
        // multi-chunk reassembly.
        let len = CHUNK_THRESHOLD_BYTES * 2 + CHUNK_THRESHOLD_BYTES / 2;
        let value: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();

        put_object(&mut store, "big", &value, 60).await.unwrap();

        // One manifest entry + 3 chunk entries.
        assert_eq!(store.data.len(), 4);

        let round_tripped = get_object(&mut store, "big").await.unwrap();
        assert_eq!(round_tripped, Some(value));
    }

    #[tokio::test]
    async fn missing_manifest_is_a_clean_none() {
        let mut store = MemoryStore::default();
        assert_eq!(get_object(&mut store, "never-put").await.unwrap(), None);
    }

    #[tokio::test]
    async fn a_missing_chunk_is_reported_as_an_error_not_a_silent_miss() {
        let mut store = MemoryStore::default();
        let value = vec![0x11u8; CHUNK_THRESHOLD_BYTES + 1];
        put_object(&mut store, "big", &value, 60).await.unwrap();

        // Simulate one chunk having been evicted/lost independently of
        // the manifest (e.g. a TTL edge case, or backend data loss).
        store.data.remove(&chunk_key("big", 0));

        let err = get_object(&mut store, "big").await.unwrap_err();
        assert!(matches!(
            err,
            ObjectStoreError::MissingChunk { index: 0, .. }
        ));
    }

    #[tokio::test]
    async fn a_foreign_non_manifest_value_is_a_malformed_manifest_error() {
        let mut store = MemoryStore::default();
        store
            .set_with_ttl("k", b"not a manifest at all", 60)
            .await
            .unwrap();
        assert!(matches!(
            get_object(&mut store, "k").await.unwrap_err(),
            ObjectStoreError::MalformedManifest
        ));
    }

    #[tokio::test]
    async fn overwriting_a_chunked_object_with_a_smaller_inline_one_still_reads_back_correctly() {
        let mut store = MemoryStore::default();
        let big = vec![0x22u8; CHUNK_THRESHOLD_BYTES * 2];
        put_object(&mut store, "k", &big, 60).await.unwrap();

        put_object(&mut store, "k", b"now small", 60).await.unwrap();
        assert_eq!(
            get_object(&mut store, "k").await.unwrap(),
            Some(b"now small".to_vec())
        );
    }

    #[test]
    fn error_display_messages_are_informative() {
        let e: ObjectStoreError<MemoryStoreError> = ObjectStoreError::MissingChunk {
            key: "k::pq-object-store-chunk::00000000".to_string(),
            index: 0,
        };
        assert!(e.to_string().contains("00000000"));

        let e: ObjectStoreError<MemoryStoreError> = ObjectStoreError::LengthMismatch {
            expected: 10,
            actual: 5,
        };
        assert!(e.to_string().contains('5'));
    }

    #[test]
    fn empty_value_round_trips_as_an_empty_inline_object() {
        // Pure manifest round trip, no async store needed.
        let manifest = Manifest::Inline(Vec::new());
        let bytes = manifest.to_bytes();
        match Manifest::from_bytes::<MemoryStoreError>(&bytes).unwrap() {
            Manifest::Inline(data) => assert!(data.is_empty()),
            Manifest::Chunked { .. } => panic!("expected inline"),
        }
    }
}
