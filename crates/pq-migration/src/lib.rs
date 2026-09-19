//! Applies a directory of `NNN_name.sql` migration files, in
//! numeric-prefix order, against a live [`tiberius`] connection, and
//! tracks which ones have already run so a repeat [`run_migrations`]
//! call is a safe no-op for anything already applied.
//!
//! This crate is generic over `pq-sql-vault`'s `sql/` directory (it
//! does not depend on `pq-sql-vault` itself -- `pq-sql-vault` depends
//! on *this* crate, so the reverse would be a dependency cycle) but is
//! written to, and tested against, that directory's actual four
//! migration files.
//!
//! # Division of responsibility (kept deliberately narrow)
//!
//! This crate only ever runs SQL text it is handed; it does not know or
//! care what `pq-sql-vault`'s schema looks like. The one piece of SQL
//! this crate *does* own is the tracking table's own bootstrap DDL (see
//! [`ensure_tracking_table`] below) -- kept out of the numbered
//! migration files on purpose, since the tracking table has to exist
//! *before* this crate can determine which numbered migrations (if any)
//! have already run, and putting "migration 000" inside the tracked
//! sequence would make that bootstrap step itself un-trackable by the
//! very mechanism it's bootstrapping. The tracking table therefore
//! lives in `dbo` (the default schema, always present), independent of
//! whatever schema `sql/001_schema.sql` and later files create (`pq`,
//! for `pq-sql-vault`) -- so this crate never has to guess at, or
//! depend on, an application's own schema name.
//!
//! # `GO` batch separation
//!
//! `sqlcmd`/SSMS's `GO` is a *client-side* batch separator; SQL
//! Server's wire protocol (and so `tiberius`) has no concept of it, and
//! sending a `GO`-containing string as one query would just be a syntax
//! error. [`split_batches`] splits a migration file's text on lines
//! that are exactly `GO` (case-insensitively, ignoring surrounding
//! whitespace) before executing each resulting batch as its own
//! request -- the same convention `sql/001_schema.sql` through
//! `sql/004_signed_procedures.sql` already write to.
//!
//! Owns: [`run_migrations`].
#![forbid(unsafe_code)]

use futures_io::{AsyncRead, AsyncWrite};
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::Path;
use tiberius::Client;

/// One parsed migration file: its numeric sequence (the `NNN` prefix),
/// its filename (recorded in the tracking table so a human auditing it
/// can see exactly what ran), and its raw SQL text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Migration {
    pub sequence: u32,
    pub filename: String,
    pub sql: String,
}

#[derive(Debug)]
pub enum MigrationError {
    Io(std::io::Error),
    Sql(tiberius::error::Error),
    /// A file directly inside the migrations directory didn't match the
    /// `NNN_name.sql` convention (no `.sql` extension, no leading
    /// digits, or no `_` separator right after them).
    InvalidFilename(String),
    /// Two files in the same directory claim the same sequence number.
    DuplicateSequence(u32),
    /// A row came back from the tracking table in a shape this crate
    /// didn't expect (should be unreachable against the schema
    /// `ensure_tracking_table` creates, but this crate would rather
    /// report that clearly than panic on a `None` from `Row::get`).
    UnexpectedRowShape(&'static str),
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationError::Io(e) => write!(f, "I/O error: {e}"),
            MigrationError::Sql(e) => write!(f, "SQL error: {e}"),
            MigrationError::InvalidFilename(name) => {
                write!(f, "migration filename doesn't match NNN_name.sql: {name:?}")
            }
            MigrationError::DuplicateSequence(seq) => {
                write!(f, "more than one migration file claims sequence {seq}")
            }
            MigrationError::UnexpectedRowShape(what) => {
                write!(f, "unexpected row shape: {what}")
            }
        }
    }
}
impl std::error::Error for MigrationError {}

impl From<std::io::Error> for MigrationError {
    fn from(e: std::io::Error) -> Self {
        MigrationError::Io(e)
    }
}
impl From<tiberius::error::Error> for MigrationError {
    fn from(e: tiberius::error::Error) -> Self {
        MigrationError::Sql(e)
    }
}

/// Parse the leading numeric prefix of a `NNN_name.sql` filename, e.g.
/// `"001_schema.sql"` -> `1`. Requires a `.sql` extension, at least one
/// leading digit, and an `_` immediately after the digits -- this is
/// deliberately stricter than "starts with a digit," so a stray file
/// like `"README.sql"` or `"1schema.sql"` fails loudly instead of
/// sorting unpredictably among the real migrations.
pub fn parse_sequence(filename: &str) -> Result<u32, MigrationError> {
    let invalid = || MigrationError::InvalidFilename(filename.to_string());

    let stem = filename.strip_suffix(".sql").ok_or_else(invalid)?;
    let digits: String = stem.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return Err(invalid());
    }
    if !stem[digits.len()..].starts_with('_') {
        return Err(invalid());
    }
    digits.parse::<u32>().map_err(|_| invalid())
}

