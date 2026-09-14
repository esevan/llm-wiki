use chrono::{SecondsFormat, Utc};
use rusqlite::{Connection, Error as SqliteError, ErrorCode, Transaction, TransactionBehavior};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

const IMMEDIATE_TRANSACTION_ATTEMPTS: usize = 4;
const IMMEDIATE_TRANSACTION_RETRY_DELAYS: [Duration; IMMEDIATE_TRANSACTION_ATTEMPTS - 1] = [
    Duration::from_millis(25),
    Duration::from_millis(50),
    Duration::from_millis(100),
];

fn is_transient_lock(error: &SqliteError) -> bool {
    matches!(
        error,
        SqliteError::SqliteFailure(error, _)
            if matches!(error.code, ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked)
    )
}

/// Acquire the writer slot before an operation reads mutable state.
///
/// Retrying is deliberately limited to `BEGIN IMMEDIATE`: no caller-provided operation has run
/// yet, so a retry cannot replay an external effect or a partially committed database mutation.
pub(crate) fn immediate_transaction(connection: &mut Connection) -> Result<Transaction<'_>, String> {
    for attempt in 0..IMMEDIATE_TRANSACTION_ATTEMPTS {
        // `new_unchecked` accepts the shared reference needed for retrying this acquisition;
        // this helper still requires a mutable connection, and all callers use a fresh one.
        match Transaction::new_unchecked(connection, TransactionBehavior::Immediate) {
            Ok(transaction) => return Ok(transaction),
            Err(error)
                if is_transient_lock(&error) && attempt + 1 < IMMEDIATE_TRANSACTION_ATTEMPTS =>
            {
                std::thread::sleep(IMMEDIATE_TRANSACTION_RETRY_DELAYS[attempt]);
            }
            Err(error) => return Err(error.to_string()),
        }
    }
    unreachable!("the final transaction attempt always returns")
}

#[derive(Debug, Clone)]
pub struct MigrationBackup {
    pub database: PathBuf,
    pub manifest: PathBuf,
    #[allow(dead_code)]
    pub hash: String,
}

pub fn open(path: &Path) -> Result<Connection, String> {
    let connection = Connection::open(path).map_err(|error| error.to_string())?;
    connection
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| error.to_string())?;
    connection
        .execute_batch("PRAGMA foreign_keys=ON;")
        .map_err(|error| error.to_string())?;
    Ok(connection)
}

pub fn initialize(path: &Path) -> Result<(), String> {
    let mut connection = open(path)?;
    connection
        .execute_batch("PRAGMA journal_mode=WAL;")
        .map_err(|error| error.to_string())?;
    let version = super::migrations::schema_version(&connection)?;
    let has_user_data = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .unwrap_or(false);
    let backup = if version < super::migrations::CURRENT_SCHEMA_VERSION && has_user_data {
        connection
            .execute_batch("PRAGMA wal_checkpoint(FULL);")
            .map_err(|error| format!("migration_backup_checkpoint_failed: {error}"))?;
        drop(connection);
        let backup = match create_verified_backup(path, version) {
            Ok(backup) => backup,
            Err(error) => {
                let _ = write_failure_marker(path, "migration_backup", None, &error);
                return Err(format!("migration_failed:migration_backup: {error}"));
            }
        };
        connection = open(path)?;
        connection
            .execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|error| error.to_string())?;
        Some(backup)
    } else {
        None
    };
    match super::migrations::apply(&mut connection) {
        Ok(()) => {
            clear_failure_marker(path)?;
            Ok(())
        }
        Err(error) => {
            let stage = "schema_migration";
            let _ = write_failure_marker(path, stage, backup.as_ref(), &error);
            Err(format!("migration_failed:{stage}: {error}"))
        }
    }
}

fn timestamp() -> String {
    Utc::now()
        .to_rfc3339_opts(SecondsFormat::Nanos, true)
        .replace([':', '.'], "-")
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("llm-wiki.db");
    path.with_file_name(format!("{name}.{suffix}"))
}

