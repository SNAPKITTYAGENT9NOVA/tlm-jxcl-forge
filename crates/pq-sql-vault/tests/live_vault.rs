//! A real integration test against a live SQL Server, gated behind an
//! env var rather than `#[ignore]` so it's opt-in-and-obvious rather
//! than silently skipped by a plain `cargo test`.
//!
//! This was **not** run in this repository's development environment
//! (no SQL Server available there -- see `src/lib.rs`'s crate docs).
//! To actually exercise it, apply `sql/001_schema.sql` through
//! `sql/002_procedures.sql` to a real SQL Server database, then run:
//!
//! ```sh
//! PQ_SQL_VAULT_TEST_CONNECTION_STRING="Server=tcp:host,1433;Database=db;User Id=sa;Password=...;TrustServerCertificate=true;" \
//!     cargo test -p pq-sql-vault --test live_vault -- --ignored --nocapture
//! ```
//!
//! (still passed as an explicit opt-in via `--ignored` even though the
//! test itself checks the env var, so a plain `cargo test --workspace`
//! never attempts a network connection nobody asked for.)

use pq_crypto::{KeyPair, KeyRing, KeyStatus};
use pq_sql_vault::SqlVault;
use std::path::PathBuf;

fn connection_string() -> Option<String> {
    std::env::var("PQ_SQL_VAULT_TEST_CONNECTION_STRING").ok()
}

fn sql_migrations_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sql")
}

#[tokio::test]
#[ignore = "requires a live SQL Server; see this file's module docs"]
async fn set_then_get_roundtrips_through_a_real_sql_server() {
    let Some(conn_str) = connection_string() else {
        eprintln!("PQ_SQL_VAULT_TEST_CONNECTION_STRING not set; skipping");
        return;
    };

    let mut ring = KeyRing::new();
    ring.insert(1, KeyPair::generate(), KeyStatus::Active);

    let mut vault = SqlVault::connect(&conn_str, ring)
        .await
        .expect("connect to test SQL Server");

    vault
        .set_with_ttl(
            "pq-sql-vault-live-test-key",
            b"hello from a real SQL Server",
            60,
        )
        .await
        .expect("set_with_ttl");

    let value = vault.get("pq-sql-vault-live-test-key").await.expect("get");
    assert_eq!(value, Some(b"hello from a real SQL Server".to_vec()));
}

#[tokio::test]
#[ignore = "requires a live SQL Server; see this file's module docs"]
async fn rotation_keeps_old_entries_readable_until_retired_against_a_real_server() {
    let Some(conn_str) = connection_string() else {
        eprintln!("PQ_SQL_VAULT_TEST_CONNECTION_STRING not set; skipping");
        return;
    };

    let mut ring = KeyRing::new();
    ring.insert(1001, KeyPair::generate(), KeyStatus::Active);
    let mut vault = SqlVault::connect(&conn_str, ring)
        .await
        .expect("connect to test SQL Server");

    vault
        .set_with_ttl("pq-sql-vault-rotation-test", b"sealed under v1001", 60)
        .await
        .expect("set_with_ttl under v1001");

    vault
        .register_key_version(1002, KeyPair::generate(), Some("rotation test"), true)
        .await
        .expect("register_key_version 1002");

    // Old entry is still readable during the grace period.
    assert_eq!(
        vault.get("pq-sql-vault-rotation-test").await.unwrap(),
        Some(b"sealed under v1001".to_vec())
    );

    vault.retire_key_version(1001).await.expect("retire 1001");

    // Now it's a miss, not an error.
    assert_eq!(vault.get("pq-sql-vault-rotation-test").await.unwrap(), None);
}

#[tokio::test]
#[ignore = "requires a live SQL Server; see this file's module docs"]
async fn connect_and_migrate_applies_schema_and_is_safe_to_call_repeatedly() {
    let Some(conn_str) = connection_string() else {
        eprintln!("PQ_SQL_VAULT_TEST_CONNECTION_STRING not set; skipping");
        return;
    };

    let mut ring = KeyRing::new();
    ring.insert(1, KeyPair::generate(), KeyStatus::Active);

    let mut vault = SqlVault::connect_and_migrate(&conn_str, ring, &sql_migrations_dir())
        .await
        .expect("connect_and_migrate should apply sql/*.sql against an empty test database");

    vault
        .set_with_ttl(
            "pq-sql-vault-migrated-key",
            b"schema came from run_migrations",
            60,
        )
        .await
        .expect("the vault's schema/procedures must exist and work after migrating");

    // Reconnecting and migrating again must be a safe no-op, not an
    // error (e.g. re-running sql/001_schema.sql's CREATE SCHEMA/TABLE
    // would fail loudly if applied-migration tracking weren't working).
    let mut ring2 = KeyRing::new();
    ring2.insert(1, KeyPair::generate(), KeyStatus::Active);
    let _vault2 = SqlVault::connect_and_migrate(&conn_str, ring2, &sql_migrations_dir())
        .await
        .expect("a second connect_and_migrate against the same database must be a no-op");
}
