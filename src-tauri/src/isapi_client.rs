use anyhow::{bail, Result};
use md5::{Digest, Md5};
use reqwest::Response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct IsapiClient {
    pub base_url: String,
    pub username: String,
    pub password: String,
    client: reqwest::Client,
}

// ========== Digest Auth ==========

struct DigestAuthParams {
    realm: String,
    nonce: String,
    qop: Option<String>,
    opaque: Option<String>,
}

impl DigestAuthParams {
    fn parse(header: &str) -> Option<Self> {
        let header = header.trim();
        if !header.to_lowercase().starts_with("digest ") {
            return None;
        }

        let params_str = &header[7..];
        let mut realm = String::new();
        let mut nonce = String::new();
        let mut qop = None;
        let mut opaque = None;

        for part in params_str.split(',') {
            let part = part.trim();
            if let Some((key, value)) = part.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');
                match key {
                    "realm" => realm = value.to_string(),
                    "nonce" => nonce = value.to_string(),
                    "qop" => qop = Some(value.to_string()),
                    "opaque" => opaque = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        if realm.is_empty() || nonce.is_empty() {
            return None;
        }

        Some(Self {
            realm,
            nonce,
            qop,
            opaque,
        })
    }

    fn to_header(&self, username: &str, password: &str, method: &str, uri: &str) -> String {
        let nc = "00000001";
        let cnonce = uuid::Uuid::new_v4().to_string().replace('-', "")[..16].to_string();

        let ha1 = format!("{:x}", {
            let mut hasher = Md5::new();
            hasher.update(format!("{}:{}:{}", username, self.realm, password).as_bytes());
            hasher.finalize()
        });

        let ha2 = format!("{:x}", {
            let mut hasher = Md5::new();
            hasher.update(format!("{}:{}", method, uri).as_bytes());
            hasher.finalize()
        });

        let response = if let Some(ref qop) = self.qop {
            if qop.contains("auth") {
                format!("{:x}", {
                    let mut hasher = Md5::new();
                    hasher.update(format!("{}:{}:{}:{}:auth:{}", ha1, self.nonce, nc, cnonce, ha2).as_bytes());
                    hasher.finalize()
                })
            } else {
                format!("{:x}", {
                    let mut hasher = Md5::new();
                    hasher.update(format!("{}:{}:{}", ha1, self.nonce, ha2).as_bytes());
                    hasher.finalize()
                })
            }
        } else {
            format!("{:x}", {
                let mut hasher = Md5::new();
                hasher.update(format!("{}:{}:{}", ha1, self.nonce, ha2).as_bytes());
                hasher.finalize()
            })
        };

        let mut auth = format!(
            "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\"",
            username, self.realm, self.nonce, uri
        );

        if let Some(ref qop) = self.qop {
            auth.push_str(&format!(", qop={}", qop));
        }

        auth.push_str(&format!(", nc={}", nc));
        auth.push_str(&format!(", cnonce=\"{}\"", cnonce));
        auth.push_str(&format!(", response=\"{}\"", response));

        if let Some(ref opaque) = self.opaque {
            if !opaque.is_empty() {
                auth.push_str(&format!(", opaque=\"{}\"", opaque));
            }
        }

        auth
    }
}

// ========== 数据结构 ==========

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct DeviceInfo {
    pub device_name: Option<String>,
    pub device_id: Option<String>,
    pub model: Option<String>,
    pub serial_number: Option<String>,
    pub mac_address: Option<String>,
    pub firmware_version: Option<String>,
    pub firmware_released_date: Option<String>,
    pub device_type: Option<String>,
    pub sub_device_type: Option<String>,
}

/// 有效期配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidPeriod {
    pub enable: bool,
    pub begin_time: Option<String>,
    pub end_time: Option<String>,
    pub time_type: Option<String>,
}

/// 门权限计划
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RightPlan {
    pub door_no: Option<i32>,
    pub plan_template_no: Option<String>,
}

/// 人员信息（与设备实际返回一致）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub employee_no: String,
    pub name: Option<String>,
    pub user_type: Option<String>,
    pub gender: Option<String>,
    pub only_verify: Option<bool>,
    pub close_delay_enabled: Option<bool>,
    pub valid: Option<ValidPeriod>,
    pub belong_group: Option<String>,
    pub password: Option<String>,
    pub door_right: Option<String>,
    #[serde(default)]
    pub right_plan: Vec<RightPlan>,
    pub max_open_door_time: Option<i32>,
    pub open_door_time: Option<i32>,
    pub room_number: Option<i32>,
    pub floor_number: Option<i32>,
    pub local_ui_right: Option<bool>,
    pub linkage_user_id: Option<i32>,
    pub num_of_card: Option<i32>,
    pub num_of_remote_control: Option<i32>,
    pub num_of_face: Option<i32>,
    pub face_url: Option<String>,
    #[serde(default)]
    pub person_info_extends: Vec<PersonInfoExtend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonInfoExtend {
    pub value: Option<String>,
}

/// 人员查询结果（外层包装 UserInfoSearch）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfoSearchResponse {
    #[serde(rename = "UserInfoSearch")]
    pub result: UserInfoSearchResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfoSearchResult {
    #[serde(alias = "searchID")]
    pub search_id: String,
    pub response_status_strg: String,
    pub num_of_matches: i32,
    pub total_matches: i32,
    #[serde(default, alias = "UserInfo")]
    pub user_info: Vec<UserInfo>,
}

/// 同步结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub device_id: String,
    pub device_name: String,
    pub success: bool,
    pub total: i32,
    pub message: String,
}

