//! A real integration test against a live SQL Server, gated behind an
//! env var rather than `#[ignore]` alone (mirroring
//! `pq-sql-vault/tests/live_vault.rs`'s pattern) so it's opt-in-and-
//! obvious rather than silently skipped by a plain `cargo test`.
//!
//! This was **not** run in this repository's development environment
//! (no SQL Server available there -- see `docs/BASELINE.md`). To
//! actually exercise it, point it at an empty test database and run:
//!
//! ```sh
//! PQ_MIGRATION_TEST_CONNECTION_STRING="Server=tcp:host,1433;Database=db;User Id=sa;Password=...;TrustServerCertificate=true;" \
//!     cargo test -p pq-migration --test live_migration -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::TokioAsyncWriteCompatExt;

fn connection_string() -> Option<String> {
    std::env::var("PQ_MIGRATION_TEST_CONNECTION_STRING").ok()
}

fn pq_sql_vault_sql_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../pq-sql-vault/sql")
}

#[tokio::test]
#[ignore = "requires a live SQL Server; see this file's module docs"]
async fn run_migrations_applies_all_four_pq_sql_vault_migrations_and_is_idempotent() {
    let Some(conn_str) = connection_string() else {
        eprintln!("PQ_MIGRATION_TEST_CONNECTION_STRING not set; skipping");
        return;
    };

    let config = Config::from_ado_string(&conn_str).expect("parse connection string");
    let tcp = TcpStream::connect(config.get_addr())
        .await
        .expect("connect tcp");
    tcp.set_nodelay(true).expect("set_nodelay");
    let mut client = Client::connect(config, tcp.compat_write())
        .await
        .expect("connect to test SQL Server");

    pq_migration::run_migrations(&mut client, &pq_sql_vault_sql_dir())
        .await
        .expect("first run_migrations application");

    // Confirm the schema this migration set is supposed to create is
    // actually usable -- not just that no error was returned.
    client
        .execute(
            "EXEC pq.sp_register_key_version @KeyVersionId = @P1, @MakeActive = @P2",
            &[&9999i32, &true],
        )
        .await
        .expect("pq.sp_register_key_version should exist and work after migrating");

    // A second application must be a safe no-op: nothing here should
    // error (e.g. re-running sql/001_schema.sql's CREATE TABLE would
    // fail loudly if this weren't tracked correctly).
    pq_migration::run_migrations(&mut client, &pq_sql_vault_sql_dir())
        .await
        .expect("second run_migrations application must be a no-op, not an error");
}
