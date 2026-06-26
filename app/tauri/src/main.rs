#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod service_client;

use commands::{
    diagnostics::*, disable::*, enable::*, logs::*, profiles::*, recovery::*, settings::*,
    status::*, updates::*,
};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};
use zapret_manager_core::RuntimeStatus;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            setup_close_to_tray(app.handle());
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
    let recovery = MenuItem::with_id(app, "recovery", "Восстановление", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Закрыть", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &toggle, &diagnostics, &recovery, &quit])?;
    let _tray = TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().expect("default app icon").clone())
        .menu(&menu)
        .tooltip("Zapret Manager: отключено")
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" | "diagnostics" | "recovery" => show_main_window(app),
            "toggle" => handle_tray_toggle(app),
            "quit" => {
                if let Ok(mut guard) = service_client::client().lock() {
                    let _ = guard.disable_all();
                }
                set_tray_status(app, false);
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn setup_close_to_tray(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let window_for_event = window.clone();
        window.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window_for_event.hide();
            }
        });
    }
}

pub(crate) fn set_tray_status(app: &AppHandle, running: bool) {
    if let Some(tray) = app.tray_by_id("main") {
        let tooltip = if running {
            "Zapret Manager: работает"
        } else {
            "Zapret Manager: отключено"
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

fn handle_tray_toggle(app: &AppHandle) {
    let mut guard = match service_client::client().lock() {
        Ok(guard) => guard,
        Err(_) => return show_main_window(app),
    };
    let status = match guard.status() {
        Ok(status) => status,
        Err(_) => return show_main_window(app),
    };

    if status.status == RuntimeStatus::Running {
        if guard.disable_all().is_ok() {
            set_tray_status(app, false);
        }
        return;
    }

    show_main_window(app);
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