// ========== ISAPI 客户端实现 ==========

impl IsapiClient {
    pub fn new(ip: &str, port: u16, username: &str, password: &str) -> Self {
        let base_url = if port == 80 {
            format!("http://{}", ip)
        } else {
            format!("http://{}:{}", ip, port)
        };

        Self {
            base_url,
            username: username.to_string(),
            password: password.to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    /// 发送请求，使用 Digest Auth
    async fn request_with_auth(
        &self,
        method: reqwest::Method,
        url: &str,
        body: Option<String>,
    ) -> Result<Response> {
        // 第一步：发送无认证请求，获取 Digest 质询
        let mut req = match method {
            reqwest::Method::GET => self.client.get(url),
            reqwest::Method::POST => self.client.post(url),
            reqwest::Method::PUT => self.client.put(url),
            reqwest::Method::DELETE => self.client.delete(url),
            _ => self.client.get(url),
        };

        if let Some(ref b) = body {
            req = req.header("Content-Type", "application/json").body(b.clone());
        }

        let response = req.send().await?;

        // 第二步：解析 Digest 质询并携带认证信息重试
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            if let Some(www_auth) = response.headers().get("www-authenticate") {
                let auth_str = www_auth.to_str().unwrap_or("");
                if auth_str.to_lowercase().starts_with("digest") {
                    if let Some(params) = DigestAuthParams::parse(auth_str) {
                        let method_str = method.as_str();
                        let uri = Self::extract_uri(url);
                        let auth_header =
                            params.to_header(&self.username, &self.password, method_str, &uri);

                        let mut retry_req = match method {
                            reqwest::Method::GET => self.client.get(url),
                            reqwest::Method::POST => self.client.post(url),
                            reqwest::Method::PUT => self.client.put(url),
                            reqwest::Method::DELETE => self.client.delete(url),
                            _ => self.client.get(url),
                        };

                        retry_req = retry_req.header("Authorization", auth_header);

                        if let Some(b) = body {
                            retry_req = retry_req
                                .header("Content-Type", "application/json")
                                .body(b);
                        }

                        return retry_req
                            .send()
                            .await
                            .map_err(|e| anyhow::anyhow!("请求失败: {}", e));
                    }
                }
            }
            bail!("认证失败: 401 Unauthorized");
        }

        Ok(response)
    }

    /// 从完整 URL 中提取路径部分（用于 Digest Auth 的 uri 字段）
    fn extract_uri(url: &str) -> String {
        if let Some(protocol_end) = url.find("//") {
            let after_protocol = &url[protocol_end + 2..];
            if let Some(path_start) = after_protocol.find('/') {
                return after_protocol[path_start..].to_string();
            }
        }
        "/".to_string()
    }

    /// 测试设备连通性
    pub async fn test_connection(&self) -> Result<bool> {
        let url = format!("{}/ISAPI/System/deviceInfo", self.base_url);
        let response = self
            .request_with_auth(reqwest::Method::GET, &url, None)
            .await?;
        Ok(response.status().is_success())
    }

    /// 获取设备信息
    #[allow(dead_code)]
    pub async fn get_device_info(&self) -> Result<DeviceInfo> {
        let url = format!("{}/ISAPI/System/deviceInfo", self.base_url);
        let response = self
            .request_with_auth(reqwest::Method::GET, &url, None)
            .await?;

        if !response.status().is_success() {
            bail!("获取设备信息失败: HTTP {}", response.status());
        }

        let info: DeviceInfo = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("解析设备信息失败: {}", e))?;
        Ok(info)
    }