fn file_hash(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sync_file_and_parent(path: &Path) -> Result<(), String> {
    OpenOptions::new()
        .read(true)
        .open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| error.to_string())?;
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn integrity_check(path: &Path) -> Result<(), String> {
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| error.to_string())?;
    let result: String = connection
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if result != "ok" {
        return Err(format!("integrity_check_failed:{result}"));
    }
    Ok(())
}

pub fn create_verified_backup(path: &Path, source_version: i64) -> Result<MigrationBackup, String> {
    let suffix = format!("migration-v{source_version}-{}", timestamp());
    let backup = sidecar(path, &format!("{suffix}.sqlite3"));
    let manifest = sidecar(path, &format!("{suffix}.manifest.json"));
    create_verified_backup_at(path, source_version, &backup, &manifest, |_| Ok(()))
}

fn create_verified_backup_at<F>(
    path: &Path,
    source_version: i64,
    backup: &Path,
    manifest: &Path,
    after_copy: F,
) -> Result<MigrationBackup, String>
where
    F: FnOnce(&Path) -> Result<(), String>,
{
    let temporary = manifest.with_extension("json.tmp");
    let result = (|| {
        let source = open(path).map_err(|error| format!("migration_backup_open_failed:{error}"))?;
        source
            .execute("VACUUM INTO ?1", [backup.to_string_lossy().as_ref()])
            .map_err(|error| format!("migration_backup_copy_failed:{error}"))?;
        after_copy(backup)?;
        sync_file_and_parent(backup)
            .map_err(|error| format!("migration_backup_sync_failed:{error}"))?;
        integrity_check(backup)
            .map_err(|error| format!("migration_backup_integrity_failed:{error}"))?;
        let hash = file_hash(backup)?;
        let size = fs::metadata(backup)
            .map_err(|error| error.to_string())?
            .len();
        let manifest_value = json!({
            "formatVersion": 1,
            "sourceSchemaVersion": source_version,
            "targetSchemaVersion": super::migrations::CURRENT_SCHEMA_VERSION,
            "appVersion": env!("CARGO_PKG_VERSION"),
            "createdAt": Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
            "backupFile": backup.file_name().and_then(|value| value.to_str()).unwrap_or(""),
            "size": size,
            "sha256": hash,
        });
        {
            let mut file = File::create(&temporary).map_err(|error| error.to_string())?;
            file.write_all(manifest_value.to_string().as_bytes())
                .map_err(|error| error.to_string())?;
            file.sync_all().map_err(|error| error.to_string())?;
        }
        fs::rename(&temporary, manifest).map_err(|error| error.to_string())?;
        sync_file_and_parent(manifest)?;
        verify_backup_manifest(manifest)?;
        Ok(MigrationBackup {
            database: backup.to_owned(),
            manifest: manifest.to_owned(),
            hash,
        })
    })();
    if result.is_err() {
        // A backup is usable only with its verified manifest. Remove artifacts from
        // this attempt so startup cannot mistake a partial copy for a recovery point.
        let _ = fs::remove_file(&temporary);
        let _ = fs::remove_file(backup);
    }
    result
}

pub fn verify_backup_manifest(manifest_path: &Path) -> Result<MigrationBackup, String> {
    let value: Value =
        serde_json::from_slice(&fs::read(manifest_path).map_err(|error| error.to_string())?)
            .map_err(|error| format!("migration_manifest_invalid:{error}"))?;
    let file_name = value
        .get("backupFile")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or("migration_manifest_invalid:backupFile")?;
    if Path::new(file_name).components().count() != 1 {
        return Err("migration_manifest_invalid:backupFile".into());
    }
    let database = manifest_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(file_name);
    let expected_size = value
        .get("size")
        .and_then(Value::as_u64)
        .ok_or("migration_manifest_invalid:size")?;
    if fs::metadata(&database)
        .map_err(|error| error.to_string())?
        .len()
        != expected_size
    {
        return Err("migration_backup_size_mismatch".into());
    }
    let expected_hash = value
        .get("sha256")
        .and_then(Value::as_str)
        .ok_or("migration_manifest_invalid:sha256")?;
    let hash = file_hash(&database)?;
    if hash != expected_hash {
        return Err("migration_backup_hash_mismatch".into());
    }
    integrity_check(&database)?;
    Ok(MigrationBackup {
        database,
        manifest: manifest_path.to_owned(),
        hash,
    })
}

