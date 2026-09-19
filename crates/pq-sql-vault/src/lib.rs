//! A SQL Server-backed vault for post-quantum-sealed values, alongside
//! (or instead of) the Redis-backed `pq-cache`.
//!
//! # Division of responsibility
//!
//! SQL Server stores only opaque, already-sealed bytes (`pq-crypto`'s
//! [`Envelope`]) and operational metadata (which key version sealed
//! each row, when, expiry). It performs **no cryptography**: ML-KEM and
//! AES-GCM stay entirely in `pq-crypto`, in Rust. T-SQL is a policy and
//! enforcement layer -- see `sql/002_procedures.sql`, whose stored
//! procedures refuse to seal new data under a retired key version, and
//! refuse to retire the currently-active version without a replacement
//! promoted first, both enforced in the database independent of
//! whether the calling application remembers to check. This split
//! (real cryptography only in an audited library, T-SQL only for
//! policy) is deliberate; do not add cryptographic primitives to the
//! T-SQL side.
//!
//! # Environment note
//!
//! This crate's SQL is written against Microsoft SQL Server (T-SQL:
//! `MERGE`, `SYSUTCDATETIME()`, filtered unique indexes, `THROW`,
//! `CREATE SECURITY POLICY`). It has **not** been exercised against a
//! live SQL Server instance in this repository's CI or development
//! environment (no SQL Server is available there). The Rust code below
//! is exercised by unit tests that don't require a live connection;
//! `tests/live_vault.rs` contains a real integration test that runs
//! only when `PQ_SQL_VAULT_TEST_CONNECTION_STRING` is set to a real
//! server's ADO connection string.
#![forbid(unsafe_code)]

use pq_crypto::{Envelope, KeyRing};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use tiberius::{Client, Config};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

type SqlClient = Client<Compat<TcpStream>>;

/// A vault backed by a SQL Server database, sealing/opening entries
/// with an owned [`KeyRing`] exactly like `pq-cache`'s `EncryptedCache`.
pub struct SqlVault {
    client: SqlClient,
    key_ring: KeyRing,
}

#[derive(Debug)]
pub enum VaultError {
    Sql(tiberius::error::Error),
    Io(std::io::Error),
    Crypto(pq_crypto::Error),
    /// The ADO connection string couldn't be parsed.
    ConnectionString(String),
    /// A query returned rows in a shape this crate didn't expect
    /// (should be unreachable against the schema in `sql/`, but this
    /// crate would rather report that clearly than panic on a `None`
    /// from `Row::get`).
    UnexpectedRowShape(&'static str),
}

impl fmt::Display for VaultError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VaultError::Sql(e) => write!(f, "SQL error: {}", e),
            VaultError::Io(e) => write!(f, "I/O error: {}", e),
            VaultError::Crypto(e) => write!(f, "envelope sealing/opening failed: {}", e),
            VaultError::ConnectionString(e) => write!(f, "invalid connection string: {}", e),
            VaultError::UnexpectedRowShape(what) => write!(f, "unexpected row shape: {}", what),
        }
    }
}
impl std::error::Error for VaultError {}

impl From<tiberius::error::Error> for VaultError {
    fn from(e: tiberius::error::Error) -> Self {
        VaultError::Sql(e)
    }
}
impl From<std::io::Error> for VaultError {
    fn from(e: std::io::Error) -> Self {
        VaultError::Io(e)
    }
}

impl SqlVault {
    /// Connect using a standard ADO.NET-style connection string, e.g.
    /// `"Server=tcp:host,1433;Database=db;User Id=user;Password=pass;TrustServerCertificate=false;Encrypt=true;"`.
    /// `key_ring` seals/opens every value stored through this vault.
    pub async fn connect(
        ado_connection_string: &str,
        key_ring: KeyRing,
    ) -> Result<Self, VaultError> {
        let config = Config::from_ado_string(ado_connection_string)
            .map_err(|e| VaultError::ConnectionString(e.to_string()))?;

        let tcp = TcpStream::connect(config.get_addr()).await?;
        tcp.set_nodelay(true)?;

        let client = Client::connect(config, tcp.compat_write()).await?;
        Ok(SqlVault { client, key_ring })
    }

