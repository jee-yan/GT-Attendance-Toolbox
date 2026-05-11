use crate::config::ScheduleConfig;
use crate::executor;
use crate::logger::Logger;
use chrono::Utc;
use cron::Schedule;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct Scheduler {
    handles: Vec<JoinHandle<()>>,
    enabled: Arc<AtomicBool>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            handles: vec![],
            enabled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn start_schedule(&mut self, config: &ScheduleConfig, logger: Arc<Logger>) {
        let schedule = match Schedule::from_str(&config.cron) {
            Ok(s) => s,
            Err(e) => {
                logger.error(
                    "调度器",
                    format!("无效的 cron 表达式 '{}': {}", config.cron, e),
                );
                return;
            }
        };

        let name = config.name.clone();
        let id = config.id.clone();
        let action = config.action.clone();
        let logger_clone = logger.clone();
        let enabled = self.enabled.clone();

        logger.info(
            "调度器",
            format!("已注册定时任务: {} ({})", name, config.cron),
        );

        let handle = tokio::spawn(async move {
            let mut upcoming = schedule.upcoming(Utc);
            loop {
                if let Some(next) = upcoming.next() {
                    let now = Utc::now();
                    let duration = (next - now)
                        .to_std()
                        .unwrap_or(std::time::Duration::from_secs(1));
                    tokio::time::sleep(duration).await;

                    // 检查开关状态
                    if !enabled.load(Ordering::Relaxed) {
                        logger_clone.info(
                            "调度器",
                            format!("定时任务 {} 已跳过（开关关闭）", name),
                        );
                        continue;
                    }

                    logger_clone.info(
                        "调度器",
                        format!("触发定时任务: {} [{}]", name, id),
                    );

                    let result = executor::execute_action(&action).await;
                    if result.success {
                        logger_clone.info(&name, result.message);
                    } else {
                        logger_clone.error(&name, result.message);
                    }
                }
            }
        });

        self.handles.push(handle);
    }

    pub fn start_all(&mut self, schedules: &[ScheduleConfig], logger: Arc<Logger>) {
        self.stop_all();
        for config in schedules {
            self.start_schedule(config, logger.clone());
        }
    }

    pub fn stop_all(&mut self) {
        for handle in self.handles.drain(..) {
            handle.abort();
        }
    }
}
