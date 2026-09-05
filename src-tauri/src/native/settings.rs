use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

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
        }
    }
}

static SETTINGS_WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
type ApiKeyResult = Result<Option<String>, String>;
static API_KEY_CACHE: OnceLock<Mutex<Option<ApiKeyResult>>> = OnceLock::new();
static TEST_API_KEY: OnceLock<Mutex<Option<Option<String>>>> = OnceLock::new();
#[cfg(test)]
static KEYRING_OPERATION_COUNT: AtomicUsize = AtomicUsize::new(0);

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

fn key_entry() -> Result<keyring::Entry, String> {
    #[cfg(test)]
    KEYRING_OPERATION_COUNT.fetch_add(1, Ordering::SeqCst);
    keyring::Entry::new("llm-wiki", "provider-api-key").map_err(|error| error.to_string())
}

fn test_mode() -> bool {
    std::env::var("LLM_WIKI_TEST_MODE").as_deref() == Ok("1")
}

fn test_api_key() -> ApiKeyResult {
    let cached = TEST_API_KEY
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Test credential store is unavailable".to_string())?
        .clone();
    if let Some(secret) = cached {
        return Ok(secret);
    }
    Ok(std::env::var("LLM_WIKI_TEST_API_KEY")
        .ok()
        .filter(|secret| !secret.is_empty()))
}

fn set_test_api_key(secret: String) -> Result<(), String> {
    *TEST_API_KEY
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Test credential store is unavailable".to_string())? = Some(Some(secret));
    Ok(())
}

fn read_keychain_api_key() -> ApiKeyResult {
    match key_entry()?.get_password() {
        Ok(secret) if !secret.is_empty() => Ok(Some(secret)),
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("Could not access the OS credential store: {error}")),
    }
}

fn cached_api_key(read: impl FnOnce() -> ApiKeyResult) -> ApiKeyResult {
    let mut cache = API_KEY_CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Credential cache is unavailable".to_string())?;
    if let Some(result) = cache.as_ref() {
        return result.clone();
    }
    let result = read();
    *cache = Some(result.clone());
    result
}

fn cache_api_key(result: ApiKeyResult) -> Result<(), String> {
    *API_KEY_CACHE
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Credential cache is unavailable".to_string())? = Some(result);
    Ok(())
}

fn api_key() -> ApiKeyResult {
    if test_mode() {
        return test_api_key();
    }
    cached_api_key(read_keychain_api_key)
}

fn read(path: &Path) -> Result<AppSettings, String> {
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

fn write(path: &Path, settings: &AppSettings) -> Result<(), String> {
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
    fs::write(&temporary, format!("{content}\n")).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }
    if let Err(error) = crate::native::vault::replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    Ok(())
}

fn update(path: &Path, change: impl FnOnce(&mut AppSettings)) -> Result<AppSettings, String> {
    let _guard = SETTINGS_WRITE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "Application settings lock is unavailable")?;
    let mut settings = read(path)?;
    settings.version = 2;
    change(&mut settings);
    write(path, &settings)?;
    Ok(settings)
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
                    })
                },
            )
            .optional()
            .map_err(|error| error.to_string())?;
        found |= settings.provider.is_some();
    }
    if found {
        write(settings_path, &settings)?;
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
    let (api_key_configured, api_key_error) = match api_key() {
        Ok(secret) => (secret.is_some(), None),
        Err(error) => (false, Some(error)),
    };
    Ok(
        json!({"base_url":provider.base_url,"model":provider.model,"advanced_model":provider.advanced_model,"advanced_tasks":tasks,"report_language":provider.report_language,"async_worker_count":provider.async_worker_count,"api_key_configured":api_key_configured,"api_key_error":api_key_error}),
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
    update(settings_path, |settings| {
        settings.provider = Some(SavedProvider {
            base_url: base_url.into(),
            model: model.into(),
            advanced_model: advanced_model.into(),
            advanced_tasks,
            report_language: report_language.into(),
            async_worker_count: workers,
        });
    })?;
    if let Some(secret) = input
        .get("api_key")
        .and_then(Value::as_str)
        .filter(|secret| !secret.is_empty())
    {
        if test_mode() {
            set_test_api_key(secret.into())?;
        } else {
            key_entry()?
                .set_password(secret)
                .map_err(|error| error.to_string())?;
            cache_api_key(Ok(Some(secret.into())))?;
        }
    }
    provider(settings_path)
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
    let model = if advanced && !provider.advanced_model.trim().is_empty() {
        provider.advanced_model
    } else {
        provider.model
    };
    let api_key = api_key()?.ok_or("Configure an API key in AI setup before using AI")?;
    Ok((provider.base_url, model, api_key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Mutex;

    static TEST_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn reset_test_credential_state() {
        *TEST_API_KEY.get_or_init(|| Mutex::new(None)).lock().unwrap() = None;
        *API_KEY_CACHE.get_or_init(|| Mutex::new(None)).lock().unwrap() = None;
        KEYRING_OPERATION_COUNT.store(0, Ordering::SeqCst);
    }

    #[test]
    fn test_mode_never_reads_or_writes_the_os_credential_store() {
        let _guard = TEST_ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        std::env::set_var("LLM_WIKI_TEST_MODE", "1");
        std::env::remove_var("LLM_WIKI_TEST_API_KEY");
        reset_test_credential_state();
        let state = tempfile::tempdir().unwrap();
        let settings_path = state.path().join("settings.json");

        assert_eq!(api_key().unwrap(), None);
        save_provider(
            &settings_path,
            &json!({"base_url":"https://example.test/v1","model":"test","api_key":"test-only-key"}),
        )
        .unwrap();
        assert_eq!(api_key().unwrap().as_deref(), Some("test-only-key"));
        assert_eq!(KEYRING_OPERATION_COUNT.load(Ordering::SeqCst), 0);
        std::env::remove_var("LLM_WIKI_TEST_MODE");
        reset_test_credential_state();
    }

    #[test]
    fn cached_keychain_error_is_shared_by_concurrent_passive_reads() {
        let _guard = TEST_ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        std::env::remove_var("LLM_WIKI_TEST_MODE");
        reset_test_credential_state();
        let reads = std::sync::Arc::new(AtomicUsize::new(0));
        let mut workers = Vec::new();
        for _ in 0..8 {
            let reads = reads.clone();
            workers.push(std::thread::spawn(move || {
                cached_api_key(|| {
                    reads.fetch_add(1, Ordering::SeqCst);
                    Err("credential access denied".into())
                })
            }));
        }
        for worker in workers {
            assert_eq!(worker.join().unwrap(), Err("credential access denied".into()));
        }
        assert_eq!(reads.load(Ordering::SeqCst), 1);
        reset_test_credential_state();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn provider_credential_uses_macos_keychain() {
        assert!(key_entry()
            .unwrap()
            .get_credential()
            .is::<keyring::macos::MacCredential>());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn provider_credential_uses_windows_credential_manager() {
        assert!(key_entry()
            .unwrap()
            .get_credential()
            .is::<keyring::windows::WinCredential>());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn provider_credential_uses_linux_secret_service() {
        assert!(key_entry()
            .unwrap()
            .get_credential()
            .is::<keyring::secret_service::SsCredential>());
    }
}
