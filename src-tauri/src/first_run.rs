use crate::NativeApplication;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

const MAIN_WINDOW_LABEL: &str = "main";
const INTRO_WINDOW_LABEL: &str = "first-run-intro";

pub fn create_intro_window(app: &tauri::App) -> Result<(), String> {
    let main = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or("The main application window is unavailable")?;
    main.center().map_err(|error| error.to_string())?;
    let monitor = main
        .current_monitor()
        .map_err(|error| error.to_string())?
        .or_else(|| app.primary_monitor().ok().flatten())
        .ok_or("The current monitor is unavailable")?;
    let scale = monitor.scale_factor();
    let position = monitor.position();
    let size = monitor.size();

    WebviewWindowBuilder::new(
        app,
        INTRO_WINDOW_LABEL,
        WebviewUrl::App("index.html?surface=first-run-intro".into()),
    )
    .title("LLM Wiki welcome")
    .position(position.x as f64 / scale, position.y as f64 / scale)
    .inner_size(size.width as f64 / scale, size.height as f64 / scale)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .resizable(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .on_page_load(|window, payload| {
        if matches!(payload.event(), tauri::webview::PageLoadEvent::Finished) {
            let _ = window.show();
            let _ = window.set_focus();
        }
    })
    .build()
    .map_err(|error| format!("Could not create the first-run welcome surface: {error}"))?;
    Ok(())
}

pub fn choose_vault(
    app: &tauri::AppHandle,
    application: &NativeApplication,
) -> Result<bool, String> {
    let selected = app
        .dialog()
        .file()
        .set_title("Choose your LLM Wiki Vault")
        .blocking_pick_folder();
    let Some(selected) = selected else {
        return Ok(false);
    };
    let path = selected.into_path().map_err(|error| error.to_string())?;
    application.save_vault_selection(&path)?;
    Ok(true)
}

pub fn complete_intro_and_choose_vault(
    app: &tauri::AppHandle,
    application: &NativeApplication,
) -> Result<bool, String> {
    application.complete_first_run_intro()?;
    hide_intro(app);
    let result = choose_vault(app, application);
    if !result.as_ref().copied().unwrap_or(false) {
        close_intro(app);
    }
    result
}

fn focus_main(app: &tauri::AppHandle) {
    if let Some(main) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = main.show();
        let _ = main.set_focus();
    }
}

fn hide_intro(app: &tauri::AppHandle) {
    if let Some(intro) = app.get_webview_window(INTRO_WINDOW_LABEL) {
        let _ = intro.hide();
    }
    focus_main(app);
}

fn close_intro(app: &tauri::AppHandle) {
    if let Some(intro) = app.get_webview_window(INTRO_WINDOW_LABEL) {
        let _ = intro.close();
    }
    focus_main(app);
}
