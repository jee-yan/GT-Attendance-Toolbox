// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod config;
mod executor;
mod isapi_client;
mod license;
mod logger;
mod scheduler;
mod sync_manager;

use commands::AppState;
use config::ConfigManager;
use license::LicenseManager;
use logger::Logger;
use scheduler::Scheduler;
use sync_manager::SyncManager;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WindowEvent,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let logger = Arc::new(Logger::new());
            logger.set_app_handle(app.handle().clone());

            let config_manager = ConfigManager::new();
            let license_manager = LicenseManager::new();

            // 检查 license
            let has_license = license_manager.has_license();

            // 加载配置
            let config = config_manager.load().unwrap_or_else(|e| {
                logger.error("系统", format!("加载配置失败，使用默认配置: {}", e));
                config_manager.get()
            });

            logger.info("系统", "应用已启动");

            let scheduler = Arc::new(tokio::sync::Mutex::new(Scheduler::new()));

            let sync_manager = Arc::new(tokio::sync::Mutex::new(SyncManager::new(
                config.devices.clone(),
                config.server.clone(),
                logger.clone(),
            )));

            app.manage(AppState {
                config_manager,
                scheduler: scheduler.clone(),
                logger: logger.clone(),
                license_manager,
                sync_manager,
            });

            // 启动定时任务（但默认关闭，需要用户手动开启）
            let schedules = config.schedules.clone();
            let logger_clone = logger.clone();
            let scheduler_clone = scheduler.clone();

            tauri::async_runtime::spawn(async move {
                let mut scheduler = scheduler_clone.lock().await;
                scheduler.start_all(&schedules, logger_clone);
            });

            // 如果没有有效 license，发送事件给前端显示弹窗
            if !has_license {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    let _ = handle.emit("license-required", ());
                });
            }

            // 创建系统托盘菜单
            let show_item = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("工具箱 - 右键退出")
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::load_config,
            commands::import_config,
            commands::execute_action,
            commands::check_license,
            commands::verify_license,
            commands::get_license_code,
            commands::logout_license,
            commands::quit_app,
            commands::toggle_scheduler,
            commands::get_scheduler_state,
            commands::sync_personnel,
            commands::configure_push,
            commands::test_device_connection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
