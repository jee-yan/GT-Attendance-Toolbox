use crate::config::{ActionConfig, HttpAction, ScriptAction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActionResult {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl ActionResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    pub fn success_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

pub async fn execute_action(action: &ActionConfig) -> ActionResult {
    match action {
        ActionConfig::Http(http) => execute_http(http).await,
        ActionConfig::Script(script) => execute_script(script).await,
    }
}

async fn execute_http(action: &HttpAction) -> ActionResult {
    let client = reqwest::Client::new();

    let mut req = match action.method.to_uppercase().as_str() {
        "GET" => client.get(&action.url),
        "POST" => client.post(&action.url),
        "PUT" => client.put(&action.url),
        "DELETE" => client.delete(&action.url),
        "PATCH" => client.patch(&action.url),
        _ => return ActionResult::error(format!("不支持的 HTTP 方法: {}", action.method)),
    };

    // 设置 headers
    for (key, value) in &action.headers {
        req = req.header(key.as_str(), value.as_str());
    }

    // 设置 body
    if let Some(body) = &action.body {
        req = req
            .header("content-type", "application/json")
            .body(body.clone());
    }

    match req.send().await {
        Ok(response) => {
            let status = response.status().as_u16();
            match response.text().await {
                Ok(text) => {
                    if status >= 200 && status < 300 {
                        // 尝试解析为 JSON
                        if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                            ActionResult::success_with_data(
                                format!("HTTP {} - 请求成功", status),
                                data,
                            )
                        } else {
                            ActionResult::success(format!("HTTP {} - {}", status, text))
                        }
                    } else {
                        ActionResult::error(format!("HTTP {} - {}", status, text))
                    }
                }
                Err(e) => ActionResult::error(format!("读取响应失败: {}", e)),
            }
        }
        Err(e) => ActionResult::error(format!("请求失败: {}", e)),
    }
}

async fn execute_script(action: &ScriptAction) -> ActionResult {
    // JS 脚本通过前端 WebView 执行，这里返回脚本内容供前端执行
    // 在实际执行中，前端会调用此命令，然后自行在 WebView 中 eval
    ActionResult::success_with_data(
        "脚本已提交执行",
        serde_json::json!({ "script": action.script }),
    )
}