pub fn restore_verified_backup(path: &Path, manifest_path: &Path) -> Result<PathBuf, String> {
    let backup = verify_backup_manifest(manifest_path)?;
    let temporary = sidecar(path, "restore.tmp");
    fs::copy(backup.database, &temporary).map_err(|error| error.to_string())?;
    sync_file_and_parent(&temporary)?;
    integrity_check(&temporary)?;
    let recovery = sidecar(path, &format!("failed-{}.sqlite3", timestamp()));
    let mut moved_sidecars = Vec::new();
    if path.exists() {
        if let Ok(connection) = open(path) {
            let _ = connection.execute_batch("PRAGMA wal_checkpoint(FULL);");
        }
        fs::rename(path, &recovery).map_err(|error| error.to_string())?;
        for suffix in ["-wal", "-shm"] {
            let live_sidecar = PathBuf::from(format!("{}{suffix}", path.to_string_lossy()));
            if live_sidecar.exists() {
                let recovery_sidecar =
                    PathBuf::from(format!("{}{suffix}", recovery.to_string_lossy()));
                if let Err(error) = fs::rename(&live_sidecar, &recovery_sidecar) {
                    for (original, moved) in moved_sidecars.iter().rev() {
                        let _ = fs::rename(moved, original);
                    }
                    let _ = fs::rename(&recovery, path);
                    let _ = fs::remove_file(&temporary);
                    return Err(error.to_string());
                }
                moved_sidecars.push((live_sidecar, recovery_sidecar));
            }
        }
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if recovery.exists() {
            for (original, moved) in moved_sidecars.iter().rev() {
                let _ = fs::rename(moved, original);
            }
            let _ = fs::rename(&recovery, path);
        }
        return Err(error.to_string());
    }
    if let Err(error) = sync_file_and_parent(path).and_then(|_| integrity_check(path)) {
        let failed_restore = sidecar(path, &format!("restore-failed-{}.sqlite3", timestamp()));
        let _ = fs::rename(path, failed_restore);
        for (original, moved) in moved_sidecars.iter().rev() {
            let _ = fs::rename(moved, original);
        }
        if recovery.exists() {
            let _ = fs::rename(&recovery, path);
        }
        return Err(error);
    }
    clear_failure_marker(path)?;
    Ok(recovery)
}

fn failure_marker(path: &Path) -> PathBuf {
    sidecar(path, "migration-failure.json")
}

fn write_failure_marker(
    path: &Path,
    stage: &str,
    backup: Option<&MigrationBackup>,
    _internal_error: &str,
) -> Result<(), String> {
    let marker = failure_marker(path);
    let value = json!({
        "stage": stage,
        "safeError": "migration_failed",
        "manifestFile": backup.and_then(|item| item.manifest.file_name()).and_then(|value| value.to_str()),
        "retryAvailable": true,
        "restoreAvailable": backup.is_some(),
        "createdAt": Utc::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
    });
    fs::write(&marker, value.to_string()).map_err(|error| error.to_string())?;
    sync_file_and_parent(&marker)
}

