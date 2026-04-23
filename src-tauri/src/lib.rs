mod clipper;
mod commands;
mod config;
mod editor;
mod error;
mod models;
mod notifications;
mod vault;
mod watcher;

use std::sync::{Arc, Mutex};

use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use config::{load_config, ConfigState};
use notifications::{NotificationsOffset, NotificationsState};
use std::sync::RwLock;
use watcher::WatcherState;

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
                            .title("Quick Note")
                            .inner_size(480.0, 360.0)
                            .resizable(false)
                            .always_on_top(true)
                            .build();
                        }
                    }
                })
                .build(),
        )
        .setup(move |app| {
            // 설정 로드 (app_data_dir/config.json)
            let app_data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let config = load_config(&app_data_dir);

            let config_state: ConfigState = Arc::new(RwLock::new(config.clone()));
            let watcher_state: WatcherState = Arc::new(Mutex::new(None));
            let notifications_state: NotificationsState =
                Arc::new(Mutex::new(NotificationsOffset::default()));

            app.manage(config_state);
            app.manage(watcher_state.clone());
            app.manage(notifications_state.clone());

            // watcher 시작 (vault_path가 설정된 경우에만)
            if config.vault_path.is_some() {
                match watcher::start_watching(
                    app.handle().clone(),
                    &config,
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

            // 글로벌 단축키 등록
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
            commands::onboarding::get_vault_status,
            commands::onboarding::connect_vault,
            commands::settings::get_config,
            commands::settings::update_config,
            commands::settings::detect_editors,
            commands::vaults::list_vaults,
            commands::vaults::switch_vault,
            commands::vaults::remove_vault,
            commands::transcribe::save_recording,
            commands::transcribe::delete_recording,
            commands::transcribe::list_recordings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
