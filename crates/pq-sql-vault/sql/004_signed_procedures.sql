-- pq-sql-vault: certificate-signed procedures (optional, least-privilege
-- application logins)
--
-- Signs the two rotation procedures with a certificate and grants the
-- permissions they need (UPDATE on pq.KeyVersion, INSERT on
-- pq.AuditLog) to a certificate-mapped user rather than to the calling
-- application login directly. An application login granted only
-- EXECUTE on these procedures can then rotate/retire keys through them,
-- without ever holding direct UPDATE rights on pq.KeyVersion itself --
-- so a SQL-injection or over-broad-grant mistake elsewhere in the
-- application can't be used to tamper with key state directly.
--
-- This is standard SQL Server "module signing" and is independent of
-- TLS/application-layer auth; it controls what an *authenticated*
-- database principal can do once connected.

CREATE CERTIFICATE pq_vault_proc_signing_cert
    WITH SUBJECT = N'pq-sql-vault stored procedure signing';
GO

ADD SIGNATURE TO pq.sp_set_active_key_version BY CERTIFICATE pq_vault_proc_signing_cert;
ADD SIGNATURE TO pq.sp_retire_key_version BY CERTIFICATE pq_vault_proc_signing_cert;
ADD SIGNATURE TO pq.sp_register_key_version BY CERTIFICATE pq_vault_proc_signing_cert;
GO

CREATE USER pq_vault_proc_signing_user FOR CERTIFICATE pq_vault_proc_signing_cert;
GO

GRANT UPDATE, INSERT ON pq.KeyVersion TO pq_vault_proc_signing_user;
GRANT INSERT ON pq.AuditLog TO pq_vault_proc_signing_user;
GO

-- The application login itself needs only EXECUTE on the procedures
-- (not table-level grants) -- adjust the principal name to match your
-- deployment's actual application login/user.
-- GRANT EXECUTE ON pq.sp_register_key_version TO [your_app_login];
-- GRANT EXECUTE ON pq.sp_set_active_key_version TO [your_app_login];
-- GRANT EXECUTE ON pq.sp_retire_key_version TO [your_app_login];
-- GRANT EXECUTE ON pq.sp_upsert_encrypted_object TO [your_app_login];
-- GRANT EXECUTE ON pq.sp_get_encrypted_object TO [your_app_login];