    /// Mutable access to the key ring, for `register`/`set_active`/`retire`
    /// operations against a live vault (mirroring the equivalent calls in
    /// `sql/002_procedures.sql`, which must be kept in sync with these --
    /// see [`SqlVault::register_key_version`] etc. below, which do both).
    pub fn key_ring_mut(&mut self) -> &mut KeyRing {
        &mut self.key_ring
    }

    /// Fetch and open a sealed value. Returns `Ok(None)` both for a
    /// genuine miss (no row, or the row's TTL expired -- filtered by
    /// `pq.sp_get_encrypted_object` itself) and for a row that exists
    /// but fails to decrypt (foreign data, or sealed under a
    /// since-retired key version); the latter is logged at `warn`.
    pub async fn get(&mut self, object_key: &str) -> Result<Option<Vec<u8>>, VaultError> {
        let stream = self
            .client
            .query(
                "EXEC pq.sp_get_encrypted_object @ObjectKey = @P1",
                &[&object_key],
            )
            .await?;
        let Some(row) = stream.into_row().await? else {
            return Ok(None);
        };

        let key_version: i32 = row
            .get(0)
            .ok_or(VaultError::UnexpectedRowShape("missing KeyVersionId"))?;
        let kem_ciphertext: &[u8] = row
            .get(1)
            .ok_or(VaultError::UnexpectedRowShape("missing KemCiphertext"))?;
        let nonce: &[u8] = row
            .get(2)
            .ok_or(VaultError::UnexpectedRowShape("missing Nonce"))?;
        let aead_ciphertext: &[u8] = row
            .get(3)
            .ok_or(VaultError::UnexpectedRowShape("missing AeadCiphertext"))?;

        let envelope = Envelope {
            key_version: key_version as u32,
            kem_ciphertext: kem_ciphertext.to_vec(),
            nonce: nonce.to_vec(),
            aead_ciphertext: aead_ciphertext.to_vec(),
        };

        match self.key_ring.open(&envelope) {
            Ok(plaintext) => Ok(Some(plaintext)),
            Err(e) => {
                tracing::warn!(object_key, error = %e, "vault entry failed to decrypt; treating as miss");
                Ok(None)
            }
        }
    }

    /// Seal `value` under the ring's active key version and upsert it,
    /// expiring `ttl_seconds` from now (or never, if `ttl_seconds <= 0`).
    pub async fn set_with_ttl(
        &mut self,
        object_key: &str,
        value: &[u8],
        ttl_seconds: i64,
    ) -> Result<(), VaultError> {
        let envelope = self.key_ring.seal(value).map_err(VaultError::Crypto)?;
        let key_version = envelope.key_version as i32;
        let expires_at_unix_seconds: Option<i64> = if ttl_seconds > 0 {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            Some(now + ttl_seconds)
        } else {
            None
        };

        self.client
            .execute(
                "EXEC pq.sp_upsert_encrypted_object \
                 @ObjectKey = @P1, @KeyVersionId = @P2, @KemCiphertext = @P3, \
                 @Nonce = @P4, @AeadCiphertext = @P5, @ExpiresAtUnixSeconds = @P6",
                &[
                    &object_key,
                    &key_version,
                    &envelope.kem_ciphertext.as_slice(),
                    &envelope.nonce.as_slice(),
                    &envelope.aead_ciphertext.as_slice(),
                    &expires_at_unix_seconds,
                ],
            )
            .await?;
        Ok(())
    }

    /// Register a new key version with the database (`pq.sp_register_key_version`)
    /// *and* insert it into the in-memory [`KeyRing`], so the two stay in
    /// sync. Call this after generating a new [`pq_crypto::KeyPair`] as
    /// part of a rotation.
    pub async fn register_key_version(
        &mut self,
        version: u32,
        key_pair: pq_crypto::KeyPair,
        description: Option<&str>,
        make_active: bool,
    ) -> Result<(), VaultError> {
        self.client
            .execute(
                "EXEC pq.sp_register_key_version @KeyVersionId = @P1, @Description = @P2, @MakeActive = @P3",
                &[&(version as i32), &description, &make_active],
            )
            .await?;

        self.key_ring.insert(
            version,
            key_pair,
            if make_active {
                pq_crypto::KeyStatus::Active
            } else {
                pq_crypto::KeyStatus::DecryptOnly
            },
        );
        if make_active {
            self.key_ring
                .set_active(version)
                .map_err(VaultError::Crypto)?;
        }
        Ok(())
    }

