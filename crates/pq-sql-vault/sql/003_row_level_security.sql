-- pq-sql-vault: row-level security (optional, multi-tenant deployments)
--
-- Isolates which ObjectKey rows a given database principal can see or
-- write, by prefix. This is genuinely enforced by SQL Server itself at
-- the row level -- it applies even to ad-hoc queries, not just access
-- through the stored procedures in 002_procedures.sql.
--
-- Only meaningful if your deployment actually has multiple tenants
-- sharing one database and sets SESSION_CONTEXT(N'tenant_prefix') per
-- connection (e.g. right after login, via sp_set_session_context).
-- Skip this file entirely for a single-tenant deployment.

CREATE FUNCTION pq.fn_rls_object_key_predicate(@ObjectKey NVARCHAR(400))
    RETURNS TABLE
    WITH SCHEMABINDING
AS
    RETURN SELECT 1 AS fn_result
    WHERE IS_MEMBER(N'pq_vault_admin') = 1
       OR @ObjectKey LIKE CONVERT(NVARCHAR(400), SESSION_CONTEXT(N'tenant_prefix')) + N'%';
GO

CREATE SECURITY POLICY pq.EncryptedObjectRLS
    ADD FILTER PREDICATE pq.fn_rls_object_key_predicate(ObjectKey) ON pq.EncryptedObject,
    ADD BLOCK PREDICATE pq.fn_rls_object_key_predicate(ObjectKey) ON pq.EncryptedObject AFTER INSERT
    WITH (STATE = ON);
GO

-- Membership in pq_vault_admin bypasses the tenant-prefix filter
-- entirely (for operational/maintenance access, e.g. pq.sp_purge_expired_objects
-- run by a service account). Grant it sparingly.
CREATE ROLE pq_vault_admin;
GO
