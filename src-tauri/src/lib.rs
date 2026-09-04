mod conversation;
mod desktop_e2e;
mod first_run;
mod native;
mod provider;

pub use native::{NativeApplication, NativeOperation, NativeResponse};
use std::path::PathBuf;
use tauri::Manager;

struct VaultResolution {
    path: PathBuf,
    setup_required: bool,
    persist_path: bool,
    intro_required: bool,
}

fn resolve_vault(
    default: PathBuf,
    db: &std::path::Path,
    settings_path: &std::path::Path,
    forced: Option<PathBuf>,
) -> Result<VaultResolution, String> {
    if let Some(path) = forced {
        return Ok(VaultResolution {
            path,
            setup_required: false,
            persist_path: false,
            intro_required: false,
        });
    }
    let database_existed = db.is_file();
    match native::settings::vault_startup(settings_path)? {
        native::settings::VaultStartup::Configured(path) if path.is_dir() => Ok(VaultResolution {
            path,
            setup_required: false,
            persist_path: false,
            intro_required: false,
        }),
        native::settings::VaultStartup::Configured(_) => Ok(VaultResolution {
            path: default,
            setup_required: true,
            persist_path: false,
            intro_required: false,
        }),
        native::settings::VaultStartup::Pending => {
            let intro_required = native::settings::intro_required(settings_path)?;
            Ok(VaultResolution {
                path: default,
                setup_required: true,
                persist_path: false,
                intro_required,
            })
        }
        native::settings::VaultStartup::Unset if database_existed => Ok(VaultResolution {
            path: default,
            setup_required: false,
            persist_path: true,
            intro_required: false,
        }),
        native::settings::VaultStartup::Unset => Ok(VaultResolution {
            path: default,
            setup_required: true,
            persist_path: false,
            intro_required: true,
        }),
    }
}

fn application_paths() -> Result<(VaultResolution, PathBuf, PathBuf), String> {
    let default_vault = dirs::document_dir()
        .map(|path| path.join("LLM Wiki Vault"))
        .ok_or("A local vault path is required")?;
    let data_dir = dirs::data_local_dir()
        .ok_or("The local application data directory is unavailable")?
        .join("LLM Wiki");
    let db = std::env::var_os("LLM_WIKI_DB")
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("llm-wiki.sqlite3"));
    let settings_dir = std::env::var_os("LLM_WORKBENCH_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|path| path.join(".llm-workbench")))
        .ok_or("The user home directory is unavailable")?;
    let settings_path = settings_dir.join("settings.json");
    if db.is_file() {
        native::database::initialize(&db)
            .map_err(|error| format!("Could not migrate the application database: {error}"))?;
    }
    native::settings::migrate_legacy(&db, &settings_path)
        .map_err(|error| format!("Could not import legacy application settings: {error}"))?;
    let forced = std::env::var_os("LLM_WIKI_VAULT").map(PathBuf::from);
    Ok((
        resolve_vault(default_vault, &db, &settings_path, forced)?,
        db,
        settings_path,
    ))
}

fn bundled_resource_dir(app: &tauri::App) -> Result<PathBuf, String> {
    let resource_dir = app.path().resource_dir();
    #[cfg(target_os = "macos")]
    let resource_dir = resource_dir.or_else(|error| {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("../Resources")))
            .and_then(|path| path.canonicalize().ok())
            .ok_or(error)
    });
    resource_dir.map_err(|error| format!("Could not resolve bundled resources: {error}"))
}

fn execute_domain(
    application: tauri::State<'_, NativeApplication>,
    domain: &str,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    Ok(application.execute_domain(domain, operation))
}

#[tauri::command]
fn system_command(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    execute_domain(application, "system", operation)
}

#[tauri::command]
fn vault_command(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    execute_domain(application, "vault", operation)
}

#[tauri::command]
fn settings_command(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    execute_domain(application, "settings", operation)
}

#[tauri::command]
async fn workflow_command(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    Ok(application.execute_workflow(operation).await)
}

#[tauri::command]
fn jobs_command(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    execute_domain(application, "jobs", operation)
}

