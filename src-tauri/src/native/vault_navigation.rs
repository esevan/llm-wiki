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