    /// 获取所有人员（自动翻页）
    pub async fn get_all_personnel(&self) -> Result<Vec<UserInfo>> {
        let mut all_personnel = Vec::new();
        let mut position = 0;
        let max_results = 50;
        let search_id = uuid::Uuid::new_v4().to_string();

        loop {
            let url = format!(
                "{}/ISAPI/AccessControl/UserInfo/Search?format=json",
                self.base_url
            );

            let body = serde_json::json!({
                "UserInfoSearchCond": {
                    "searchID": search_id,
                    "searchResultPosition": position,
                    "maxResults": max_results
                }
            });

            let response = self
                .request_with_auth(reqwest::Method::POST, &url, Some(body.to_string()))
                .await?;

            if !response.status().is_success() {
                bail!("查询人员失败: HTTP {}", response.status());
            }

            let resp_text = response.text().await?;

            // 外层包装 {"UserInfoSearch": {...}}
            let wrapper: UserInfoSearchResponse = serde_json::from_str(&resp_text)
                .map_err(|e| anyhow::anyhow!("解析人员数据失败: {} | 原始数据: {}", e, &resp_text[..200.min(resp_text.len())]))?;

            let total = wrapper.result.total_matches;
            let status = wrapper.result.response_status_strg.clone();

            all_personnel.extend(wrapper.result.user_info);

            // responseStatusStrg == "OK" 表示查询结束
            if status == "OK" || all_personnel.len() >= total as usize {
                break;
            }

            position += max_results;
        }

        Ok(all_personnel)
    }

    /// 配置设备 HTTP 推送（用于考勤事件上报）
    pub async fn configure_http_push(
        &self,
        server_url: &str,
        server_ip: &str,
        server_port: u16,
    ) -> Result<()> {
        // 先查看当前配置
        let get_url = format!("{}/ISAPI/Event/notification/httpHosts", self.base_url);
        let _ = self
            .request_with_auth(reqwest::Method::GET, &get_url, None)
            .await;

        // 配置推送参数（XML 格式，海康设备要求）
        // 只推送人脸和指纹匹配事件
        let xml_body = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<HttpHostNotificationList version="2.0" xmlns="http://www.isapi.org/ver20/XMLSchema">
<HttpHostNotification>
<id>1</id>
<url>{server_url}</url>
<protocolType>HTTP</protocolType>
<parameterFormatType>JSON</parameterFormatType>
<addressingFormatType>ipaddress</addressingFormatType>
<ipAddress>{server_ip}</ipAddress>
<portNo>{server_port}</portNo>
<httpAuthenticationMethod>none</httpAuthenticationMethod>
<eventType>AccessControllerEvent</eventType>
<eventMode>active</eventMode>
<AccessControllerEvent>
<eventType>faceRecognition</eventType>
</AccessControllerEvent>
<AccessControllerEvent>
<eventType>fingerprintMatch</eventType>
</AccessControllerEvent>
</HttpHostNotification>
</HttpHostNotificationList>"#
        );

        let put_url = format!("{}/ISAPI/Event/notification/httpHosts", self.base_url);
        let response = self
            .request_with_auth(reqwest::Method::PUT, &put_url, Some(xml_body))
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("配置推送失败: HTTP {} | {}", status, body);
        }

        Ok(())
    }
}
