use std::fs::{File, OpenOptions};
use std::path::Path;

/// An OS-released lock acquired before the GUI opens its persistence adapters.
/// The stdio transport never acquires this lock or opens the database.
pub struct GuiOwnerLock {
    _file: File,
}

impl GuiOwnerLock {
    pub fn acquire(database: &Path) -> Result<Self, String> {
        let parent = database
            .parent()
            .ok_or("Database directory is unavailable")?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let path = database.with_extension("gui-owner.lock");
        if let Ok(metadata) = std::fs::symlink_metadata(&path) {
            if !metadata.is_file() || metadata.file_type().is_symlink() {
                return Err("GUI owner lock is not a regular file".into());
            }
        }
        let mut options = OpenOptions::new();
        options.create(true).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let file = options
            .open(&path)
            .map_err(|error| format!("GUI owner lock: {error}"))?;
        file.try_lock()
            .map_err(|_| "Another LLM Wiki GUI already owns this database".to_owned())?;
        Ok(Self { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn one_owner_per_database_and_release_on_exit() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("state.sqlite");
        let first = GuiOwnerLock::acquire(&db).unwrap();
        assert!(GuiOwnerLock::acquire(&db).is_err());
        assert!(
            !db.exists(),
            "acquiring ownership must not open persistence"
        );
        drop(first);
        assert!(GuiOwnerLock::acquire(&db).is_ok());
    }
}
