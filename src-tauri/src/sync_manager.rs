use serde::{Deserialize, Serialize};

use crate::config::{DeviceConfig, ServerConfig};
use crate::isapi_client::{IsapiClient, SyncResult, UserInfo};
use crate::logger::Logger;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonnelSyncRequest {
    pub device_id: String,
    pub device_ip: String,
    pub personnel: Vec<UserInfo>,
}

pub struct SyncManager {
    devices: Vec<DeviceConfig>,
    server_config: ServerConfig,
    #[allow(dead_code)]
    logger: Arc<Logger>,
}

impl SyncManager {
    pub fn new(devices: Vec<DeviceConfig>, server_config: ServerConfig, logger: Arc<Logger>) -> Self {
        Self {
            devices,
            server_config,
            logger,
        }
    }

    fn create_client(&self, device: &DeviceConfig) -> IsapiClient {
        IsapiClient::new(&device.ip, device.port, &device.username, &device.password)
    }

    pub async fn sync_personnel(&self, device_id: Option<&str>) -> Vec<SyncResult> {
        let mut results = Vec::new();

        let devices = if let Some(id) = device_id {
            self.devices.iter().filter(|d| d.id == id).collect::<Vec<_>>()
        } else {
            self.devices.iter().collect::<Vec<_>>()
        };

        for device in devices {
            let result = self.sync_device_personnel(device).await;
            results.push(result);
        }

        results
    }

    async fn sync_device_personnel(&self, device: &DeviceConfig) -> SyncResult {
        let client = self.create_client(device);

        match client.test_connection().await {
            Ok(false) => {
                return SyncResult {
                    device_id: device.id.clone(),
                    device_name: device.name.clone(),
                    success: false,
                    total: 0,
                    message: "设备不可达".to_string(),
                };
            }
            Err(e) => {
                return SyncResult {
                    device_id: device.id.clone(),
                    device_name: device.name.clone(),
                    success: false,
                    total: 0,
                    message: format!("连接失败: {}", e),
                };
            }
            _ => {}
        }

        let personnel = match client.get_all_personnel().await {
            Ok(p) => p,
            Err(e) => {
                return SyncResult {
                    device_id: device.id.clone(),
                    device_name: device.name.clone(),
                    success: false,
                    total: 0,
                    message: format!("获取人员失败: {}", e),
                };
            }
        };

        let total = personnel.len() as i32;

        if self.server_config.base_url.is_empty() {
            return SyncResult {
                device_id: device.id.clone(),
                device_name: device.name.clone(),
                success: true,
                total,
                message: format!("获取到 {} 条人员记录（未配置服务器地址）", total),
            };
        }

        let url = format!(
            "{}{}",
            self.server_config.base_url, self.server_config.upload_personnel_url
        );

        let request = PersonnelSyncRequest {
            device_id: device.id.clone(),
            device_ip: device.ip.clone(),
            personnel,
        };

        let client_http = reqwest::Client::new();
        match client_http.post(&url).json(&request).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    SyncResult {
                        device_id: device.id.clone(),
                        device_name: device.name.clone(),
                        success: true,
                        total,
                        message: format!("同步成功: {} 条记录", total),
                    }
                } else {
                    SyncResult {
                        device_id: device.id.clone(),
                        device_name: device.name.clone(),
                        success: false,
                        total,
                        message: format!("上传失败: HTTP {}", resp.status()),
                    }
                }
            }
            Err(e) => SyncResult {
                device_id: device.id.clone(),
                device_name: device.name.clone(),
                success: false,
                total,
                message: format!("上传失败: {}", e),
            },
        }
    }

    /// 配置所有设备的 HTTP 推送（考勤事件直接推送到后端服务器）
    pub async fn configure_push(&self, device_id: Option<&str>) -> Vec<SyncResult> {
        let mut results = Vec::new();

        let devices = if let Some(id) = device_id {
            self.devices.iter().filter(|d| d.id == id).collect::<Vec<_>>()
        } else {
            self.devices.iter().collect::<Vec<_>>()
        };

        if self.server_config.base_url.is_empty() || self.server_config.push_path.is_empty() {
            results.push(SyncResult {
                device_id: "all".to_string(),
                device_name: "所有设备".to_string(),
                success: false,
                total: 0,
                message: "未配置服务器地址 (server.base_url)".to_string(),
            });
            return results;
        }

        for device in devices {
            let result = self.configure_device_push(device).await;
            results.push(result);
        }

        results
    }

    async fn configure_device_push(&self, device: &DeviceConfig) -> SyncResult {
        let client = self.create_client(device);

        match client.test_connection().await {
            Ok(false) => {
                return SyncResult {
                    device_id: device.id.clone(),
                    device_name: device.name.clone(),
                    success: false,
                    total: 0,
                    message: "设备不可达".to_string(),
                };
            }
            Err(e) => {
                return SyncResult {
                    device_id: device.id.clone(),
                    device_name: device.name.clone(),
                    success: false,
                    total: 0,
                    message: format!("连接失败: {}", e),
                };
            }
            _ => {}
        }

        let push_path = &self.server_config.push_path;
        let server_host = self.server_config.host().to_string();
        let server_port = self.server_config.port();

        match client.configure_http_push(push_path, &server_host, server_port).await {
            Ok(()) => SyncResult {
                device_id: device.id.clone(),
                device_name: device.name.clone(),
                success: true,
                total: 0,
                message: format!("推送配置成功 -> {}:{}{}", server_host, server_port, push_path),
            },
            Err(e) => SyncResult {
                device_id: device.id.clone(),
                device_name: device.name.clone(),
                success: false,
                total: 0,
                message: format!("推送配置失败: {}", e),
            },
        }
    }
}
