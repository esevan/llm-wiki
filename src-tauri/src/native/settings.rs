use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub enum VaultStartup {
    Configured(PathBuf),
    Pending,
    Unset,
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct AppSettings {
    version: u8,
    vault_path: Option<PathBuf>,
    vault_setup_pending: bool,
    intro_completed: Option<bool>,
    locale: Option<SavedLocale>,
    provider: Option<SavedProvider>,
    codex_home: Option<PathBuf>,
}

#[derive(Clone, Deserialize, Serialize)]
struct SavedLocale {
    value: String,
    explicit: bool,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, rename_all = "camelCase")]
struct SavedProvider {
    base_url: String,
    model: String,
    advanced_model: String,
    advanced_tasks: Value,
    report_language: String,
    async_worker_count: i64,
    request_timeout_seconds: u64,
    api_key: Option<String>,
}

impl Default for SavedProvider {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".into(),
            model: String::new(),
            advanced_model: String::new(),
            advanced_tasks: json!({}),
            report_language: "ko".into(),
            async_worker_count: 2,
            request_timeout_seconds: DEFAULT_AI_TIMEOUT_SECONDS,
            api_key: None,
        }
    }
}

const DEFAULT_AI_TIMEOUT_SECONDS: u64 = 180;

/// One process-wide settings owner. Paths keep isolated application/test homes separate.
/// Readers receive snapshots; only this store can publish a persisted update.
#[derive(Default)]
struct SettingsStore {
    snapshots: Mutex<HashMap<PathBuf, AppSettings>>,
}

impl SettingsStore {
    fn shared() -> &'static Self {
        static INSTANCE: OnceLock<SettingsStore> = OnceLock::new();
        INSTANCE.get_or_init(Self::default)
    }

    fn read(&self, path: &Path) -> Result<AppSettings, String> {
        let mut snapshots = self.snapshots.lock()
            .map_err(|_| "Application settings lock is unavailable")?;
        if let Some(settings) = snapshots.get(path) {
            return Ok(settings.clone());
        }
        let settings = read_disk(path)?;
        snapshots.insert(path.to_owned(), settings.clone());
        Ok(settings)
    }

    fn update(&self, path: &Path, change: impl FnOnce(&mut AppSettings)) -> Result<AppSettings, String> {
        let mut snapshots = self.snapshots.lock()
            .map_err(|_| "Application settings lock is unavailable")?;
        let mut settings = match snapshots.get(path) {
            Some(settings) => settings.clone(),
            None => read_disk(path)?,
        };
        settings.version = 2;
        change(&mut settings);
        write_disk(path, &settings)?;
        // Failed writes never expose unpersisted values to another reader.
        snapshots.insert(path.to_owned(), settings.clone());
        Ok(settings)
    }
}

const TASKS: &[(&str, bool)] = &[
    ("capture_assistance", true),
    ("problem_drafting", true),
    ("problem_assistance", true),
    ("workbench_organization", false),
    ("solution_drafting", true),
    ("solution_assistance", true),
    ("completed_solution_chat", false),
    ("conflict_review", true),
    ("image_summary", true),
    ("completion_review", true),
    ("completion_report", true),
    ("lineage_inference", true),
    ("problem_enrichment", false),
    ("knowledge_translation", false),
];

fn read_disk(path: &Path) -> Result<AppSettings, String> {
    if !path.is_file() {
        return Ok(AppSettings::default());
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| {
        format!(
            "Application settings are invalid at {}: {error}",
            path.display()
        )
    })
}

fn write_disk(path: &Path, settings: &AppSettings) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Application settings path has no parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|error| error.to_string())?;
    }
    let temporary = parent.join(format!(".settings.{}.tmp", uuid::Uuid::new_v4()));
    let content = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(format!("{content}\n").as_bytes())
            .map_err(|error| error.to_string())?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }
    #[cfg(not(unix))]
    fs::write(&temporary, format!("{content}\n")).map_err(|error| error.to_string())?;
    if let Err(error) = crate::native::vault::replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    Ok(())
}

