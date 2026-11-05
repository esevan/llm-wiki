//! Read-only Vault navigation. Every file read is canonicalized inside the Vault.
use super::vault::{document_title, NOTE_READ_LIMIT};
use crate::domain::retrieval::is_indexable_path;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use walkdir::WalkDir;

const IMAGE_LIMIT: u64 = 10 * 1024 * 1024;

fn bounded_read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|_| "Vault file is unavailable")?;
    if file
        .metadata()
        .map_err(|_| "Vault file is unavailable")?
        .len()
        > limit
    {
        return Err("Vault file exceeds the reader size limit".into());
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Vault file is unavailable")?;
    if bytes.len() as u64 > limit {
        return Err("Vault file exceeds the reader size limit".into());
    }
    Ok(bytes)
}

fn contained_file(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty()
        || relative.contains(['\\', ':', '\0'])
        || Path::new(relative).is_absolute()
    {
        return Err("Vault path is not a relative file".into());
    }
    let root = root.canonicalize().map_err(|_| "Vault is unavailable")?;
    let path = root
        .join(relative)
        .canonicalize()
        .map_err(|_| "Vault file is unavailable")?;
    let inner = path
        .strip_prefix(&root)
        .map_err(|_| "Vault path is outside the Vault")?;
    if !path.is_file() || !is_indexable_path(&inner.to_string_lossy()) {
        return Err("Vault file is unavailable".into());
    }
    Ok(path)
}

pub fn browse(vault: &Path) -> Result<Value, String> {
    let root = vault.canonicalize().map_err(|_| "Vault is unavailable")?;
    let mut notes = Vec::new();
    let mut folders = Vec::new();
    let entries = WalkDir::new(&root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry
                .path()
                .strip_prefix(&root)
                .map(|p| {
                    is_indexable_path(&p.to_string_lossy())
                        && !p
                            .components()
                            .any(|c| matches!(c.as_os_str().to_str(), Some(".git" | ".obsidian")))
                })
                .unwrap_or(false)
        });
    for entry in entries {
        let entry =
            entry.map_err(|_| "A Vault folder is unavailable; retry after checking access")?;
        let relative = entry
            .path()
            .strip_prefix(&root)
            .map_err(|_| "Vault path is outside the Vault")?
            .to_string_lossy()
            .replace('\\', "/");
        if entry.file_type().is_dir() && !relative.is_empty() {
            folders.push(relative);
            continue;
        }
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|v| v.to_str()) != Some("md")
        {
            continue;
        }
        // Listing remains useful when an individual note is oversized or unreadable.
        let body = bounded_read(entry.path(), NOTE_READ_LIMIT)
            .ok()
            .and_then(|b| String::from_utf8(b).ok());
        let content = body.as_deref().unwrap_or("");
        let normalized = content.replace("\r\n", "\n");
        let metadata = normalized
            .strip_prefix("---\n")
            .and_then(|rest| rest.split_once("\n---"))
            .and_then(|(head, _)| serde_yaml::from_str::<Value>(head).ok())
            .unwrap_or(Value::Null);
        let is_moc = relative
            .split('/')
            .any(|part| part.eq_ignore_ascii_case("mocs") || part.eq_ignore_ascii_case("moc"))
            || entry
                .path()
                .file_stem()
                .is_some_and(|stem| stem.to_string_lossy().to_lowercase().ends_with("moc"))
            || metadata["llm_wiki"]["information_type"]
                .as_str()
                .is_some_and(|v| v.eq_ignore_ascii_case("moc"))
            || ["type", "information_type"].iter().any(|key| {
                metadata[key]
                    .as_str()
                    .is_some_and(|v| v.eq_ignore_ascii_case("moc"))
            });
        let modified = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
            .map(|v| v.as_secs());
        notes.push(json!({"path":relative,"title":document_title(entry.path(),content),"isMoc":is_moc,"modifiedAt":modified,
            "managed":metadata["llm_wiki_managed"].as_bool().unwrap_or(false),"readable":body.is_some()}));
        if notes.len() > 10000 {
            return Err("Vault navigation supports up to 10000 Markdown notes".into());
        }
    }
    notes.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
    folders.sort();
    Ok(json!({"notes":notes,"folders":folders}))
}

