mod commands;
mod service_client;

use commands::{
    diagnostics::*, disable::*, enable::*, logs::*, profiles::*, recovery::*, settings::*,
    status::*, updates::*,
};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            setup_tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_status,
            toggle_enabled,
            enable_selected_profiles,
            disable_all,
            list_profiles,
            set_profile_enabled,
            run_diagnostics,
            run_dns_check,
            run_service_connectivity_tests,
            read_user_logs,
            export_debug_logs,
            check_strategy_updates,
            apply_strategy_update,
            rollback_strategy_update,
            repair_driver,
            repair_service,
            restart_engine,
            emergency_disable,
            create_snapshot,
            restore_snapshot,
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Zapret Manager");
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Открыть", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Включить / Выключить", true, None::<&str>)?;
    let diagnostics = MenuItem::with_id(app, "diagnostics", "Диагностика", true, None::<&str>)?;
    let recovery = MenuItem::with_id(app, "recovery", "Восстановить", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &toggle, &diagnostics, &recovery, &quit])?;
    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" | "diagnostics" | "recovery" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