/// Split a migration file's SQL text into separate batches on lines
/// that are exactly `GO` (case-insensitive, surrounding whitespace
/// ignored) -- see the module docs for why. Empty batches (a file with
/// no statements, or consecutive `GO` lines) are dropped.
pub fn split_batches(sql: &str) -> Vec<String> {
    let mut batches = vec![String::new()];
    for line in sql.lines() {
        if line.trim().eq_ignore_ascii_case("GO") {
            batches.push(String::new());
        } else {
            let current = batches.last_mut().expect("batches always has >= 1 entry");
            current.push_str(line);
            current.push('\n');
        }
    }
    batches
        .into_iter()
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .collect()
}

/// Load every `*.sql` file directly inside `dir`, parse each one's
/// sequence number, and return them sorted in ascending numeric order
/// (**not** lexical order -- `"2_x.sql"` sorts before `"10_y.sql"`).
/// Non-`.sql` files are silently skipped; an unparseable `.sql`
/// filename or a sequence-number collision is an error.
pub fn load_migrations(dir: &Path) -> Result<Vec<Migration>, MigrationError> {
    let mut migrations = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("sql") {
            continue;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| MigrationError::InvalidFilename(path.display().to_string()))?
            .to_string();
        let sequence = parse_sequence(&filename)?;
        let sql = fs::read_to_string(&path)?;
        migrations.push(Migration {
            sequence,
            filename,
            sql,
        });
    }

    migrations.sort_by_key(|m| m.sequence);
    for pair in migrations.windows(2) {
        if pair[0].sequence == pair[1].sequence {
            return Err(MigrationError::DuplicateSequence(pair[0].sequence));
        }
    }

    Ok(migrations)
}

/// Pure planning step: given every known migration (in ascending
/// order) and the set of sequence numbers already recorded as applied,
/// return the ones still to run, in the same order. Exposed separately
/// from [`run_migrations`] so the part that actually decides what
/// happens is testable without a live connection.
pub fn pending<'a>(all: &'a [Migration], applied: &BTreeSet<u32>) -> Vec<&'a Migration> {
    all.iter()
        .filter(|m| !applied.contains(&m.sequence))
        .collect()
}

/// Idempotent bootstrap DDL for the tracking table itself -- see the
/// module docs for why this lives in Rust rather than as a numbered
/// migration file. `dbo` (SQL Server's always-present default schema)
/// rather than `pq`, since `pq` doesn't exist yet the first time this
/// runs (it's created by `sql/001_schema.sql`).
const ENSURE_TRACKING_TABLE_SQL: &str = r#"
IF NOT EXISTS (SELECT 1 FROM sys.tables WHERE name = 'SchemaMigrations' AND schema_id = SCHEMA_ID('dbo'))
BEGIN
    CREATE TABLE dbo.SchemaMigrations (
        Sequence INT NOT NULL PRIMARY KEY,
        Filename NVARCHAR(400) NOT NULL,
        AppliedAtUtc DATETIME2 NOT NULL CONSTRAINT DF_SchemaMigrations_AppliedAtUtc DEFAULT SYSUTCDATETIME()
    );
END
"#;

async fn ensure_tracking_table<S>(client: &mut Client<S>) -> Result<(), MigrationError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    client
        .simple_query(ENSURE_TRACKING_TABLE_SQL)
        .await?
        .into_results()
        .await?;
    Ok(())
}

async fn fetch_applied<S>(client: &mut Client<S>) -> Result<BTreeSet<u32>, MigrationError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    let rows = client
        .simple_query("SELECT Sequence FROM dbo.SchemaMigrations")
        .await?
        .into_first_result()
        .await?;

    let mut applied = BTreeSet::new();
    for row in rows {
        let sequence: i32 = row
            .get(0)
            .ok_or(MigrationError::UnexpectedRowShape("missing Sequence"))?;
        applied.insert(sequence as u32);
    }
    Ok(applied)
}

async fn apply_one<S>(client: &mut Client<S>, migration: &Migration) -> Result<(), MigrationError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    for batch in split_batches(&migration.sql) {
        client.simple_query(batch).await?.into_results().await?;
    }
    client
        .execute(
            "INSERT INTO dbo.SchemaMigrations (Sequence, Filename) VALUES (@P1, @P2)",
            &[&(migration.sequence as i32), &migration.filename.as_str()],
        )
        .await?;
    Ok(())
}