fn clear_failure_marker(path: &Path) -> Result<(), String> {
    let marker = failure_marker(path);
    if marker.exists() {
        fs::remove_file(marker).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn migration_failure(path: &Path) -> Result<Option<Value>, String> {
    fs::read(failure_marker(path))
        .ok()
        .map(|bytes| serde_json::from_slice(&bytes).map_err(|error| error.to_string()))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wal_database(root: &Path) -> PathBuf {
        let path = root.join("state.db");
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE records(id TEXT PRIMARY KEY,value TEXT NOT NULL); INSERT INTO records VALUES('one','before'); PRAGMA user_version=7;").unwrap();
        path
    }

    #[test]
    fn verified_backup_captures_committed_wal_and_detects_tampering() {
        let root = tempfile::tempdir().unwrap();
        let path = wal_database(root.path());
        let backup = create_verified_backup(&path, 7).unwrap();
        assert_eq!(
            verify_backup_manifest(&backup.manifest).unwrap().hash,
            backup.hash
        );
        let manifest: Value = serde_json::from_slice(&fs::read(&backup.manifest).unwrap()).unwrap();
        assert_eq!(manifest["sourceSchemaVersion"], 7);
        assert_eq!(
            manifest["targetSchemaVersion"],
            super::super::migrations::CURRENT_SCHEMA_VERSION
        );
        assert_eq!(manifest["formatVersion"], 1);
        assert!(manifest["size"].as_u64().unwrap() > 0);
        assert_eq!(
            Connection::open(&backup.database)
                .unwrap()
                .query_row("SELECT value FROM records WHERE id='one'", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
            "before"
        );
        OpenOptions::new()
            .append(true)
            .open(&backup.database)
            .unwrap()
            .write_all(b"tamper")
            .unwrap();
        assert!(verify_backup_manifest(&backup.manifest)
            .unwrap_err()
            .contains("mismatch"));
    }

    #[test]
    fn restore_keeps_failed_database_as_recovery_and_reopens_verified_source() {
        let root = tempfile::tempdir().unwrap();
        let path = wal_database(root.path());
        let backup = create_verified_backup(&path, 7).unwrap();
        Connection::open(&path)
            .unwrap()
            .execute("UPDATE records SET value='after' WHERE id='one'", [])
            .unwrap();
        let recovery = restore_verified_backup(&path, &backup.manifest).unwrap();
        assert!(recovery.exists());
        assert_eq!(
            Connection::open(&path)
                .unwrap()
                .query_row("SELECT value FROM records WHERE id='one'", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
            "before"
        );
        assert_eq!(
            Connection::open(&recovery)
                .unwrap()
                .query_row("SELECT value FROM records WHERE id='one'", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
            "after"
        );
    }

    #[test]
    fn failed_startup_migration_retains_backup_and_safe_retry_restore_state() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state.db");
        let mut connection = Connection::open(&path).unwrap();
        super::super::migrations::apply(&mut connection).unwrap();
        connection.pragma_update(None, "user_version", 7).unwrap();
        connection.execute("INSERT INTO captures(id,text,created_at,source_mode,last_user_activity_at) VALUES('c','x','now','capture','now')",[]).unwrap();
        connection.execute("INSERT INTO problems(id,capture_id,statement,state,created_at,current_revision) VALUES('p','c','p','open','now',1)",[]).unwrap();
        connection.execute("INSERT INTO features(id,problem_id,title,outcome,state,created_at) VALUES('bad','p','bad','bad','invalid','now')",[]).unwrap();
        drop(connection);
        assert!(initialize(&path).unwrap_err().contains("schema_migration"));
        let state = migration_failure(&path).unwrap().unwrap();
        assert_eq!(state["safeError"], "migration_failed");
        assert_eq!(state["restoreAvailable"], true);
        let manifest = root.path().join(state["manifestFile"].as_str().unwrap());
        restore_verified_backup(&path, &manifest).unwrap();
        assert_eq!(
            Connection::open(&path)
                .unwrap()
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            7
        );
    }

    #[test]
    fn interrupted_backup_write_leaves_source_intact_and_no_partial_recovery_point() {
        let root = tempfile::tempdir().unwrap();
        let path = wal_database(root.path());
        let backup = root.path().join("interrupted.sqlite3");
        let manifest = root.path().join("manifest-target");
        fs::create_dir(&manifest).unwrap();

        assert!(create_verified_backup_at(&path, 7, &backup, &manifest, |_| Ok(())).is_err());
        assert!(!backup.exists());
        assert!(!manifest.with_extension("json.tmp").exists());
        assert_eq!(
            Connection::open(&path)
                .unwrap()
                .query_row("SELECT value FROM records WHERE id='one'", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
            "before"
        );
    }

    #[test]
    fn unavailable_backup_destination_never_mutates_the_source_database() {
        let root = tempfile::tempdir().unwrap();
        let path = wal_database(root.path());
        let unavailable = root.path().join("missing-parent");
        let backup = unavailable.join("backup.sqlite3");
        let manifest = unavailable.join("backup.manifest.json");

        let error =
            create_verified_backup_at(&path, 7, &backup, &manifest, |_| Ok(())).unwrap_err();
        assert!(error.contains("migration_backup_copy_failed"));
        assert!(!backup.exists());
        assert!(!manifest.exists());
        assert_eq!(
            Connection::open(&path)
                .unwrap()
                .query_row("SELECT value FROM records WHERE id='one'", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
            "before"
        );
    }

    #[test]
    fn immediate_transaction_retries_a_contended_writer_slot() {
        let root = tempfile::tempdir().unwrap();
        let path = wal_database(root.path());
        let (locked, ready) = std::sync::mpsc::channel();
        let lock_path = path.clone();
        let holder = std::thread::spawn(move || {
            let connection = Connection::open(lock_path).unwrap();
            connection.execute_batch("BEGIN IMMEDIATE").unwrap();
            locked.send(()).unwrap();
            std::thread::sleep(Duration::from_millis(75));
            connection.execute_batch("COMMIT").unwrap();
        });
        ready.recv().unwrap();

        let mut contender = open(&path).unwrap();
        // Make each failed BEGIN return immediately so this test exercises the bounded retry
        // path instead of only SQLite's longer per-attempt busy handler.
        contender.busy_timeout(Duration::ZERO).unwrap();
        let transaction = immediate_transaction(&mut contender).unwrap();
        transaction
            .execute("UPDATE records SET value='after' WHERE id='one'", [])
            .unwrap();
        transaction.commit().unwrap();
        holder.join().unwrap();

        assert_eq!(
            Connection::open(path)
                .unwrap()
                .query_row("SELECT value FROM records WHERE id='one'", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "after"
        );
    }

    #[test]
    fn immediate_writer_avoids_a_deferred_snapshot_upgrade_failure() {
        let root = tempfile::tempdir().unwrap();
        let path = wal_database(root.path());

        let mut deferred = open(&path).unwrap();
        let deferred_tx = deferred.transaction().unwrap();
        let _: String = deferred_tx
            .query_row("SELECT value FROM records WHERE id='one'", [], |row| row.get(0))
            .unwrap();
        open(&path)
            .unwrap()
            .execute("UPDATE records SET value='other-writer' WHERE id='one'", [])
            .unwrap();
        let upgrade_error = deferred_tx
            .execute("UPDATE records SET value='deferred' WHERE id='one'", [])
            .unwrap_err();
        assert!(is_transient_lock(&upgrade_error));
        drop(deferred_tx);

        let mut immediate = open(&path).unwrap();
        let immediate_tx = immediate_transaction(&mut immediate).unwrap();
        let _: String = immediate_tx
            .query_row("SELECT value FROM records WHERE id='one'", [], |row| row.get(0))
            .unwrap();
        let competing = {
            let connection = open(&path).unwrap();
            connection.busy_timeout(Duration::ZERO).unwrap();
            connection
                .execute("UPDATE records SET value='competing' WHERE id='one'", [])
                .unwrap_err()
        };
        assert!(is_transient_lock(&competing));
        immediate_tx
            .execute("UPDATE records SET value='immediate' WHERE id='one'", [])
            .unwrap();
        immediate_tx.commit().unwrap();

        assert_eq!(
            open(&path)
                .unwrap()
                .query_row("SELECT value FROM records WHERE id='one'", [], |row| row
                    .get::<_, String>(0))
                .unwrap(),
            "immediate"
        );
    }

    #[test]
    fn capacity_failure_after_copy_removes_partial_backup_and_preserves_source() {
        let root = tempfile::tempdir().unwrap();
        let path = wal_database(root.path());
        let backup = root.path().join("capacity.sqlite3");
        let manifest = root.path().join("capacity.manifest.json");

        let error = create_verified_backup_at(&path, 7, &backup, &manifest, |_| {
            Err("migration_backup_write_failed:no_space".into())
        })
        .unwrap_err();
        assert_eq!(error, "migration_backup_write_failed:no_space");
        assert!(!backup.exists());
        assert!(!manifest.exists());
        assert_eq!(
            Connection::open(&path)
                .unwrap()
                .query_row("SELECT value FROM records WHERE id='one'", [], |row| row
                    .get::<_, String>(
                    0
                ))
                .unwrap(),
            "before"
        );
    }
}