    /// Promote `version` to Active in both the database
    /// (`pq.sp_set_active_key_version`) and the in-memory [`KeyRing`].
    pub async fn set_active_key_version(&mut self, version: u32) -> Result<(), VaultError> {
        self.client
            .execute(
                "EXEC pq.sp_set_active_key_version @KeyVersionId = @P1",
                &[&(version as i32)],
            )
            .await?;
        self.key_ring
            .set_active(version)
            .map_err(VaultError::Crypto)
    }

    /// Retire `version` in both the database (`pq.sp_retire_key_version`)
    /// and the in-memory [`KeyRing`].
    pub async fn retire_key_version(&mut self, version: u32) -> Result<(), VaultError> {
        self.client
            .execute(
                "EXEC pq.sp_retire_key_version @KeyVersionId = @P1",
                &[&(version as i32)],
            )
            .await?;
        self.key_ring.retire(version);
        Ok(())
    }

    /// Liveness check (used by health endpoints).
    pub async fn ping(&mut self) -> Result<(), VaultError> {
        self.client.query("SELECT 1", &[]).await?.into_row().await?;
        Ok(())
    }
}

/// Build an ADO connection string from discrete parts -- a small
/// convenience over hand-formatting one, and a single place that gets
/// the escaping/format right rather than every call site.
///
/// `encrypt` should be `true` outside local development (see
/// `docs/HARDENING.md` "Threat model" -- `pq-crypto` encrypting values
/// before they reach SQL Server is not a substitute for transport
/// security to the database itself).
pub fn build_ado_connection_string(
    host: &str,
    port: u16,
    database: &str,
    user: &str,
    password: &str,
    encrypt: bool,
    trust_server_certificate: bool,
) -> String {
    format!(
        "Server=tcp:{host},{port};Database={database};User Id={user};Password={password};Encrypt={encrypt};TrustServerCertificate={trust_server_certificate};"
    )
}

/// Like [`build_ado_connection_string`], but with the password redacted
/// -- for logging the resolved config the same way
/// `photo_cache_service::redact_url_credentials` does for `REDIS_URL`.
pub fn redact_ado_connection_string(connection_string: &str) -> String {
    connection_string
        .split(';')
        .map(|part| {
            if part
                .trim_start()
                .to_ascii_lowercase()
                .starts_with("password=")
            {
                "Password=***"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_ado_connection_string_has_expected_shape() {
        let s = build_ado_connection_string(
            "db.example.internal",
            1433,
            "vault",
            "svc_pq",
            "hunter2",
            true,
            false,
        );
        assert!(s.contains("Server=tcp:db.example.internal,1433"));
        assert!(s.contains("Database=vault"));
        assert!(s.contains("User Id=svc_pq"));
        assert!(s.contains("Password=hunter2"));
        assert!(s.contains("Encrypt=true"));
        assert!(s.contains("TrustServerCertificate=false"));
    }

    #[test]
    fn redaction_hides_the_password_but_keeps_everything_else() {
        let s = build_ado_connection_string("host", 1433, "db", "user", "hunter2", true, false);
        let redacted = redact_ado_connection_string(&s);
        assert!(!redacted.contains("hunter2"));
        assert!(redacted.contains("Password=***"));
        assert!(redacted.contains("Server=tcp:host,1433"));
        assert!(redacted.contains("Database=db"));
    }

    #[test]
    fn redaction_is_case_insensitive_on_the_password_key() {
        let redacted =
            redact_ado_connection_string("server=tcp:host,1433;password=hunter2;database=db;");
        assert!(!redacted.contains("hunter2"));
    }

    #[test]
    fn connection_string_without_a_password_is_unchanged_by_redaction() {
        let s = "Server=tcp:host,1433;Database=db;IntegratedSecurity=true;";
        assert_eq!(redact_ado_connection_string(s), s);
    }
}
