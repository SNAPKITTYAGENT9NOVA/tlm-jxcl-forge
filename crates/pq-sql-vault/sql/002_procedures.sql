-- pq-sql-vault: stored procedures
--
-- Every write path that matters for security policy goes through one of
-- these procedures rather than a bare INSERT/UPDATE from the
-- application -- that's what makes the policy enforceable at the
-- database layer independent of the calling code (see
-- 004_signed_procedures.sql to additionally require callers to go
-- through these procs rather than being granted table access at all).
--
-- Run after 001_schema.sql.

-- Register a brand-new key version (e.g. right after pq-crypto's
-- KeyRing generates one). Starts DecryptOnly by default -- promote it
-- with pq.sp_set_active_key_version once you're ready to cut new
-- writes over to it, or pass @MakeActive = 1 to do both in one call.
CREATE OR ALTER PROCEDURE pq.sp_register_key_version
    @KeyVersionId INT,
    @Description NVARCHAR(200) = NULL,
    @MakeActive BIT = 0
AS
BEGIN
    SET NOCOUNT ON;
    SET XACT_ABORT ON;

    IF EXISTS (SELECT 1 FROM pq.KeyVersion WHERE KeyVersionId = @KeyVersionId)
        THROW 51030, 'KeyVersionId already registered.', 1;

    INSERT INTO pq.KeyVersion (KeyVersionId, Status, Description)
    VALUES (@KeyVersionId, N'DecryptOnly', @Description);

    INSERT INTO pq.AuditLog (KeyVersionId, Action, Detail)
    VALUES (@KeyVersionId, N'Register', @Description);

    IF @MakeActive = 1
        EXEC pq.sp_set_active_key_version @KeyVersionId = @KeyVersionId;
END
GO

-- Promote @KeyVersionId to Active, demoting whatever was Active before
-- it to DecryptOnly (never straight to Retired -- that's the separate,
-- deliberate pq.sp_retire_key_version step once a grace period ends).
CREATE OR ALTER PROCEDURE pq.sp_set_active_key_version
    @KeyVersionId INT
AS
BEGIN
    SET NOCOUNT ON;
    SET XACT_ABORT ON;

    DECLARE @Status NVARCHAR(20);
    SELECT @Status = Status FROM pq.KeyVersion WHERE KeyVersionId = @KeyVersionId;

    IF @Status IS NULL
        THROW 51010, 'Unknown KeyVersionId.', 1;
    IF @Status = N'Retired'
        THROW 51011, 'Cannot re-activate a retired key version.', 1;

    BEGIN TRAN;

    UPDATE pq.KeyVersion
        SET Status = N'DecryptOnly'
        WHERE Status = N'Active' AND KeyVersionId <> @KeyVersionId;

    UPDATE pq.KeyVersion
        SET Status = N'Active'
        WHERE KeyVersionId = @KeyVersionId;

    INSERT INTO pq.AuditLog (KeyVersionId, Action)
    VALUES (@KeyVersionId, N'SetActive');

    COMMIT TRAN;
END
GO

-- Retire a key version so it can no longer decrypt anything. Refuses to
-- retire the *current* Active version -- callers must promote a
-- replacement with pq.sp_set_active_key_version first, which is a
-- deliberate guardrail against accidentally locking out all future
-- reads *and* writes at once. Idempotent: retiring an already-retired
-- version is a no-op, not an error.
CREATE OR ALTER PROCEDURE pq.sp_retire_key_version
    @KeyVersionId INT
AS
BEGIN
    SET NOCOUNT ON;
    SET XACT_ABORT ON;

    DECLARE @Status NVARCHAR(20);
    SELECT @Status = Status FROM pq.KeyVersion WHERE KeyVersionId = @KeyVersionId;

    IF @Status IS NULL
        THROW 51020, 'Unknown KeyVersionId.', 1;
    IF @Status = N'Active'
        THROW 51021, 'Cannot retire the active key version -- call pq.sp_set_active_key_version to promote a replacement first.', 1;
    IF @Status = N'Retired'
        RETURN;

    UPDATE pq.KeyVersion
        SET Status = N'Retired', RetiredAtUtc = SYSUTCDATETIME()
        WHERE KeyVersionId = @KeyVersionId;

    INSERT INTO pq.AuditLog (KeyVersionId, Action)
    VALUES (@KeyVersionId, N'Retire');