pub fn image(vault: &Path, document: &str, source: &str) -> Result<Value, String> {
    let note = contained_file(vault, document)?;
    if note.extension().and_then(|v| v.to_str()) != Some("md") {
        return Err("Knowledge document not found".into());
    }
    if source.is_empty()
        || source.contains(['\\', ':', '\0', '?', '#'])
        || Path::new(source).is_absolute()
    {
        return Err("Only relative Vault images are supported".into());
    }
    let parent = Path::new(document).parent().unwrap_or(Path::new(""));
    let relative = parent.join(source);
    let path = contained_file(vault, &relative.to_string_lossy())?;
    let bytes = bounded_read(&path, IMAGE_LIMIT)?;
    let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "image/png"
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        "image/jpeg"
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        "image/gif"
    } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
        "image/webp"
    } else {
        return Err("Supported Vault images: PNG, JPEG, GIF and WebP".into());
    };
    Ok(json!({"dataUrl":format!("data:{mime};base64,{}",STANDARD.encode(bytes))}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lists_live_folders_and_authored_mocs_without_recovery_or_symlinks() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("MOCs")).unwrap();
        fs::create_dir_all(root.path().join("Empty")).unwrap();
        fs::create_dir_all(root.path().join(".llm-wiki-withdrawn")).unwrap();
        fs::write(root.path().join("MOCs/map.md"), "# My map\n\n[[Guide]]").unwrap();
        fs::write(
            root.path().join(".llm-wiki-withdrawn/private.md"),
            "private",
        )
        .unwrap();
        let result = browse(root.path()).unwrap();
        assert_eq!(result["notes"].as_array().unwrap().len(), 1);
        assert_eq!(result["notes"][0]["title"], "My map");
        assert_eq!(result["notes"][0]["isMoc"], true);
        assert!(result["folders"]
            .as_array()
            .unwrap()
            .contains(&json!("Empty")));
    }
    #[test]
    fn permits_parent_images_inside_vault_and_rejects_escape_and_active_content() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("Guides")).unwrap();
        fs::write(root.path().join("Guides/note.md"), "# Note").unwrap();
        fs::write(root.path().join("image.png"), b"\x89PNG\r\n\x1a\nfixture").unwrap();
        assert!(
            image(root.path(), "Guides/note.md", "../image.png").unwrap()["dataUrl"]
                .as_str()
                .unwrap()
                .starts_with("data:image/png;base64,")
        );
        for src in [
            "../../outside.png",
            "/etc/passwd",
            "file:///etc/passwd",
            "https://example.com/a.png",
            "..\\image.png",
        ] {
            assert!(image(root.path(), "Guides/note.md", src).is_err());
        }
        fs::write(root.path().join("evil.svg"), "<svg onload='alert(1)'/>").unwrap();
        assert!(image(root.path(), "Guides/note.md", "../evil.svg").is_err());
        #[cfg(unix)]
        {
            let outside = tempfile::tempdir().unwrap();
            fs::write(outside.path().join("image.png"), b"\x89PNG\r\n\x1a\n").unwrap();
            std::os::unix::fs::symlink(outside.path(), root.path().join("linked")).unwrap();
            assert!(image(root.path(), "Guides/note.md", "../linked/image.png").is_err());
        }
    }
}