#[tauri::command]
fn vault_setup_status(
    application: tauri::State<'_, NativeApplication>,
) -> Result<serde_json::Value, String> {
    application.vault_setup_status()
}

#[tauri::command]
async fn complete_first_run_intro(
    app: tauri::AppHandle,
    application: tauri::State<'_, NativeApplication>,
) -> Result<bool, String> {
    if first_run::complete_intro_and_choose_vault(&app, &application)? {
        app.restart();
    }
    Ok(false)
}

#[tauri::command]
async fn choose_vault(
    app: tauri::AppHandle,
    application: tauri::State<'_, NativeApplication>,
) -> Result<bool, String> {
    if first_run::choose_vault(&app, &application)? {
        app.restart();
    }
    Ok(false)
}

#[tauri::command]
async fn enqueue_ai_job(
    application: tauri::State<'_, NativeApplication>,
    operation: NativeOperation,
) -> Result<NativeResponse, String> {
    if operation.name != "jobs.enqueue" {
        return Ok(NativeResponse {
            status: 400,
            body: serde_json::json!({"detail":"Unsupported AI job command"}),
        });
    }
    Ok(application.enqueue_job(operation.input).await)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(conversation::RequestRegistry::default())
        .setup(|app| {
            let (vault, db, settings_path) = application_paths()
                .map_err(|error| format!("Could not resolve application paths: {error}"))?;
            let VaultResolution {
                path,
                setup_required,
                persist_path,
                intro_required,
            } = vault;
            let model_dir = bundled_resource_dir(app)?.join("resources/embedding-model");
            if setup_required {
                native::settings::mark_vault_setup_pending(&settings_path, intro_required)?;
            } else if persist_path {
                native::settings::save_vault_path(&settings_path, &path)?;
            }
            let application = NativeApplication::with_vault_setup(
                path,
                db,
                settings_path,
                Some(model_dir),
                setup_required,
            )
            .map_err(|error| format!("Could not initialize native application state: {error}"))?;
            let background_index = application.clone();
            let should_index = !setup_required;
            app.manage(application);
            if intro_required {
                first_run::create_intro_window(app)?;
            }
            if should_index {
                tauri::async_runtime::spawn_blocking(move || {
                    let _ = background_index.execute_domain(
                        "vault",
                        NativeOperation {
                            name: "vault.index".into(),
                            input: serde_json::json!({"semantic":true}),
                        },
                    );
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            system_command,
            vault_command,
            settings_command,
            workflow_command,
            jobs_command,
            enqueue_ai_job,
            vault_setup_status,
            complete_first_run_intro,
            choose_vault,
            conversation::conversation_stream,
            conversation::cancel_conversation,
            desktop_e2e::desktop_e2e_mode,
            desktop_e2e::desktop_e2e_complete,
            provider::provider_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running LLM Wiki desktop");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::settings::VaultStartup;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn first_launch_requires_a_vault_and_restores_the_selected_folder() {
        let state = tempdir().unwrap();
        let db = state.path().join("state.sqlite3");
        let settings = state.path().join("settings.json");
        let default = state.path().join("default-vault");
        let first_launch = resolve_vault(default.clone(), &db, &settings, None).unwrap();
        assert!(first_launch.setup_required);
        assert!(first_launch.intro_required);
        assert_eq!(first_launch.path, default);

        let application = NativeApplication::with_vault_setup(
            first_launch.path,
            db.clone(),
            settings.clone(),
            None,
            first_launch.setup_required,
        )
        .unwrap();
        native::settings::mark_vault_setup_pending(&settings, true).unwrap();
        assert_eq!(application.vault_setup_status().unwrap()["required"], true);
        assert_eq!(
            application.vault_setup_status().unwrap()["introRequired"],
            true
        );
        application.complete_first_run_intro().unwrap();
        assert_eq!(
            application.vault_setup_status().unwrap()["introRequired"],
            false
        );

        let selected = state.path().join("chosen-vault");
        std::fs::create_dir(&selected).unwrap();
        application.save_vault_selection(&selected).unwrap();
        let restored = resolve_vault(default, &db, &settings, None).unwrap();
        assert!(!restored.setup_required);
        assert_eq!(restored.path, selected.canonicalize().unwrap());
        assert!(matches!(
            native::settings::vault_startup(&settings).unwrap(),
            VaultStartup::Configured(_)
        ));

        std::fs::remove_dir(&selected).unwrap();
        let unavailable =
            resolve_vault(state.path().join("fallback"), &db, &settings, None).unwrap();
        assert!(unavailable.setup_required);
    }

    #[test]
    fn existing_installation_without_a_vault_setting_keeps_the_legacy_default() {
        let state = tempdir().unwrap();
        let db = state.path().join("state.sqlite3");
        let settings = state.path().join("settings.json");
        let default = state.path().join("legacy-default-vault");
        NativeApplication::isolated(&default, &db).unwrap();

        let restored = resolve_vault(default.clone(), &db, &settings, None).unwrap();

        assert!(!restored.setup_required);
        assert!(restored.persist_path);
        assert!(!restored.intro_required);
        assert_eq!(restored.path, default);
    }

    #[test]
    fn legacy_sqlite_settings_migrate_once_to_the_home_settings_file() {
        let state = tempdir().unwrap();
        let db = state.path().join("legacy.sqlite3");
        let settings = state.path().join(".llm-workbench/settings.json");
        let vault = state.path().join("vault");
        std::fs::create_dir(&vault).unwrap();
        let connection = rusqlite::Connection::open(&db).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE app_settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
                 CREATE TABLE locale_settings(id INTEGER PRIMARY KEY,locale TEXT NOT NULL,explicit INTEGER NOT NULL);
                 CREATE TABLE provider_settings(
                   id INTEGER PRIMARY KEY,base_url TEXT NOT NULL,model TEXT NOT NULL,
                   advanced_model TEXT NOT NULL,advanced_tasks TEXT NOT NULL,
                   report_language TEXT NOT NULL,async_worker_count INTEGER NOT NULL);",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO app_settings(key,value) VALUES ('vault_path',?)",
                [vault.to_string_lossy().as_ref()],
            )
            .unwrap();
        connection
            .execute("INSERT INTO locale_settings VALUES (1,'ko',1)", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO provider_settings VALUES (1,'https://example.test/v1','small','large','{\"problem_drafting\":true}','en',4)",
                [],
            )
            .unwrap();
        drop(connection);

        native::settings::migrate_legacy(&db, &settings).unwrap();

        assert!(matches!(
            native::settings::vault_startup(&settings).unwrap(),
            VaultStartup::Configured(path) if path == vault
        ));
        assert_eq!(
            native::settings::locale(&settings, "en").unwrap()["locale"],
            "ko"
        );
        let provider = native::settings::provider(&settings).unwrap();
        assert_eq!(provider["model"], "small");
        assert_eq!(provider["advanced_model"], "large");
        assert_eq!(provider["async_worker_count"], 4);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&settings).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                std::fs::metadata(settings.parent().unwrap())
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o700
            );
        }

        let original = std::fs::read_to_string(&settings).unwrap();
        native::settings::migrate_legacy(&db, &settings).unwrap();
        assert_eq!(std::fs::read_to_string(&settings).unwrap(), original);
    }

    #[test]
    fn old_provider_schema_is_upgraded_before_settings_migration() {
        let state = tempdir().unwrap();
        let db = state.path().join("legacy.sqlite3");
        let settings = state.path().join(".llm-workbench/settings.json");
        let connection = rusqlite::Connection::open(&db).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE provider_settings(
                   id INTEGER PRIMARY KEY,base_url TEXT NOT NULL,model TEXT NOT NULL);
                 INSERT INTO provider_settings VALUES (1,'https://example.test/v1','legacy-model');",
            )
            .unwrap();
        drop(connection);

        native::database::initialize(&db).unwrap();
        native::settings::migrate_legacy(&db, &settings).unwrap();

        let provider = native::settings::provider(&settings).unwrap();
        assert_eq!(provider["base_url"], "https://example.test/v1");
        assert_eq!(provider["model"], "legacy-model");
        assert_eq!(provider["advanced_model"], "");
        assert_eq!(provider["advanced_tasks"]["problem_drafting"], true);
        assert_eq!(provider["advanced_tasks"]["workbench_organization"], false);
        assert_eq!(provider["report_language"], "ko");
        assert_eq!(provider["async_worker_count"], 2);
    }

    #[test]
    fn native_runtime_uses_sqlite_without_a_loopback_origin() {
        let state = tempdir().unwrap();
        let app = NativeApplication::isolated(
            &state.path().join("vault"),
            &state.path().join("db.sqlite"),
        )
        .unwrap();
        let created = app.execute(NativeOperation {
            name: "capture.create".into(),
            input: json!({"text":"Native state"}),
        });
        let board = app.execute(NativeOperation {
            name: "board.get".into(),
            input: json!({}),
        });
        assert_eq!(created.status, 201);
        assert_eq!(board.status, 200);
        assert!(board.body.to_string().contains("Native state"));
    }

    #[test]
    fn native_board_preserves_bilingual_versions_and_legacy_fallback() {
        let state = tempdir().unwrap();
        let app = NativeApplication::isolated(
            &state.path().join("vault"),
            &state.path().join("db.sqlite"),
        )
        .unwrap();
        let capture = app.execute(NativeOperation {
            name: "capture.create".into(),
            input: json!({"text":"원문 캡처"}),
        });
        let problem = app.execute(NativeOperation {
            name: "capture.promote".into(),
            input: json!({
                "captureId": capture.body["id"],
                "statement": "한글 문제",
                "detail": "한글 맥락",
                "localized_versions": {
                    "ko": {"statement":"한글 문제","detail":"한글 맥락"},
                    "en": {"statement":"English problem","detail":"English context"}
                }
            }),
        });
        assert_eq!(problem.status, 201, "{}", problem.body);

        let english = app.execute(NativeOperation {
            name: "board.get".into(),
            input: json!({"locale":"en-US"}),
        });
        assert_eq!(english.body["problems"][0]["statement"], "English problem");
        assert_eq!(english.body["problems"][0]["fallback_used"], false);
        assert_eq!(
            english.body["problems"][0]["available_locales"],
            json!(["ko", "en"])
        );

        let supplemented = app.execute(NativeOperation {
            name: "item.localization.save".into(),
            input: json!({
                "entityType":"problems",
                "entityId":problem.body["id"],
                "locale":"en",
                "fields":{"statement":"Updated English"}
            }),
        });
        assert_eq!(supplemented.status, 204, "{}", supplemented.body);
        let english = app.execute(NativeOperation {
            name: "board.get".into(),
            input: json!({"locale":"en"}),
        });
        assert_eq!(english.body["problems"][0]["statement"], "Updated English");
        assert_eq!(english.body["problems"][0]["detail"], "English context");
    }

    #[test]
    fn bundled_model_drives_native_semantic_search_offline() {
        let state = tempdir().unwrap();
        let vault = state.path().join("vault");
        std::fs::create_dir_all(&vault).unwrap();
        std::fs::write(
            vault.join("korean.md"),
            "# 배포 안내\n\n네이티브 앱은 인터넷 연결 없이 문서를 검색합니다.",
        )
        .unwrap();
        std::fs::write(
            vault.join("cooking.md"),
            "# Dinner\n\nRoast vegetables in the oven.",
        )
        .unwrap();
        let model = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("embedding-model");
        let app =
            NativeApplication::isolated_with_model(&vault, &state.path().join("db.sqlite"), &model)
                .unwrap();
        let indexed = app.execute(NativeOperation {
            name: "vault.index".into(),
            input: json!({}),
        });
        assert_eq!(indexed.status, 200, "{}", indexed.body);
        let health = app.execute(NativeOperation {
            name: "health.get".into(),
            input: json!({}),
        });
        assert_eq!(health.body["semantic_available"], true);
        assert_eq!(health.body["semantic_documents"], 2);
        let search = app.execute(NativeOperation {
            name: "vault.search".into(),
            input: json!({"query":"인터넷", "semantic":true}),
        });
        assert_eq!(search.status, 200, "{}", search.body);
        assert_eq!(search.body["semantic_available"], true);
        assert!(search.body["results"][0]["semantic_score"].is_number());
    }
}
