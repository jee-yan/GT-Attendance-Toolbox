use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppInfo,
    #[serde(default)]
    pub devices: Vec<DeviceConfig>,
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub sync: SyncConfig,
    #[serde(default)]
    pub buttons: Vec<ButtonConfig>,
    #[serde(default)]
    pub schedules: Vec<ScheduleConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfig {
    pub id: String,
    pub name: String,
    pub ip: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_username")]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

fn default_port() -> u16 {
    80
}

fn default_username() -> String {
    "admin".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default)]
    pub base_url: String,
    #[serde(default = "default_upload_personnel_url")]
    pub upload_personnel_url: String,
    /// 设备推送考勤事件的路径（如 /api/attendance/push）
    #[serde(default = "default_push_path")]
    pub push_path: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            upload_personnel_url: default_upload_personnel_url(),
            push_path: default_push_path(),
        }
    }
}

impl ServerConfig {
    /// 从 base_url 解析主机名（域名或 IP）
    pub fn host(&self) -> &str {
        let url = self.base_url.trim_start_matches("http://").trim_start_matches("https://");
        url.split(':').next().unwrap_or("127.0.0.1").split('/').next().unwrap_or("127.0.0.1")
    }

    /// 从 base_url 解析端口号
    pub fn port(&self) -> u16 {
        let url = self.base_url.trim_start_matches("http://").trim_start_matches("https://");
        if let Some(port_str) = url.split(':').nth(1) {
            port_str.split('/').next().unwrap_or("80").parse().unwrap_or(80)
        } else if self.base_url.starts_with("https://") {
            443
        } else {
            80
        }
    }
}

fn default_upload_personnel_url() -> String {
    "/api/personnel/sync".to_string()
}

fn default_push_path() -> String {
    "/api/attendance/push".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    #[serde(default = "default_personnel_cron")]
    pub personnel_cron: String,
    #[serde(default = "default_attendance_cron")]
    pub attendance_cron: String,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            personnel_cron: default_personnel_cron(),
            attendance_cron: default_attendance_cron(),
        }
    }
}

fn default_personnel_cron() -> String {
    "0 0 8 * * *".to_string()
}

fn default_attendance_cron() -> String {
    "0 */30 * * * *".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonConfig {
    pub id: String,
    pub label: String,
    pub action: ActionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleConfig {
    pub id: String,
    pub name: String,
    pub cron: String,
    pub action: ActionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActionConfig {
    #[serde(rename = "http")]
    Http(HttpAction),
    #[serde(rename = "script")]
    Script(ScriptAction),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpAction {
    pub method: String,
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptAction {
    pub script: String,
}

pub struct ConfigManager {
    config: Arc<Mutex<AppConfig>>,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let config_path = Self::default_config_path();
        let config = Self::load_from_path(&config_path).unwrap_or_else(|_| Self::default_config());

        Self {
            config: Arc::new(Mutex::new(config)),
            config_path,
        }
    }

    pub fn default_config_path() -> PathBuf {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // 按优先级查找：当前目录 → 父目录（项目根）
        let candidates = [
            cwd.join("config.yaml"),
            cwd.parent().unwrap_or(&cwd).join("config.yaml"),
        ];

        for path in &candidates {
            if path.exists() {
                return path.clone();
            }
        }

        // 都不存在时默认用当前目录
        cwd.join("config.yaml")
    }

    pub fn load_from_path(path: &Path) -> anyhow::Result<AppConfig> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn load(&self) -> anyhow::Result<AppConfig> {
        match Self::load_from_path(&self.config_path) {
            Ok(config) => {
                let mut current = self.config.lock().unwrap();
                *current = config.clone();
                Ok(config)
            }
            Err(_) => {
                // 文件不存在时返回默认配置
                Ok(self.config.lock().unwrap().clone())
            }
        }
    }

    pub fn import(&self, path: &Path) -> anyhow::Result<AppConfig> {
        let config = Self::load_from_path(path)?;

        // 复制到默认路径
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(path, &self.config_path)?;

        let mut current = self.config.lock().unwrap();
        *current = config.clone();
        Ok(config)
    }

    pub fn get(&self) -> AppConfig {
        self.config.lock().unwrap().clone()
    }

    pub fn _config_path(&self) -> &Path {
        &self.config_path
    }

    fn default_config() -> AppConfig {
        AppConfig {
            app: AppInfo {
                title: "工具箱".to_string(),
            },
            devices: vec![],
            server: ServerConfig::default(),
            sync: SyncConfig::default(),
            buttons: vec![
                ButtonConfig {
                    id: "sync_personnel".to_string(),
                    label: "同步人员".to_string(),
                    action: ActionConfig::Script(ScriptAction {
                        script: "return { success: true, message: '未配置设备' };".to_string(),
                    }),
                },
                ButtonConfig {
                    id: "configure_push".to_string(),
                    label: "配置推送".to_string(),
                    action: ActionConfig::Script(ScriptAction {
                        script: "return { success: true, message: '未配置设备' };".to_string(),
                    }),
                },
            ],
            schedules: vec![],
        }
    }
}