fn read(path: &Path) -> Result<AppSettings, String> {
    SettingsStore::shared().read(path)
}

fn update(path: &Path, change: impl FnOnce(&mut AppSettings)) -> Result<AppSettings, String> {
    SettingsStore::shared().update(path, change)
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool, String> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?)",
            [table],
            |row| row.get(0),
        )
        .map_err(|error| error.to_string())
}

pub fn migrate_legacy(db_path: &Path, settings_path: &Path) -> Result<(), String> {
    if settings_path.is_file() || !db_path.is_file() {
        return Ok(());
    }
    let connection = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| error.to_string())?;
    let mut settings = AppSettings {
        version: 2,
        ..AppSettings::default()
    };
    let mut found = false;
    if table_exists(&connection, "app_settings")? {
        settings.vault_path = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key='vault_path'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        settings.vault_setup_pending = connection
            .query_row(
                "SELECT value FROM app_settings WHERE key='vault_setup_pending'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .as_deref()
            == Some("1");
        found |= settings.vault_path.is_some() || settings.vault_setup_pending;
    }
    if table_exists(&connection, "locale_settings")? {
        settings.locale = connection
            .query_row(
                "SELECT locale,explicit FROM locale_settings WHERE id=1",
                [],
                |row| {
                    Ok(SavedLocale {
                        value: row.get(0)?,
                        explicit: row.get::<_, i64>(1)? != 0,
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())?;
        found |= settings.locale.is_some();
    }
    if table_exists(&connection, "provider_settings")? {
        settings.provider = connection
            .query_row(
                "SELECT base_url,model,advanced_model,advanced_tasks,report_language,async_worker_count FROM provider_settings WHERE id=1",
                [],
                |row| {
                    let advanced_tasks = row.get::<_, String>(3)?;
                    Ok(SavedProvider {
                        base_url: row.get(0)?,
                        model: row.get(1)?,
                        advanced_model: row.get(2)?,
                        advanced_tasks: serde_json::from_str(&advanced_tasks).unwrap_or_else(|_| json!({})),
                        report_language: row.get(4)?,
                        async_worker_count: row.get(5)?,
                        request_timeout_seconds: DEFAULT_AI_TIMEOUT_SECONDS,
                        api_key: None,
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())?;
        found |= settings.provider.is_some();
    }
    if found {
        update(settings_path, |saved| *saved = settings)?;
    }
    Ok(())
}

pub fn vault_startup(settings_path: &Path) -> Result<VaultStartup, String> {
    let settings = read(settings_path)?;
    if let Some(path) = settings.vault_path {
        return Ok(VaultStartup::Configured(path));
    }
    Ok(if settings.vault_setup_pending {
        VaultStartup::Pending
    } else {
        VaultStartup::Unset
    })
}

pub fn mark_vault_setup_pending(settings_path: &Path, intro_required: bool) -> Result<(), String> {
    update(settings_path, |settings| {
        settings.vault_setup_pending = true;
        if intro_required {
            settings.intro_completed = Some(false);
        }
    })?;
    Ok(())
}

pub fn intro_required(settings_path: &Path) -> Result<bool, String> {
    Ok(read(settings_path)?.intro_completed == Some(false))
}

pub fn complete_intro(settings_path: &Path) -> Result<(), String> {
    update(settings_path, |settings| {
        settings.intro_completed = Some(true);
    })?;
    Ok(())
}

pub fn save_vault_path(settings_path: &Path, vault: &Path) -> Result<(), String> {
    update(settings_path, |settings| {
        settings.vault_path = Some(vault.to_owned());
        settings.vault_setup_pending = false;
    })?;
    Ok(())
}

/// Returns the Codex state exposed to the Settings screen.  Keeping the
/// alternate value absent is intentional: Codex then chooses its normal local
/// home (`~/.codex`) just as the CLI and IDE extension do.
pub fn codex_home(settings_path: &Path) -> Result<Value, String> {
    let alternate_path = read(settings_path)?.codex_home;
    let default_path = dirs::home_dir()
        .map(|home| home.join(".codex").to_string_lossy().into_owned())
        .unwrap_or_else(|| "~/.codex".into());
    Ok(json!({
        "mode": if alternate_path.is_some() { "alternate" } else { "default" },
        "defaultPath": default_path,
        "alternatePath": alternate_path.map(|path| path.to_string_lossy().into_owned()),
    }))
}

/// Save only an explicit, existing alternate home. An empty value restores the
/// shared Codex default rather than persisting a second implicit location.
pub fn save_codex_home(settings_path: &Path, input: &Value) -> Result<Value, String> {
    let alternate = input
        .get("alternatePath")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let alternate = match alternate {
        Some(path) => {
            if !path.is_absolute() || !path.is_dir() {
                return Err("Choose an existing absolute Codex home folder".into());
            }
            Some(path.canonicalize().map_err(|error| error.to_string())?)
        }
        None => None,
    };
    update(settings_path, |settings| settings.codex_home = alternate)?;
    codex_home(settings_path)
}

pub fn configured_codex_home(settings_path: &Path) -> Result<Option<PathBuf>, String> {
    Ok(read(settings_path)?.codex_home)
}

pub fn resources(locale: &str) -> Result<Value, String> {
    let raw = match locale {
        "en" => include_str!("../../../frontend/public/i18n/en.json"),
        "ko" => include_str!("../../../frontend/public/i18n/ko.json"),
        _ => return Err("Unsupported locale".into()),
    };
    serde_json::from_str(raw).map_err(|error| error.to_string())
}

pub fn locale(settings_path: &Path, browser_locale: &str) -> Result<Value, String> {
    let saved = read(settings_path)?.locale;
    let locale = saved
        .as_ref()
        .map(|value| value.value.as_str())
        .unwrap_or_else(|| {
            if browser_locale.to_lowercase().starts_with("ko") {
                "ko"
            } else {
                "en"
            }
        });
    Ok(json!({"locale":locale,"explicit":saved.is_some_and(|value| value.explicit)}))
}

pub fn save_locale(settings_path: &Path, input: &Value) -> Result<Value, String> {
    let locale = input.get("locale").and_then(Value::as_str).unwrap_or("");
    if !matches!(locale, "ko" | "en") {
        return Err("Unsupported locale".into());
    }
    update(settings_path, |settings| {
        settings.locale = Some(SavedLocale {
            value: locale.into(),
            explicit: true,
        });
    })?;
    Ok(json!({"locale":locale,"explicit":true}))
}

pub fn provider(settings_path: &Path) -> Result<Value, String> {
    let provider = read(settings_path)?.provider.unwrap_or_default();
    let mut tasks = serde_json::Map::new();
    for (name, default) in TASKS {
        tasks.insert(
            (*name).into(),
            provider
                .advanced_tasks
                .get(*name)
                .and_then(Value::as_bool)
                .unwrap_or(*default)
                .into(),
        );
    }
    Ok(
        json!({"base_url":provider.base_url,"model":provider.model,"advanced_model":provider.advanced_model,"advanced_tasks":tasks,"report_language":provider.report_language,"async_worker_count":provider.async_worker_count,"request_timeout_seconds":provider.request_timeout_seconds,"api_key_configured":provider.api_key.as_ref().is_some_and(|key| !key.is_empty())}),
    )
}

pub fn save_provider(settings_path: &Path, input: &Value) -> Result<Value, String> {
    let base_url = input
        .get("base_url")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim_end_matches('/');
    if !(base_url.starts_with("http://127.0.0.1:") || base_url.starts_with("https://")) {
        return Err("Provider URL must use HTTPS or loopback HTTP".into());
    }
    let model = input.get("model").and_then(Value::as_str).unwrap_or("");
    let advanced_model = input
        .get("advanced_model")
        .and_then(Value::as_str)
        .unwrap_or("");
    let advanced_tasks = input
        .get("advanced_tasks")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let report_language = input
        .get("report_language")
        .and_then(Value::as_str)
        .unwrap_or("ko");
    let workers = input
        .get("async_worker_count")
        .and_then(Value::as_i64)
        .unwrap_or(2);
    if !(1..=32).contains(&workers) {
        return Err("Async worker count must be between 1 and 32".into());
    }
    let timeout = input.get("request_timeout_seconds").map(|value| {
        value.as_u64().filter(|seconds| (1..=3600).contains(seconds))
            .ok_or("AI request timeout must be a whole number between 1 and 3600 seconds")
    }).transpose()?;
    let api_key = input
        .get("api_key")
        .and_then(Value::as_str)
        .filter(|secret| !secret.is_empty())
        .map(str::to_owned);
    update(settings_path, |settings| {
        let existing_api_key = settings
            .provider
            .as_ref()
            .and_then(|provider| provider.api_key.clone());
        settings.provider = Some(SavedProvider {
            base_url: base_url.into(),
            model: model.into(),
            advanced_model: advanced_model.into(),
            advanced_tasks,
            report_language: report_language.into(),
            async_worker_count: workers,
            request_timeout_seconds: timeout.unwrap_or_else(|| settings.provider.as_ref()
                .map(|provider| provider.request_timeout_seconds)
                .unwrap_or(DEFAULT_AI_TIMEOUT_SECONDS)),
            api_key: api_key.or(existing_api_key),
        });
    })?;
    provider(settings_path)
}

/// All provider HTTP requests use the latest persisted application setting.
/// Existing requests retain their deadline; saving applies to the next request.
pub(crate) fn provider_client(settings_path: &Path) -> Result<reqwest::Client, String> {
    let seconds = read(settings_path)?.provider.unwrap_or_default().request_timeout_seconds;
    if !(1..=3600).contains(&seconds) {
        return Err("AI request timeout must be between 1 and 3600 seconds".into());
    }
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(seconds))
        .build()
        .map_err(|error| error.to_string())
}

pub fn provider_credentials_for(
    settings_path: &Path,
    task: &str,
) -> Result<(String, String, String), String> {
    let provider = read(settings_path)?.provider.unwrap_or_default();
    let advanced = provider
        .advanced_tasks
        .get(task)
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            TASKS
                .iter()
                .find(|(name, _)| *name == task)
                .map(|(_, enabled)| *enabled)
                .unwrap_or(false)
        });
    let api_key = provider
        .api_key
        .filter(|key| !key.is_empty())
        .ok_or("Configure an API key in AI setup before using AI")?;
    let model = if advanced && !provider.advanced_model.trim().is_empty() {
        provider.advanced_model
    } else {
        provider.model
    };
    Ok((provider.base_url, model, api_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn singleton_reuses_snapshots_and_keeps_settings_homes_isolated() {
        assert!(std::ptr::eq(SettingsStore::shared(), SettingsStore::shared()));
        let state = tempfile::tempdir().unwrap();
        let first = state.path().join("first.json");
        let second = state.path().join("second.json");
        save_locale(&first, &json!({"locale":"ko"})).unwrap();
        save_locale(&second, &json!({"locale":"en"})).unwrap();
        // A second read comes from the shared object, not another disk read.
        fs::remove_file(&first).unwrap();
        assert_eq!(read(&first).unwrap().locale.unwrap().value, "ko");
        assert_eq!(read(&second).unwrap().locale.unwrap().value, "en");
        save_locale(&first, &json!({"locale":"en"})).unwrap();
        assert_eq!(read(&first).unwrap().locale.unwrap().value, "en");
        assert!(fs::read_to_string(&first).unwrap().contains("en"));
    }

    #[test]
    fn singleton_does_not_publish_failed_writes() {
        let state = tempfile::tempdir().unwrap();
        let blocked = state.path().join("blocked");
        fs::write(&blocked, "not a directory").unwrap();
        let path = blocked.join("settings.json");
        assert!(read(&path).unwrap().locale.is_none());
        assert!(save_locale(&path, &json!({"locale":"ko"})).is_err());
        assert!(read(&path).unwrap().locale.is_none());
    }

    #[test]
    fn singleton_serializes_concurrent_changes_without_losing_other_fields() {
        let state = tempfile::tempdir().unwrap();
        let path = state.path().join("settings.json");
        let locale_path = path.clone();
        let provider_path = path.clone();
        let locale = std::thread::spawn(move || {
            save_locale(&locale_path, &json!({"locale":"ko"})).unwrap();
        });
        let provider = std::thread::spawn(move || {
            save_provider(&provider_path, &json!({"base_url":"https://example.test/v1","request_timeout_seconds":600})).unwrap();
        });
        locale.join().unwrap();
        provider.join().unwrap();
        let settings = read(&path).unwrap();
        assert_eq!(settings.locale.unwrap().value, "ko");
        assert_eq!(settings.provider.unwrap().request_timeout_seconds, 600);
        let persisted: AppSettings = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(persisted.locale.unwrap().value, "ko");
        assert_eq!(persisted.provider.unwrap().request_timeout_seconds, 600);
    }

    #[test]
    fn provider_timeout_defaults_persists_and_survives_unrelated_updates() {
        let state = tempfile::tempdir().unwrap();
        let settings_path = state.path().join("settings.json");
        assert_eq!(provider(&settings_path).unwrap()["request_timeout_seconds"], 180);
        let legacy_path = state.path().join("legacy-settings.json");
        fs::write(&legacy_path, r#"{"provider":{"baseUrl":"https://example.test/v1"}}"#).unwrap();
        assert_eq!(provider(&legacy_path).unwrap()["request_timeout_seconds"], 180);
        assert_eq!(provider(&settings_path).unwrap()["request_timeout_seconds"], 180);
        let input = json!({"base_url":"https://example.test/v1","request_timeout_seconds":600});
        assert_eq!(save_provider(&settings_path, &input).unwrap()["request_timeout_seconds"], 600);
        assert_eq!(read(&settings_path).unwrap().provider.unwrap().request_timeout_seconds, 600);
        save_provider(&settings_path, &json!({"base_url":"https://example.test/v1","model":"updated"})).unwrap();
        assert_eq!(provider(&settings_path).unwrap()["request_timeout_seconds"], 600);
    }

    #[test]
    fn invalid_provider_timeouts_do_not_change_saved_configuration() {
        let state = tempfile::tempdir().unwrap();
        let settings_path = state.path().join("settings.json");
        save_provider(&settings_path, &json!({"base_url":"https://example.test/v1","request_timeout_seconds":240})).unwrap();
        let before = fs::read_to_string(&settings_path).unwrap();
        for timeout in [json!(0), json!(-1), json!(3601), json!(1.5), json!("180"), Value::Null] {
            assert!(save_provider(&settings_path, &json!({"base_url":"https://example.test/v1","request_timeout_seconds":timeout})).is_err());
            assert_eq!(fs::read_to_string(&settings_path).unwrap(), before);
        }
    }

    #[tokio::test]
    async fn provider_timeout_covers_body_and_changes_apply_to_new_requests() {
        use std::io::Write;
        use std::net::TcpListener;
        use std::time::Duration;

        let state = tempfile::tempdir().unwrap();
        let settings_path = state.path().join("settings.json");
        save_provider(&settings_path, &json!({"base_url":"https://example.test/v1","request_timeout_seconds":1})).unwrap();
        let short_client = provider_client(&settings_path).unwrap();
        save_provider(&settings_path, &json!({"base_url":"https://example.test/v1","request_timeout_seconds":3})).unwrap();
        let long_client = provider_client(&settings_path).unwrap();
        for (client, should_timeout) in [(short_client, true), (long_client, false)] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                use std::io::Read;
                let (mut stream, _) = listener.accept().unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut request = [0; 4096];
                stream.read(&mut request).unwrap();
                stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\na").unwrap();
                std::thread::sleep(Duration::from_millis(1500));
                let _ = stream.write_all(b"b");
            });
            let response = client.get(format!("http://{address}")).send().await.unwrap();
            let body = response.text().await;
            if should_timeout { assert!(body.unwrap_err().is_timeout()); }
            else { assert_eq!(body.unwrap(), "ab"); }
            server.join().unwrap();
        }
    }

    #[test]
    fn provider_key_is_stored_locally_but_never_returned() {
        let state = tempfile::tempdir().unwrap();
        let settings_path = state.path().join("settings.json");

        let public = save_provider(
            &settings_path,
            &json!({"base_url":"https://example.test/v1","model":"test","api_key":"test-only-key"}),
        )
        .unwrap();

        assert!(public.get("api_key").is_none());
        assert_eq!(public["api_key_configured"], true);
        assert!(fs::read_to_string(&settings_path)
            .unwrap()
            .contains("test-only-key"));
        assert_eq!(
            provider_credentials_for(&settings_path, "capture_assistance")
                .unwrap()
                .2,
            "test-only-key"
        );
    }

    #[test]
    fn unrelated_provider_edits_preserve_the_api_key() {
        let state = tempfile::tempdir().unwrap();
        let settings_path = state.path().join("settings.json");
        save_provider(
            &settings_path,
            &json!({"base_url":"https://example.test/v1","model":"original","api_key":"test-only-key"}),
        )
        .unwrap();

        let public = save_provider(
            &settings_path,
            &json!({"base_url":"https://example.test/v1","model":"updated"}),
        )
        .unwrap();

        assert!(public.get("api_key").is_none());
        assert_eq!(public["api_key_configured"], true);
        assert_eq!(
            provider_credentials_for(&settings_path, "capture_assistance")
                .unwrap()
                .2,
            "test-only-key"
        );
    }

    #[test]
    fn empty_api_key_preserves_the_stored_key() {
        let state = tempfile::tempdir().unwrap();
        let settings_path = state.path().join("settings.json");
        save_provider(
            &settings_path,
            &json!({"base_url":"https://example.test/v1","api_key":"test-only-key"}),
        )
        .unwrap();

        save_provider(
            &settings_path,
            &json!({"base_url":"https://example.test/v1","api_key":""}),
        )
        .unwrap();

        assert_eq!(
            provider_credentials_for(&settings_path, "capture_assistance")
                .unwrap()
                .2,
            "test-only-key"
        );
    }

    #[test]
    fn legacy_provider_settings_without_an_api_key_remain_readable() {
        let state = tempfile::tempdir().unwrap();
        let settings_path = state.path().join("settings.json");
        fs::write(
            &settings_path,
            r#"{"version":2,"provider":{"baseUrl":"https://example.test/v1","model":"test"}}"#,
        )
        .unwrap();

        let public = provider(&settings_path).unwrap();
        assert_eq!(public["base_url"], "https://example.test/v1");
        assert_eq!(public["api_key_configured"], false);
    }

    #[test]
    fn codex_home_defaults_to_the_shared_local_home_and_only_persists_an_explicit_alternate() {
        let state = tempfile::tempdir().unwrap();
        let settings_path = state.path().join("settings.json");
        let default = codex_home(&settings_path).unwrap();
        assert_eq!(default["mode"], "default");
        assert!(default["alternatePath"].is_null());

        let alternate = state.path().join("alternate-codex");
        fs::create_dir(&alternate).unwrap();
        let saved = save_codex_home(&settings_path, &json!({"alternatePath":alternate})).unwrap();
        assert_eq!(saved["mode"], "alternate");
        assert_eq!(configured_codex_home(&settings_path).unwrap(), Some(alternate.canonicalize().unwrap()));

        let restored = save_codex_home(&settings_path, &json!({"alternatePath":""})).unwrap();
        assert_eq!(restored["mode"], "default");
        assert_eq!(configured_codex_home(&settings_path).unwrap(), None);
    }

    #[test]
    #[cfg(unix)]
    fn settings_with_an_api_key_are_owner_readable_only() {
        use std::os::unix::fs::PermissionsExt;

        let state = tempfile::tempdir().unwrap();
        let settings_path = state.path().join("settings.json");
        save_provider(
            &settings_path,
            &json!({"base_url":"https://example.test/v1","api_key":"test-only-key"}),
        )
        .unwrap();

        assert_eq!(
            fs::metadata(settings_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
