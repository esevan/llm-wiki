mod adapters;
pub mod application;
mod codex_app_server;
mod conversation;
mod desktop_e2e;
mod domain;
mod first_run;
mod gui_owner;
pub mod mcp;
pub mod mcp_ipc;
mod native;
mod ports;
mod provider;

pub use native::{NativeApplication, NativeOperation, NativeResponse};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::Manager;

static E2E_INTRO_RETRY_FAILURE: AtomicBool = AtomicBool::new(true);
static E2E_VAULT_RETRY_FAILURE: AtomicBool = AtomicBool::new(true);

fn e2e_scenario(name: &str) -> bool {
    std::env::var("LLM_WIKI_E2E_SCENARIO").ok().as_deref() == Some(name)
        && std::env::var_os("LLM_WIKI_E2E_RESULT").is_some()
}

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

fn mcp_listener_needs_shutdown(event: &tauri::RunEvent) -> bool {
    matches!(
        event,
        tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
    )
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
    if std::env::var("LLM_WIKI_E2E_SCENARIO")
        .ok()
        .is_some_and(|value| value.starts_with("global-migration-"))
        && std::env::var_os("LLM_WIKI_E2E_RESULT").is_some()
        && !db.exists()
    {
        native::database::initialize(&db)?;
        let connection = native::database::open(&db)?;
        connection
            .pragma_update(None, "user_version", 7)
            .map_err(|error| error.to_string())?;
        connection.execute("INSERT INTO captures(id,text,created_at,source_mode,last_user_activity_at) VALUES('desktop-e2e-capture','migration fixture','now','capture','now')",[]).map_err(|error| error.to_string())?;
        connection.execute("INSERT INTO problems(id,capture_id,statement,state,created_at,current_revision) VALUES('desktop-e2e-problem','desktop-e2e-capture','migration fixture','open','now',1)",[]).map_err(|error| error.to_string())?;
        connection.execute("INSERT INTO features(id,problem_id,title,outcome,state,created_at) VALUES('desktop-e2e-invalid','desktop-e2e-problem','migration fixture','migration fixture','invalid','now')",[]).map_err(|error| error.to_string())?;
    }
    if db.is_file() && native::database::migration_failure(&db)?.is_none() {
        if let Err(error) = native::database::initialize(&db) {
            if native::database::migration_failure(&db)?.is_none() {
                return Err(format!(
                    "Could not migrate the application database: {error}"
                ));
            }
        }
    }
    if native::database::migration_failure(&db)?.is_none() {
        native::settings::migrate_legacy(&db, &settings_path)
            .map_err(|error| format!("Could not import legacy application settings: {error}"))?;
    }
    let forced = std::env::var_os("LLM_WIKI_VAULT").map(PathBuf::from);
    Ok((
        resolve_vault(default_vault, &db, &settings_path, forced)?,
        db,
        settings_path,
    ))
}

