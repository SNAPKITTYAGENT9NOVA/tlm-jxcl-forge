// Copyright (C) 2026 ERRANT DIGITAL INSTITUTE OF TECHNOLOGY (SEIT NGO) % SNAPKITTY COLLECTIVE
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Commercial
// See LICENSE-AGPL and LICENSE-COMMERCIAL in this repository for license terms.

//! Integration tests against a real, locally spawned `redis-server`
//! (available in this environment) rather than a mock — matching this
//! workspace's preference for genuine end-to-end testing wherever it's
//! actually feasible.

use pq_cache::EncryptedCache;
use pq_crypto::KeyPair;
use pq_storage::SealedStore;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU16, Ordering};
use std::time::Duration;

// `cargo test` runs these concurrently by default, so each test needs
// its own server/port -- sharing one would make them flakily collide.
static NEXT_PORT: AtomicU16 = AtomicU16::new(16399);

/// Spawns `redis-server` for the test's duration and kills it on drop,
/// so a panicking assertion never leaks the child process.
struct RedisGuard {
    child: Child,
    port: u16,
}

impl RedisGuard {
    async fn start() -> Self {
        let port = NEXT_PORT.fetch_add(1, Ordering::SeqCst);
        let child = Command::new("redis-server")
            .args([
                "--port",
                &port.to_string(),
                "--save",
                "",
                "--appendonly",
                "no",
                "--daemonize",
                "no",
                "--bind",
                "127.0.0.1",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("redis-server must be installed in this environment");

        let url = format!("redis://127.0.0.1:{port}");
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            if tokio::time::Instant::now() > deadline {
                panic!("redis-server did not become ready within 5s");
            }
            if let Ok(client) = redis::Client::open(url.as_str()) {
                if let Ok(mut conn) = client.get_connection_manager().await {
                    if redis::cmd("PING")
                        .query_async::<String>(&mut conn)
                        .await
                        .is_ok()
                    {
                        break;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        RedisGuard { child, port }
    }

    fn url(&self) -> String {
        format!("redis://127.0.0.1:{}", self.port)
    }
}

impl Drop for RedisGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[tokio::test]
async fn set_then_get_roundtrips_through_encryption() {
    let redis = RedisGuard::start().await;
    let mut cache = EncryptedCache::connect(&redis.url(), KeyPair::generate())
        .await
        .unwrap();

    cache
        .set_with_ttl("greeting", b"hello, post-quantum world", 60)
        .await
        .unwrap();
    let value = cache.get("greeting").await.unwrap();
    assert_eq!(value, Some(b"hello, post-quantum world".to_vec()));
}

#[tokio::test]
async fn missing_key_is_a_clean_none() {
    let redis = RedisGuard::start().await;
    let mut cache = EncryptedCache::connect(&redis.url(), KeyPair::generate())
        .await
        .unwrap();

    assert_eq!(cache.get("never-set").await.unwrap(), None);
}

#[tokio::test]
async fn entry_expires_after_its_ttl() {
    let redis = RedisGuard::start().await;
    let mut cache = EncryptedCache::connect(&redis.url(), KeyPair::generate())
        .await
        .unwrap();

    cache
        .set_with_ttl("short-lived", b"soon gone", 1)
        .await
        .unwrap();
    assert_eq!(
        cache.get("short-lived").await.unwrap(),
        Some(b"soon gone".to_vec())
    );

    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(cache.get("short-lived").await.unwrap(), None);
}

#[tokio::test]
async fn foreign_non_envelope_value_degrades_to_a_miss_not_an_error() {
    let redis = RedisGuard::start().await;

    // Write raw garbage directly, bypassing EncryptedCache, to simulate
    // data that was never one of our envelopes.
    let client = redis::Client::open(redis.url().as_str()).unwrap();
    let mut raw_conn = client.get_connection_manager().await.unwrap();
    let _: () =
        redis::AsyncCommands::set(&mut raw_conn, "garbage", b"not an envelope at all".to_vec())
            .await
            .unwrap();

    let mut cache = EncryptedCache::connect(&redis.url(), KeyPair::generate())
        .await
        .unwrap();
    // Must not error or panic -- a corrupted/foreign entry is just a miss.
    assert_eq!(cache.get("garbage").await.unwrap(), None);
}

#[tokio::test]
async fn entry_sealed_by_a_different_keypair_is_a_miss_not_an_error() {
    let redis = RedisGuard::start().await;

    let mut writer = EncryptedCache::connect(&redis.url(), KeyPair::generate())
        .await
        .unwrap();
    writer
        .set_with_ttl("shared-key", b"written by writer's keypair", 60)
        .await
        .unwrap();

    // A different process (different key pair) reading the same key
    // cannot decrypt it, and must see a miss rather than an error.
    let mut reader = EncryptedCache::connect(&redis.url(), KeyPair::generate())
        .await
        .unwrap();
    assert_eq!(reader.get("shared-key").await.unwrap(), None);
}

#[tokio::test]
async fn ping_succeeds_against_a_live_server() {
    let redis = RedisGuard::start().await;
    let mut cache = EncryptedCache::connect(&redis.url(), KeyPair::generate())
        .await
        .unwrap();
    cache.ping().await.unwrap();
}

/// Written against the `SealedStore` trait bound alone (not
/// `EncryptedCache` directly), proving `EncryptedCache` is actually
/// usable as generic storage-agnostic code (e.g. `pq-object-store`)
/// would use it, against a real live Redis server.
async fn round_trip_through_sealed_store<S: SealedStore>(store: &mut S) -> Result<(), S::Error> {
    store.ping().await?;
    assert_eq!(store.get("missing").await?, None);
    store
        .set_with_ttl("through-trait", b"seen only via SealedStore", 60)
        .await?;
    assert_eq!(
        store.get("through-trait").await?,
        Some(b"seen only via SealedStore".to_vec())
    );
    Ok(())
}

#[tokio::test]
async fn encrypted_cache_round_trips_through_the_sealed_store_trait() {
    let redis = RedisGuard::start().await;
    let mut cache = EncryptedCache::connect(&redis.url(), KeyPair::generate())
        .await
        .unwrap();
    round_trip_through_sealed_store(&mut cache).await.unwrap();
}
