-- pq-sql-vault: schema
--
-- SQL Server holds only sealed (post-quantum-encrypted) bytes and
-- operational metadata. It never sees plaintext and never performs any
-- cryptography itself -- T-SQL is a policy/enforcement layer here, not
-- a cryptographic implementation layer (ML-KEM/AES-GCM stay in
-- crates/pq-crypto). See docs/HARDENING.md "SQL vault" for the full
-- design rationale.
--
-- Run this once against a target database before 002_procedures.sql.

CREATE SCHEMA pq;
GO

-- One row per key *version*. Exactly one row may be 'Active' at a time
-- (enforced below by a filtered unique index, not just application
-- logic) -- that's the version pq-crypto's KeyRing seals new values
-- under. Older versions stay 'DecryptOnly' through a rotation's grace
-- period so already-sealed data stays readable, then move to 'Retired'.
CREATE TABLE pq.KeyVersion (
    KeyVersionId INT NOT NULL PRIMARY KEY,
    Status NVARCHAR(20) NOT NULL
        CONSTRAINT CK_KeyVersion_Status CHECK (Status IN (N'Active', N'DecryptOnly', N'Retired')),
    CreatedAtUtc DATETIME2 NOT NULL CONSTRAINT DF_KeyVersion_CreatedAtUtc DEFAULT SYSUTCDATETIME(),
    RetiredAtUtc DATETIME2 NULL,
    Description NVARCHAR(200) NULL
);
GO

-- Database-enforced invariant: at most one Active key version ever
-- exists. A filtered unique index is the standard SQL Server pattern
-- for "unique among rows matching a condition."
CREATE UNIQUE INDEX UX_KeyVersion_OneActive
    ON pq.KeyVersion (Status)
    WHERE Status = N'Active';
GO

-- One row per logical cache key. Upserted (never appended) by
-- pq.sp_upsert_encrypted_object, so a read always sees the latest
-- sealed value for that key -- the same semantics as a Redis SET.
--
-- Expiry is stored as Unix seconds (BIGINT), not DATETIME2, so the Rust
-- client never needs a datetime-conversion dependency to compute or
-- compare it; see pq-sql-vault's crate docs for that decision.
CREATE TABLE pq.EncryptedObject (
    Id BIGINT IDENTITY(1,1) PRIMARY KEY,
    ObjectKey NVARCHAR(400) NOT NULL,
    KeyVersionId INT NOT NULL
        CONSTRAINT FK_EncryptedObject_KeyVersion REFERENCES pq.KeyVersion(KeyVersionId),
    KemCiphertext VARBINARY(MAX) NOT NULL,
    Nonce VARBINARY(12) NOT NULL,
    AeadCiphertext VARBINARY(MAX) NOT NULL,
    CreatedAtUtc DATETIME2 NOT NULL CONSTRAINT DF_EncryptedObject_CreatedAtUtc DEFAULT SYSUTCDATETIME(),
    ExpiresAtUnixSeconds BIGINT NULL,
    CONSTRAINT UQ_EncryptedObject_ObjectKey UNIQUE (ObjectKey)
);
GO

CREATE INDEX IX_EncryptedObject_KeyVersionId ON pq.EncryptedObject (KeyVersionId);
GO

-- Explicit audit trail: every policy-relevant action (seal, rotate,
-- retire, register) is recorded here by the stored procedures in
-- 002_procedures.sql, independent of any application-level logging.
CREATE TABLE pq.AuditLog (
    Id BIGINT IDENTITY(1,1) PRIMARY KEY,
    ObjectKey NVARCHAR(400) NULL,
    KeyVersionId INT NULL,
    Action NVARCHAR(30) NOT NULL,
    PerformedAtUtc DATETIME2 NOT NULL CONSTRAINT DF_AuditLog_PerformedAtUtc DEFAULT SYSUTCDATETIME(),
    PerformedBy NVARCHAR(128) NOT NULL CONSTRAINT DF_AuditLog_PerformedBy DEFAULT SUSER_SNAME(),
    Detail NVARCHAR(400) NULL
);
GO