/// Resolve stored publication lineage, never titles or authored Markdown links.
pub fn associated_tasks(db: &Path, path: &str) -> Result<Value, String> {
    let connection = super::database::open(db)?;
    let mut statement = connection
        .prepare(
            "SELECT DISTINCT t.id,r.title,t.state
         FROM tasks t JOIN task_revisions r
           ON r.task_id=t.id AND r.revision=t.current_revision
         WHERE t.archived_at IS NULL AND t.id IN (
           SELECT task_id FROM knowledge_pointers
           WHERE publication_path=?1 COLLATE NOCASE AND published_revision IS NOT NULL
           UNION
           SELECT p.task_id FROM knowledge_archive_current c
           JOIN knowledge_archive_revisions v ON v.document_id=c.document_id
           JOIN knowledge_archive_operations o ON o.operation_id=v.operation_id
           JOIN knowledge_archive_proposals p ON p.proposal_id=o.proposal_id
           WHERE c.path=?1 COLLATE NOCASE AND c.withdrawn=0 AND v.withdrawn=0
             AND c.kind='knowledge'
         ) ORDER BY r.title,t.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = statement.query_map([path], |row| Ok(json!({"id":row.get::<_,String>(0)?,"title":row.get::<_,String>(1)?,"state":row.get::<_,String>(2)?}))).map_err(|e|e.to_string())?;
    let tasks = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(json!(tasks))
}

#[cfg(test)]
mod associated_task_tests {
    use super::*;
    #[test]
    fn publication_links_use_current_task_revisions_and_exclude_archived_or_unpublished_tasks() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("state.sqlite3");
        super::super::database::initialize(&db).unwrap();
        let c = super::super::database::open(&db).unwrap();
        c.execute(
            "INSERT INTO knowledge_applicability VALUES('app','','[]','[]','[]','[]')",
            [],
        )
        .unwrap();
        for (id, archived, published) in [
            ("one", None, Some(1)),
            ("two", None, Some(1)),
            ("archived", Some("now"), Some(1)),
            ("draft", None, None),
        ] {
            c.execute("INSERT INTO tasks(id,current_revision,archived_at,created_at,last_user_activity_at) VALUES(?,2,?,'now','now')",rusqlite::params![id,archived]).unwrap();
            c.execute("INSERT INTO task_revisions(task_id,revision,title,content_hash,created_at) VALUES(?,2,?,'hash','now')",rusqlite::params![id,format!("Current {id}")]).unwrap();
            c.execute("INSERT INTO knowledge_draft_versions(task_id,revision,derivation_kind,article_type,title,body_markdown,content_hash,applicability_id,result_json,quality_state,model_status) VALUES(?,1,'legacy','guide','Guide','Body','hash','app','{}','valid','supplied')",[id]).unwrap();
            c.execute("INSERT INTO knowledge_pointers(task_id,published_revision,publication_path) VALUES(?,?,'Knowledge/Guide.md')",rusqlite::params![id,published]).unwrap();
        }
        let tasks = associated_tasks(&db, "Knowledge/Guide.md").unwrap();
        assert_eq!(tasks.as_array().unwrap().len(), 2);
        assert_eq!(tasks[0]["title"], "Current one");
        assert_eq!(tasks[1]["id"], "two");
        assert_eq!(associated_tasks(&db, "Unrelated.md").unwrap(), json!([]));
        for (index, id) in [(1, "one"), (2, "two")] {
            c.execute("INSERT INTO knowledge_archive_proposals(proposal_id,request_id,request_hash,task_id,knowledge_revision,proposal_hash,outcome,payload_json) VALUES(?1,?1,'hash',?2,1,'hash','ready','{}')",rusqlite::params![format!("p{index}"),id]).unwrap();
            c.execute("INSERT INTO knowledge_archive_operations(operation_id,proposal_id,proposal_version,payload_hash,state) VALUES(?1,?2,1,'hash','completed')",rusqlite::params![format!("o{index}"),format!("p{index}")]).unwrap();
            c.execute("INSERT INTO knowledge_archive_revisions(document_id,revision,path,kind,body,operation_id) VALUES('doc',?1,'Knowledge/Old.md','knowledge','body',?2)",rusqlite::params![format!("r{index}"),format!("o{index}")]).unwrap();
        }
        c.execute("INSERT INTO knowledge_archive_current(document_id,revision,path,kind) VALUES('doc','r2','Knowledge/Moved.md','knowledge')",[]).unwrap();
        assert_eq!(associated_tasks(&db, "Knowledge/Moved.md").unwrap(), tasks);
        assert_eq!(
            associated_tasks(&db, "Knowledge/Old.md").unwrap(),
            json!([])
        );
        c.execute("UPDATE knowledge_archive_current SET withdrawn=1", [])
            .unwrap();
        assert_eq!(
            associated_tasks(&db, "Knowledge/Moved.md").unwrap(),
            json!([])
        );
    }
}