/// Apply every migration in `migrations_dir` that isn't already
/// recorded as applied, in ascending sequence order, against `client`.
/// Ensures the tracking table exists first. Safe to call repeatedly
/// (e.g. once per process startup): a second call with nothing new to
/// apply does no writes beyond the tracking-table bootstrap check.
///
/// Generic over any stream `tiberius::Client<S>` can wrap (exactly
/// `tiberius::Client`'s own bound) rather than one fixed connection
/// type, so callers -- `pq-sql-vault` in particular -- can hand it
/// whichever already-connected client they have.
pub async fn run_migrations<S>(
    client: &mut Client<S>,
    migrations_dir: &Path,
) -> Result<(), MigrationError>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    ensure_tracking_table(client).await?;
    let migrations = load_migrations(migrations_dir)?;
    let applied = fetch_applied(client).await?;
    for migration in pending(&migrations, &applied) {
        apply_one(client, migration).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ---- parse_sequence -------------------------------------------------

    #[test]
    fn parses_the_standard_three_digit_prefix() {
        assert_eq!(parse_sequence("001_schema.sql").unwrap(), 1);
        assert_eq!(parse_sequence("004_signed_procedures.sql").unwrap(), 4);
    }

    #[test]
    fn parses_prefixes_of_any_digit_width_numerically_not_lexically() {
        assert_eq!(parse_sequence("2_x.sql").unwrap(), 2);
        assert_eq!(parse_sequence("10_y.sql").unwrap(), 10);
        assert_eq!(parse_sequence("000_bootstrap.sql").unwrap(), 0);
    }

    #[test]
    fn rejects_a_filename_with_no_sql_extension() {
        assert!(matches!(
            parse_sequence("001_schema.txt"),
            Err(MigrationError::InvalidFilename(_))
        ));
    }

    #[test]
    fn rejects_a_filename_with_no_leading_digits() {
        assert!(matches!(
            parse_sequence("schema.sql"),
            Err(MigrationError::InvalidFilename(_))
        ));
    }

    #[test]
    fn rejects_a_filename_missing_the_underscore_separator() {
        assert!(matches!(
            parse_sequence("001schema.sql"),
            Err(MigrationError::InvalidFilename(_))
        ));
    }

    // ---- split_batches ----------------------------------------------------

    #[test]
    fn splits_on_bare_go_lines() {
        let sql = "CREATE TABLE a (x INT);\nGO\nCREATE TABLE b (y INT);\nGO\n";
        let batches = split_batches(sql);
        assert_eq!(batches.len(), 2);
        assert!(batches[0].contains("CREATE TABLE a"));
        assert!(batches[1].contains("CREATE TABLE b"));
        assert!(!batches[0].to_ascii_uppercase().contains("GO\n"));
    }

    #[test]
    fn go_is_case_insensitive_and_whitespace_tolerant() {
        let sql = "SELECT 1;\n  go  \nSELECT 2;\nGo\n";
        assert_eq!(split_batches(sql).len(), 2);
    }

    #[test]
    fn a_line_containing_go_as_a_substring_is_not_treated_as_a_separator() {
        // "GOTO" or a column literally named "GO" must not split.
        let sql = "SELECT 'GOTO' AS col;\nGO\n";
        let batches = split_batches(sql);
        assert_eq!(batches.len(), 1);
        assert!(batches[0].contains("GOTO"));
    }

    #[test]
    fn consecutive_and_trailing_go_lines_produce_no_empty_batches() {
        let sql = "SELECT 1;\nGO\nGO\nSELECT 2;\nGO\n\n\n";
        assert_eq!(split_batches(sql).len(), 2);
    }

    #[test]
    fn a_file_with_no_go_separators_is_a_single_batch() {
        let sql = "SELECT 1;\nSELECT 2;\n";
        assert_eq!(split_batches(sql), vec!["SELECT 1;\nSELECT 2;".to_string()]);
    }

    #[test]
    fn splitting_pq_sql_vaults_actual_migration_files_produces_at_least_one_batch_each() {
        // Real files on disk, not fixtures -- proves this crate's
        // batching logic actually handles the files it will be pointed
        // at in production, without pq-migration depending on
        // pq-sql-vault as a crate (this just reads its sql/ directory
        // as data at test time).
        let dir = sibling_sql_dir();
        let mut checked_any = false;
        for entry in fs::read_dir(&dir).expect("read pq-sql-vault/sql") {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("sql") {
                continue;
            }
            let sql = fs::read_to_string(&path).unwrap();
            let batches = split_batches(&sql);
            assert!(!batches.is_empty(), "{path:?} produced no batches at all");
            for batch in &batches {
                assert!(
                    !batch.trim().eq_ignore_ascii_case("GO"),
                    "{path:?} left a bare GO inside a batch"
                );
            }
            checked_any = true;
        }
        assert!(checked_any, "expected at least one .sql file in {dir:?}");
    }

    fn sibling_sql_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../pq-sql-vault/sql")
    }

    // ---- load_migrations ---------------------------------------------------

    fn write(dir: &Path, name: &str, contents: &str) {
        fs::write(dir.join(name), contents).unwrap();
    }

    #[test]
    fn load_migrations_sorts_numerically_regardless_of_directory_order() {
        let dir = tempfile::tempdir().unwrap();
        // Written in an order that would sort wrong lexically (10 before 2).
        write(dir.path(), "10_second.sql", "SELECT 2;");
        write(dir.path(), "2_first.sql", "SELECT 1;");
        write(dir.path(), "not-a-migration.txt", "ignore me");

        let migrations = load_migrations(dir.path()).unwrap();
        let sequences: Vec<u32> = migrations.iter().map(|m| m.sequence).collect();
        assert_eq!(sequences, vec![2, 10]);
        assert_eq!(migrations[0].filename, "2_first.sql");
        assert_eq!(migrations[1].filename, "10_second.sql");
    }

    #[test]
    fn load_migrations_reads_each_files_actual_sql_text() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "001_a.sql", "CREATE TABLE t (x INT);\n");
        let migrations = load_migrations(dir.path()).unwrap();
        assert_eq!(migrations[0].sql, "CREATE TABLE t (x INT);\n");
    }

    #[test]
    fn load_migrations_rejects_a_sequence_collision() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "001_a.sql", "SELECT 1;");
        write(dir.path(), "001_b.sql", "SELECT 2;");
        assert!(matches!(
            load_migrations(dir.path()),
            Err(MigrationError::DuplicateSequence(1))
        ));
    }

    #[test]
    fn load_migrations_rejects_an_unparseable_filename() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "whatever.sql", "SELECT 1;");
        assert!(matches!(
            load_migrations(dir.path()),
            Err(MigrationError::InvalidFilename(_))
        ));
    }

    #[test]
    fn load_migrations_on_an_empty_directory_is_an_empty_vec_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_migrations(dir.path()).unwrap(), Vec::new());
    }

    #[test]
    fn load_migrations_against_pq_sql_vaults_real_directory_finds_all_four_files_in_order() {
        let migrations = load_migrations(&sibling_sql_dir()).unwrap();
        let sequences: Vec<u32> = migrations.iter().map(|m| m.sequence).collect();
        assert_eq!(sequences, vec![1, 2, 3, 4]);
        assert_eq!(migrations[0].filename, "001_schema.sql");
        assert_eq!(migrations[3].filename, "004_signed_procedures.sql");
    }

    // ---- pending() -----------------------------------------------------

    fn migration(sequence: u32) -> Migration {
        Migration {
            sequence,
            filename: format!("{sequence:03}_x.sql"),
            sql: String::new(),
        }
    }

    #[test]
    fn pending_returns_everything_when_nothing_is_applied() {
        let all = vec![migration(1), migration(2), migration(3)];
        let applied = BTreeSet::new();
        let result = pending(&all, &applied);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn pending_skips_already_applied_sequences() {
        let all = vec![migration(1), migration(2), migration(3)];
        let mut applied = BTreeSet::new();
        applied.insert(1);
        applied.insert(3);
        let result: Vec<u32> = pending(&all, &applied).iter().map(|m| m.sequence).collect();
        assert_eq!(result, vec![2]);
    }

    #[test]
    fn pending_is_empty_once_everything_is_applied() {
        let all = vec![migration(1), migration(2)];
        let mut applied = BTreeSet::new();
        applied.insert(1);
        applied.insert(2);
        assert!(pending(&all, &applied).is_empty());
    }

    #[test]
    fn pending_preserves_ascending_order() {
        let all = vec![migration(1), migration(2), migration(3), migration(4)];
        let mut applied = BTreeSet::new();
        applied.insert(2);
        let result: Vec<u32> = pending(&all, &applied).iter().map(|m| m.sequence).collect();
        assert_eq!(result, vec![1, 3, 4]);
    }

    // ---- error display ---------------------------------------------------

    #[test]
    fn error_messages_are_informative() {
        assert!(MigrationError::InvalidFilename("bad.sql".into())
            .to_string()
            .contains("bad.sql"));
        assert!(MigrationError::DuplicateSequence(7)
            .to_string()
            .contains('7'));
        assert!(MigrationError::UnexpectedRowShape("missing Sequence")
            .to_string()
            .contains("missing Sequence"));
    }
}
