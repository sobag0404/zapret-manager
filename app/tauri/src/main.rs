#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod service_client;

use commands::{
    diagnostics::*, disable::*, enable::*, logs::*, profiles::*, recovery::*, settings::*,
    status::*, updates::*,
};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::utils::config::Config;
use tauri::{AppHandle, Manager, RunEvent, WindowEvent};
use zapret_manager_core::RuntimeStatus;

fn main() {
    let mut context = tauri::generate_context!();
    apply_remote_test_cdp_args(context.config_mut());

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            run_messaging_diagnostics,
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
        .build(context)
        .expect("failed to build Zapret Manager")
        .run(|_app, event| {
            if matches!(event, RunEvent::ExitRequested { .. }) {
                cleanup_before_forced_exit();
            }
        });
}

fn apply_remote_test_cdp_args(config: &mut Config) {
    let Some(args) = remote_test_cdp_args_from_env() else {
        return;
    };

    for window in &mut config.app.windows {
        window.additional_browser_args = Some(args.clone());
    }
}

fn remote_test_cdp_args_from_env() -> Option<String> {
    let value = std::env::var("ZAPRET_MANAGER_REMOTE_TEST_CDP_PORT").ok()?;
    remote_test_cdp_args(&value)
}

fn remote_test_cdp_args(value: &str) -> Option<String> {
    let port = parse_remote_test_cdp_port(value)?;
    Some(format!(
        "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --remote-debugging-address=127.0.0.1 --remote-debugging-port={port}"
    ))
}

fn parse_remote_test_cdp_port(value: &str) -> Option<u16> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let port = value.parse::<u16>().ok()?;
    if (1024..=65535).contains(&port) {
        Some(port)
    } else {
        None
    }
}

fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Открыть", true, None::<&str>)?;
    let toggle = MenuItem::with_id(app, "toggle", "Включить / Выключить", true, None::<&str>)?;
    let diagnostics = MenuItem::with_id(app, "diagnostics", "Диагностика", true, None::<&str>)?;
    let recovery = MenuItem::with_id(app, "recovery", "Восстановление", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
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
                if cleanup_before_user_exit(app) {
                    app.exit(0);
                } else {
                    show_main_window(app);
                }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExitCleanupDecision {
    Exit,
    KeepRunning,
}

fn exit_cleanup_decision(cleanup_ok: bool) -> ExitCleanupDecision {
    if cleanup_ok {
        ExitCleanupDecision::Exit
    } else {
        ExitCleanupDecision::KeepRunning
    }
}

fn cleanup_before_user_exit(app: &AppHandle) -> bool {
    let cleanup_ok = match service_client::client().lock() {
        Ok(mut guard) => guard.disable_all().is_ok(),
        Err(_) => false,
    };

    match exit_cleanup_decision(cleanup_ok) {
        ExitCleanupDecision::Exit => {
            set_tray_status(app, false);
            true
        }
        ExitCleanupDecision::KeepRunning => {
            show_main_window(app);
            false
        }
    }
}

fn cleanup_before_forced_exit() {
    if let Ok(mut guard) = service_client::client().lock() {
        let _ = guard.disable_all();
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

#[cfg(test)]
mod tests {
    use super::{
        exit_cleanup_decision, parse_remote_test_cdp_port, remote_test_cdp_args,
        ExitCleanupDecision,
    };

    #[test]
    fn exit_cleanup_only_exits_after_successful_cleanup() {
        assert_eq!(exit_cleanup_decision(true), ExitCleanupDecision::Exit);
        assert_eq!(
            exit_cleanup_decision(false),
            ExitCleanupDecision::KeepRunning
        );
    }

    #[test]
    fn remote_test_cdp_args_are_loopback_only_and_env_gated() {
        assert_eq!(parse_remote_test_cdp_port("9223"), Some(9223));
        assert_eq!(parse_remote_test_cdp_port("1023"), None);
        assert_eq!(parse_remote_test_cdp_port("65536"), None);
        assert_eq!(parse_remote_test_cdp_port(" 9223"), None);
        assert_eq!(
            parse_remote_test_cdp_port("9223 --remote-allow-origins=*"),
            None
        );

        let args = remote_test_cdp_args("9223").expect("args");
        assert!(args.contains("--remote-debugging-address=127.0.0.1"));
        assert!(args.contains("--remote-debugging-port=9223"));
        assert!(args.contains("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection"));
        assert!(!args.contains("0.0.0.0"));
    }
}