pub async fn run_mcp(connection_id: String) -> Result<(), String> {
    mcp_ipc::run_stdio_bridge(connection_id).await
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

fn execution_error(error: String) -> NativeResponse {
    let code = if error.starts_with("Codex CLI is not installed") {
        "missing_executable"
    } else if error.to_ascii_lowercase().contains("authentication")
        || error.to_ascii_lowercase().contains("not logged in")
    {
        "authentication_required"
    } else if error.starts_with("Codex app-server initialization failed") {
        "initialization_failed"
    } else if error.contains(':') {
        error.split(':').next().unwrap_or("execution_unavailable")
    } else {
        "execution_unavailable"
    };
    let status = if code == "invalid_input" {
        400
    } else if code == "ownership_failure" {
        404
    } else if matches!(
        code,
        "active_run_conflict"
            | "operation_conflict"
            | "settings_revision_conflict"
            | "workspace_thread_mismatch"
    ) || code.starts_with("stale_")
    {
        409
    } else if code == "secret_input_unsupported" || code.starts_with("unsupported_") {
        422
    } else {
        503
    };
    NativeResponse {
        status,
        body: serde_json::json!({"error":{"code":code,"message":error}}),
    }
}

#[tauri::command]
async fn task_session_prepare(
    runtime: tauri::State<'_, native::task_execution_runtime::TaskExecutionRuntime>,
    input: serde_json::Value,
    on_event: tauri::ipc::Channel<serde_json::Value>,
) -> Result<NativeResponse, String> {
    let response = match runtime.prepare(&input).await {
        Ok(body) => {
            runtime.subscribe_channel(&input, on_event)?;
            NativeResponse { status: 200, body }
        }
        Err(error) => execution_error(error),
    };
    Ok(response)
}

#[tauri::command]
async fn task_session_execute(
    runtime: tauri::State<'_, native::task_execution_runtime::TaskExecutionRuntime>,
    input: serde_json::Value,
    on_event: tauri::ipc::Channel<serde_json::Value>,
) -> Result<NativeResponse, String> {
    let response = match runtime.execute(&input).await {
        Ok(body) => {
            runtime.subscribe_channel(&input, on_event)?;
            NativeResponse { status: 200, body }
        }
        Err(error) => execution_error(error),
    };
    Ok(response)
}

#[tauri::command]
fn task_session_execution_subscribe(
    runtime: tauri::State<'_, native::task_execution_runtime::TaskExecutionRuntime>,
    input: serde_json::Value,
    on_event: tauri::ipc::Channel<serde_json::Value>,
) -> Result<NativeResponse, String> {
    match runtime.snapshot(&input) {
        Ok(body) => {
            runtime.subscribe_channel(&input, on_event)?;
            Ok(NativeResponse { status: 200, body })
        }
        Err(error) => Ok(execution_error(error)),
    }
}

#[tauri::command]
async fn task_session_interrupt(
    runtime: tauri::State<'_, native::task_execution_runtime::TaskExecutionRuntime>,
    input: serde_json::Value,
    _on_event: tauri::ipc::Channel<serde_json::Value>,
) -> Result<NativeResponse, String> {
    Ok(match runtime.interrupt(&input).await {
        Ok(body) => NativeResponse { status: 200, body },
        Err(error) => execution_error(error),
    })
}

#[tauri::command]
async fn task_session_formal_response(
    runtime: tauri::State<'_, native::task_execution_runtime::TaskExecutionRuntime>,
    input: serde_json::Value,
    _on_event: tauri::ipc::Channel<serde_json::Value>,
) -> Result<NativeResponse, String> {
    Ok(match runtime.respond(&input).await {
        Ok(body) => NativeResponse { status: 200, body },
        Err(error) => execution_error(error),
    })
}

#[tauri::command]
fn task_session_work_log_sync(
    runtime: tauri::State<'_, native::task_execution_runtime::TaskExecutionRuntime>,
    input: serde_json::Value,
    _on_event: tauri::ipc::Channel<serde_json::Value>,
) -> Result<NativeResponse, String> {
    Ok(match runtime.sync_work_log(&input) {
        Ok(body) => NativeResponse { status: 200, body },
        Err(error) => execution_error(error),
    })
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
fn work_tracking_command(
    application: tauri::State<'_, NativeApplication>,
    window: tauri::WebviewWindow,
    mut operation: NativeOperation,
) -> Result<NativeResponse, String> {
    let context = serde_json::json!({"window":window.label()});
    operation.input["reviewContext"] = context.clone();
    if operation.input.get("proposal").is_some() {
        operation.input["proposal"]["reviewContext"] = context;
    }
    Ok(application.execute_work_tracking(operation))
}

#[tauri::command]
fn vault_setup_status(
    application: tauri::State<'_, NativeApplication>,
) -> Result<serde_json::Value, String> {
    if e2e_scenario("global-vault-retry") && E2E_VAULT_RETRY_FAILURE.swap(false, Ordering::SeqCst) {
        return Err("deterministic desktop E2E Vault status failure".into());
    }
    application.vault_setup_status()
}

#[tauri::command]
fn migration_recovery_status(
    application: tauri::State<'_, NativeApplication>,
) -> Result<serde_json::Value, String> {
    application.migration_recovery_status()
}

#[tauri::command]
fn migration_recovery_restore(
    app: tauri::AppHandle,
    application: tauri::State<'_, NativeApplication>,
    manifest_file: String,
) -> Result<serde_json::Value, String> {
    let result = application.restore_migration_backup(&manifest_file)?;
    if result["recovered"].as_bool().unwrap_or(false) && !e2e_scenario("global-migration-restore") {
        app.restart();
    }
    Ok(result)
}

#[tauri::command]
fn migration_recovery_retry(
    app: tauri::AppHandle,
    application: tauri::State<'_, NativeApplication>,
) -> Result<serde_json::Value, String> {
    let result = application.retry_migration()?;
    if result["recovered"].as_bool().unwrap_or(false) && !e2e_scenario("global-migration-retry") {
        app.restart();
    }
    Ok(result)
}

#[tauri::command]
async fn complete_first_run_intro(
    app: tauri::AppHandle,
    application: tauri::State<'_, NativeApplication>,
) -> Result<bool, String> {
    if e2e_scenario("global-intro-retry") && E2E_INTRO_RETRY_FAILURE.swap(false, Ordering::SeqCst) {
        return Err("deterministic desktop E2E intro completion failure".into());
    }
    if e2e_scenario("global-intro-retry") || e2e_scenario("global-intro-navigation") {
        application.complete_first_run_intro()?;
        return Ok(false);
    }
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
fn choose_project_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    Ok(first_run::pick_folder(&app, "Choose a project folder")?
        .map(|path| path.to_string_lossy().into_owned()))
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
            app.manage(gui_owner::GuiOwnerLock::acquire(&db)?);
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
            let recovery_pending = application
                .migration_recovery_status()?
                .get("recovery")
                .is_some_and(|recovery| !recovery.is_null());
            let background_index = application.clone();
            let execution_service =
                application::task_execution_service::TaskExecutionApplicationService::new(
                    application.db_path(),
                );
            if !recovery_pending {
                execution_service.recover_nonterminal()?;
            }
            let execution_runtime =
                native::task_execution_runtime::TaskExecutionRuntime::new(execution_service);
            let background_projector = application.work_tracking_service();
            let mcp_service = application.work_tracking_service();
            let mcp_listener_shutdown = mcp_ipc::McpListenerShutdown::default();
            let should_index = !setup_required && !recovery_pending;
            app.manage(application);
            app.manage(execution_runtime);
            app.manage(mcp_listener_shutdown.clone());
            if !recovery_pending {
                tauri::async_runtime::spawn(native::work_tracking_projector::run(
                    background_projector,
                ));
                tauri::async_runtime::spawn(async move {
                    if let Err(error) =
                        mcp_ipc::run_gui_listener(mcp_service, mcp_listener_shutdown).await
                    {
                        eprintln!("MCP local IPC listener stopped: {error}");
                    }
                });
            }
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
            work_tracking_command,
            enqueue_ai_job,
            vault_setup_status,
            migration_recovery_status,
            migration_recovery_restore,
            migration_recovery_retry,
            complete_first_run_intro,
            choose_vault,
            choose_project_folder,
            task_session_prepare,
            task_session_execute,
            task_session_execution_subscribe,
            task_session_interrupt,
            task_session_formal_response,
            task_session_work_log_sync,
            conversation::conversation_stream,
            conversation::cancel_conversation,
            desktop_e2e::desktop_e2e_mode,
            desktop_e2e::desktop_e2e_complete,
            desktop_e2e::desktop_e2e_mcp_probe,
            desktop_e2e::desktop_e2e_seed_legacy_refinement,
            desktop_e2e::desktop_e2e_arm_one_shot_failure,
            desktop_e2e::desktop_e2e_seed_completed_tracking,
            desktop_e2e::desktop_e2e_seed_queue_notifications,
            desktop_e2e::desktop_e2e_provider_requests,
            desktop_e2e::desktop_e2e_resize_window,
            provider::provider_request
        ])
        .build(tauri::generate_context!())
        .expect("error while building LLM Wiki desktop")
        .run(|app, event| {
            if mcp_listener_needs_shutdown(&event) {
                app.state::<mcp_ipc::McpListenerShutdown>().shutdown();
                let execution = app
                    .state::<native::task_execution_runtime::TaskExecutionRuntime>()
                    .inner()
                    .clone();
                tauri::async_runtime::spawn(async move {
                    execution.shutdown().await;
                });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::settings::VaultStartup;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn mcp_listener_shutdown_covers_all_tauri_exit_events() {
        assert!(mcp_listener_needs_shutdown(&tauri::RunEvent::Exit));
    }

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
            input: json!({"operationId":"native-state","text":"Native state"}),
        });
        let board = app.execute(NativeOperation {
            name: "workbench.get".into(),
            input: json!({}),
        });
        assert_eq!(created.status, 201);
        assert_eq!(board.status, 200);
        assert!(board.body.to_string().contains("Native state"));
    }

    #[test]
    fn task_workbench_keeps_capture_canonical() {
        let state = tempdir().unwrap();
        let app = NativeApplication::isolated(
            &state.path().join("vault"),
            &state.path().join("db.sqlite"),
        )
        .unwrap();
        let capture = app.execute(NativeOperation {
            name: "capture.create".into(),
            input: json!({"operationId":"capture-ko","text":"원문 캡처"}),
        });
        assert_eq!(capture.status, 201, "{}", capture.body);
        let workbench = app.execute(NativeOperation {
            name: "workbench.get".into(),
            input: json!({}),
        });
        assert_eq!(workbench.status, 200, "{}", workbench.body);
        assert!(workbench.body.to_string().contains("원문 캡처"));
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