END
GO

-- Upsert a sealed object. Validates the referenced key version exists
-- and is not Retired *before* writing -- the database-level guarantee
-- that "you cannot seal new data under a key that's been taken out of
-- service," independent of whether the calling application remembered
-- to check.
CREATE OR ALTER PROCEDURE pq.sp_upsert_encrypted_object
    @ObjectKey NVARCHAR(400),
    @KeyVersionId INT,
    @KemCiphertext VARBINARY(MAX),
    @Nonce VARBINARY(12),
    @AeadCiphertext VARBINARY(MAX),
    @ExpiresAtUnixSeconds BIGINT = NULL
AS
BEGIN
    SET NOCOUNT ON;
    SET XACT_ABORT ON;

    IF @ObjectKey IS NULL OR LEN(@ObjectKey) = 0
        THROW 51001, 'ObjectKey must not be empty.', 1;
    IF @Nonce IS NULL OR DATALENGTH(@Nonce) <> 12
        THROW 51002, 'Nonce must be exactly 12 bytes (AES-256-GCM).', 1;

    DECLARE @Status NVARCHAR(20);
    SELECT @Status = Status FROM pq.KeyVersion WHERE KeyVersionId = @KeyVersionId;

    IF @Status IS NULL
        THROW 51003, 'Unknown KeyVersionId.', 1;
    IF @Status = N'Retired'
        THROW 51004, 'Cannot seal new data under a retired key version.', 1;

    BEGIN TRAN;

    MERGE pq.EncryptedObject AS target
    USING (SELECT @ObjectKey AS ObjectKey) AS src
        ON target.ObjectKey = src.ObjectKey
    WHEN MATCHED THEN
        UPDATE SET
            KeyVersionId = @KeyVersionId,
            KemCiphertext = @KemCiphertext,
            Nonce = @Nonce,
            AeadCiphertext = @AeadCiphertext,
            CreatedAtUtc = SYSUTCDATETIME(),
            ExpiresAtUnixSeconds = @ExpiresAtUnixSeconds
    WHEN NOT MATCHED THEN
        INSERT (ObjectKey, KeyVersionId, KemCiphertext, Nonce, AeadCiphertext, ExpiresAtUnixSeconds)
        VALUES (@ObjectKey, @KeyVersionId, @KemCiphertext, @Nonce, @AeadCiphertext, @ExpiresAtUnixSeconds);

    INSERT INTO pq.AuditLog (ObjectKey, KeyVersionId, Action)
    VALUES (@ObjectKey, @KeyVersionId, N'Upsert');

    COMMIT TRAN;
END
GO

-- Fetch a sealed object, honoring expiry. Deliberately does NOT filter
-- by the referenced key's Status: a Retired key's ciphertext is still
-- returned as-is (SQL Server's job is to gate *writes* under a retired
-- key, not to pre-emptively hide old reads) -- the Rust-side KeyRing is
-- what will correctly refuse to decrypt it, distinguishing "retired"
-- from "corrupt" for the caller.
CREATE OR ALTER PROCEDURE pq.sp_get_encrypted_object
    @ObjectKey NVARCHAR(400)
AS
BEGIN
    SET NOCOUNT ON;

    SELECT
        KeyVersionId,
        KemCiphertext,
        Nonce,
        AeadCiphertext,
        CreatedAtUtc,
        ExpiresAtUnixSeconds
    FROM pq.EncryptedObject
    WHERE ObjectKey = @ObjectKey
      AND (
            ExpiresAtUnixSeconds IS NULL
            OR ExpiresAtUnixSeconds > DATEDIFF_BIG(SECOND, '1970-01-01', SYSUTCDATETIME())
          );
END
GO

-- Maintenance: delete objects past their expiry. SQL Server has no
-- native per-row TTL like Redis's EXPIRE, so this is meant to be run
-- periodically by a SQL Agent job (or equivalent scheduler) -- this
-- repository does not and cannot schedule that itself.
CREATE OR ALTER PROCEDURE pq.sp_purge_expired_objects
AS
BEGIN
    SET NOCOUNT ON;

    DELETE FROM pq.EncryptedObject
    WHERE ExpiresAtUnixSeconds IS NOT NULL
      AND ExpiresAtUnixSeconds <= DATEDIFF_BIG(SECOND, '1970-01-01', SYSUTCDATETIME());
END
GO
