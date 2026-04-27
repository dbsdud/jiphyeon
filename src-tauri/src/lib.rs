mod clipper;
mod commands;
mod config;
mod editor;
mod error;
mod graphify;
mod models;
mod notifications;
mod project;
mod vault;
mod watcher;
mod workspace;

use std::sync::{Arc, Mutex, RwLock};

use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use commands::projects::migrate_workspace_links;
use config::{load_config, save_config, ConfigState};
use notifications::{NotificationsOffset, NotificationsState};
use watcher::WatcherState;
use workspace::ensure_workspace_dir;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let shortcut = Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::KeyN);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(win) = app.get_webview_window("capture") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        } else {
                            let _ = tauri::WebviewWindowBuilder::new(
                                app,
                                "capture",
                                tauri::WebviewUrl::App("/capture".into()),
                            )
                            .title("Capture")
                            .inner_size(560.0, 460.0)
                            .resizable(false)
                            .always_on_top(true)
                            .build();
                        }
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let mut config = load_config(&app_data_dir);

            // workspace hub 보장 + 기존 등록 best-effort 마이그레이션
            if let Err(e) = ensure_workspace_dir(&config.workspace_path) {
                eprintln!("workspace 디렉토리 준비 실패: {e}");
            } else {
                let changed = migrate_workspace_links(
                    &config.workspace_path.clone(),
                    &mut config.projects,
                );
                if changed {
                    if let Err(e) = save_config(&config, &app_data_dir) {
                        eprintln!("마이그레이션 후 config 저장 실패: {e}");
                    }
                }
            }

            let watcher_state: WatcherState = Arc::new(Mutex::new(None));
            let notifications_state: NotificationsState =
                Arc::new(Mutex::new(NotificationsOffset::default()));

            // workspace 가 준비되어 있으면 hub 전체를 감시
            if config.workspace_path.is_dir() {
                match watcher::start_watching(
                    app.handle().clone(),
                    &config.workspace_path,
                    &config.exclude_dirs,
                    config.watch_debounce_ms,
                    notifications_state.clone(),
                ) {
                    Ok(w) => {
                        if let Ok(mut guard) = watcher_state.lock() {
                            *guard = Some(w);
                        }
                    }
                    Err(e) => {
                        eprintln!("watcher 시작 실패: {e}");
                    }
                }
            }

            let config_state: ConfigState = Arc::new(RwLock::new(config));
            app.manage(config_state);
            app.manage(watcher_state);
            app.manage(notifications_state);

            if let Err(e) = app.global_shortcut().register(shortcut) {
                eprintln!("글로벌 단축키 등록 실패: {e}");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::note::get_note,
            commands::note::open_in_editor,
            commands::note::create_quick_note,
            commands::clipper::clip_url,
            commands::graphify::get_graphify_graph,
            commands::graphify::get_graphify_report,
            commands::graphify::get_graphify_status,
            commands::graphify::get_cross_project_graph,
            commands::graphify::get_pending_graphify,
            commands::projects::list_projects,
            commands::projects::register_project,
            commands::projects::switch_project,
            commands::projects::remove_project,
            commands::projects::get_active_project,
            commands::projects::inspect_project_root,
            commands::projects::list_project_files,
            commands::projects::get_project_folder_tree,
            commands::projects::get_project_explorer_tree,
            commands::settings::get_config,
            commands::settings::update_config,
            commands::settings::detect_editors,
            commands::transcribe::save_recording,
            commands::transcribe::delete_recording,
            commands::transcribe::list_recordings,
            commands::transcribe::open_capture_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
