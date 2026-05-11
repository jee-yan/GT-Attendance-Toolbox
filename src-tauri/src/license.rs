use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const SECRET_SALT: &str = "ds-k1t670m-salt-2026";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub code: String,
    pub issued_at: String,
    pub signature: String,
}

pub struct LicenseManager {
    path: PathBuf,
}

impl LicenseManager {
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        // 按优先级查找：当前目录 → 父目录（项目根） → 可执行文件目录
        let candidates = [
            cwd.join("license.json"),
            cwd.parent().unwrap_or(&cwd).join("license.json"),
        ];

        for path in &candidates {
            if path.exists() {
                return Self { path: path.clone() };
            }
        }

        // 都不存在时默认用当前目录
        Self { path: cwd.join("license.json") }
    }

    pub fn has_license(&self) -> bool {
        self.load().is_some()
    }

    pub fn load(&self) -> Option<License> {
        let content = std::fs::read_to_string(&self.path).ok()?;
        let license: License = serde_json::from_str(&content).ok()?;

        if self.verify_signature(&license) {
            Some(license)
        } else {
            None
        }
    }

    pub fn save(&self, license: &License) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(license)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn delete(&self) -> anyhow::Result<()> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }

    pub fn verify_signature(&self, license: &License) -> bool {
        let expected = Self::compute_signature(&license.code, &license.issued_at);
        license.signature == expected
    }

    pub fn compute_signature(code: &str, issued_at: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(code.as_bytes());
        hasher.update(issued_at.as_bytes());
        hasher.update(SECRET_SALT.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}
