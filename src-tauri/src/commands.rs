use crate::config::{ActionConfig, AppConfig, ConfigManager};
use crate::executor::{self, ActionResult};
use crate::isapi_client::SyncResult;
use crate::license::{License, LicenseManager};
use crate::logger::Logger;
use crate::scheduler::Scheduler;
use crate::sync_manager::SyncManager;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};

pub struct AppState {
    pub config_manager: ConfigManager,
    pub scheduler: Arc<tokio::sync::Mutex<Scheduler>>,
    pub logger: Arc<Logger>,
    pub license_manager: LicenseManager,
    pub sync_manager: Arc<tokio::sync::Mutex<SyncManager>>,
}

#[tauri::command]
pub fn load_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let config = state
        .config_manager
        .load()
        .map_err(|e| format!("加载配置失败: {}", e))?;

    let logger = state.logger.clone();
    let schedules = config.schedules.clone();
    let scheduler = state.scheduler.clone();

    tauri::async_runtime::spawn(async move {
        let mut scheduler = scheduler.lock().await;
        scheduler.start_all(&schedules, logger);
    });

    Ok(config)
}

#[tauri::command]
pub fn import_config(state: State<'_, AppState>, path: String) -> Result<AppConfig, String> {
    let config = state
        .config_manager
        .import(&PathBuf::from(&path))
        .map_err(|e| format!("导入配置失败: {}", e))?;

    let logger = state.logger.clone();
    let schedules = config.schedules.clone();
    let scheduler = state.scheduler.clone();

    tauri::async_runtime::spawn(async move {
        let mut scheduler = scheduler.lock().await;
        scheduler.start_all(&schedules, logger);
    });

    Ok(config)
}

#[tauri::command]
pub async fn execute_action(action: ActionConfig) -> Result<ActionResult, String> {
    if let ActionConfig::Script(ref script_action) = action {
        return Ok(ActionResult::success_with_data(
            "脚本需在前端执行",
            serde_json::json!({ "script": script_action.script }),
        ));
    }
    Ok(executor::execute_action(&action).await)
}

#[tauri::command]
pub fn check_license(state: State<'_, AppState>) -> bool {
    state.license_manager.has_license()
}

#[tauri::command]
pub async fn verify_license(
    state: State<'_, AppState>,
    code: String,
) -> Result<License, String> {
    // 请求服务器验证
    let client = reqwest::Client::new();
    let resp = client
        .post("http://localhost:3000/api/license/verify")
        .json(&serde_json::json!({ "code": code }))
        .send()
        .await
        .map_err(|e| format!("网络请求失败: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("验证失败: HTTP {}", resp.status()));
    }

    let _body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("解析响应失败: {}", e))?;

    // 服务器返回验证成功，本地生成 license
    let issued_at = chrono::Utc::now().to_rfc3339();
    let signature = LicenseManager::compute_signature(&code, &issued_at);

    let license = License {
        code,
        issued_at,
        signature,
    };

    state
        .license_manager
        .save(&license)
        .map_err(|e| format!("保存 license 失败: {}", e))?;

    state.logger.info("系统", "License 验证成功");
    Ok(license)
}

#[tauri::command]
pub async fn toggle_scheduler(state: State<'_, AppState>, enabled: bool) -> Result<bool, String> {
    let scheduler = state.scheduler.lock().await;
    scheduler.set_enabled(enabled);
    state
        .logger
        .info("调度器", format!("定时任务已{}", if enabled { "开启" } else { "关闭" }));
    Ok(enabled)
}

#[tauri::command]
pub async fn get_scheduler_state(state: State<'_, AppState>) -> Result<bool, String> {
    let scheduler = state.scheduler.lock().await;
    Ok(scheduler.is_enabled())
}

#[tauri::command]
pub fn get_license_code(state: State<'_, AppState>) -> Option<String> {
    state.license_manager.load().map(|l| l.code)
}

#[tauri::command]
pub fn logout_license(state: State<'_, AppState>) -> Result<(), String> {
    state.license_manager.delete().map_err(|e| format!("{}", e))?;
    state.logger.info("系统", "License 已注销");
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
pub async fn sync_personnel(
    state: State<'_, AppState>,
    device_id: Option<String>,
) -> Result<Vec<SyncResult>, String> {
    let sync_manager = state.sync_manager.lock().await;
    let results = sync_manager.sync_personnel(device_id.as_deref()).await;

    for result in &results {
        if result.success {
            state.logger.info(
                "同步",
                format!("[{}] {}", result.device_name, result.message),
            );
        } else {
            state.logger.error(
                "同步",
                format!("[{}] {}", result.device_name, result.message),
            );
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn configure_push(
    state: State<'_, AppState>,
    device_id: Option<String>,
) -> Result<Vec<SyncResult>, String> {
    let sync_manager = state.sync_manager.lock().await;
    let results = sync_manager.configure_push(device_id.as_deref()).await;

    for result in &results {
        if result.success {
            state.logger.info(
                "推送",
                format!("[{}] {}", result.device_name, result.message),
            );
        } else {
            state.logger.error(
                "推送",
                format!("[{}] {}", result.device_name, result.message),
            );
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn test_device_connection(
    state: State<'_, AppState>,
    device_id: String,
) -> Result<bool, String> {
    let config = state.config_manager.get();
    let device = config
        .devices
        .iter()
        .find(|d| d.id == device_id)
        .ok_or_else(|| format!("设备 {} 未找到", device_id))?;

    let client = crate::isapi_client::IsapiClient::new(
        &device.ip,
        device.port,
        &device.username,
        &device.password,
    );

    client
        .test_connection()
        .await
        .map_err(|e| format!("测试连接失败: {}", e))
}
