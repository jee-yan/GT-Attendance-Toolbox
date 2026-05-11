import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";

interface ActionConfig {
  type: "http" | "script";
  method?: string;
  url?: string;
  headers?: Record<string, string>;
  body?: string;
  script?: string;
}

interface ButtonConfig {
  id: string;
  label: string;
  action: ActionConfig;
}

interface AppConfig {
  app: { title: string };
  buttons: ButtonConfig[];
  devices?: DeviceConfig[];
}

interface DeviceConfig {
  id: string;
  name: string;
  ip: string;
  port: number;
  username: string;
  password: string;
}

interface SyncResult {
  device_id: string;
  device_name: string;
  success: boolean;
  total: number;
  message: string;
}

let logExpanded = false;

// ========== 初始化 ==========
async function init() {
  // 检查 license
  const hasLicense = await invoke<boolean>("check_license");

  if (hasLicense) {
    unlockApp();
  } else {
    showLicenseModal();
  }

  // 监听后端 license 请求事件
  await listen("license-required", () => {
    showLicenseModal();
  });

  // 加载配置
  try {
    const config = await invoke<AppConfig>("load_config");
    applyConfig(config);
    appendLog("info", "系统", "配置加载成功");
  } catch (e) {
    appendLog("error", "系统", `配置加载失败: ${e}`);
    renderDefaultButtons();
  }

  // 监听后端日志事件
  await listen<{ level: string; source: string; message: string }>(
    "log-event",
    (event) => {
      appendLog(event.payload.level, event.payload.source, event.payload.message);
    }
  );

  // 定时任务开关
  const schedulerSwitch = document.getElementById("scheduler-switch") as HTMLInputElement;
  schedulerSwitch.addEventListener("change", async () => {
    try {
      const enabled = await invoke<boolean>("toggle_scheduler", {
        enabled: schedulerSwitch.checked,
      });
      updateSchedulerStatus(enabled);
      appendLog("info", "调度器", enabled ? "定时任务已开启" : "定时任务已关闭");
    } catch (e) {
      schedulerSwitch.checked = !schedulerSwitch.checked;
      appendLog("error", "调度器", `切换失败: ${e}`);
    }
  });

  // 初始化定时开关状态（默认关闭）
  updateSchedulerStatus(false);
}

// ========== License ==========
function showLicenseModal() {
  const modal = document.getElementById("license-modal")!;
  const app = document.getElementById("app")!;
  modal.classList.remove("hidden");
  app.classList.add("locked");

  const input = document.getElementById("license-input") as HTMLInputElement;
  const submitBtn = document.getElementById("license-submit") as HTMLButtonElement;
  const errorText = document.getElementById("license-error")!;

  input.value = "";
  errorText.classList.add("hidden");
  input.focus();

  const doVerify = async () => {
    const code = input.value.trim();
    if (!code) {
      errorText.textContent = "请输入注册码";
      errorText.classList.remove("hidden");
      return;
    }

    submitBtn.disabled = true;
    submitBtn.textContent = "验证中...";
    errorText.classList.add("hidden");

    try {
      await invoke("verify_license", { code });
      unlockApp();
      appendLog("info", "系统", "License 验证成功");
    } catch (e) {
      errorText.textContent = `验证失败: ${e}`;
      errorText.classList.remove("hidden");
    } finally {
      submitBtn.disabled = false;
      submitBtn.textContent = "验证";
    }
  };

  submitBtn.onclick = doVerify;
  input.onkeydown = (e) => {
    if (e.key === "Enter") doVerify();
  };
}

function unlockApp() {
  const modal = document.getElementById("license-modal")!;
  const app = document.getElementById("app")!;
  modal.classList.add("hidden");
  app.classList.remove("locked");
}

// ========== License 管理 ==========
(window as any).openLicenseManage = async function (e: Event) {
  e.stopPropagation();
  const modal = document.getElementById("license-manage-modal")!;
  const codeSpan = document.getElementById("current-code")!;
  const input = document.getElementById("new-license-input") as HTMLInputElement;
  const errorText = document.getElementById("new-license-error")!;
  const switchBtn = document.getElementById("new-license-submit") as HTMLButtonElement;
  const logoutBtn = document.getElementById("license-logout") as HTMLButtonElement;

  // 显示当前注册码
  try {
    const code = await invoke<string | null>("get_license_code");
    codeSpan.textContent = code || "-";
  } catch {
    codeSpan.textContent = "-";
  }

  input.value = "";
  errorText.classList.add("hidden");
  modal.classList.remove("hidden");

  // 切换注册码
  const doSwitch = async () => {
    const code = input.value.trim();
    if (!code) {
      errorText.textContent = "请输入新注册码";
      errorText.classList.remove("hidden");
      return;
    }
    switchBtn.disabled = true;
    switchBtn.textContent = "验证中...";
    errorText.classList.add("hidden");

    try {
      await invoke("verify_license", { code });
      modal.classList.add("hidden");
      appendLog("info", "系统", "License 已切换");
    } catch (err) {
      errorText.textContent = `验证失败: ${err}`;
      errorText.classList.remove("hidden");
    } finally {
      switchBtn.disabled = false;
      switchBtn.textContent = "切换";
    }
  };

  switchBtn.onclick = doSwitch;
  input.onkeydown = (ev) => {
    if (ev.key === "Enter") doSwitch();
  };

  // 注销
  logoutBtn.onclick = async () => {
    try {
      await invoke("logout_license");
      modal.classList.add("hidden");
      appendLog("info", "系统", "License 已注销，应用即将关闭");
      // 延迟后关闭应用
      setTimeout(async () => {
        await invoke("quit_app");
      }, 1500);
    } catch (err) {
      errorText.textContent = `注销失败: ${err}`;
      errorText.classList.remove("hidden");
    }
  };
};

function closeLicenseManage() {
  document.getElementById("license-manage-modal")!.classList.add("hidden");
}
(window as any).closeLicenseManage = closeLicenseManage;

// ESC 键关闭弹窗
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    const manageModal = document.getElementById("license-manage-modal")!;
    if (!manageModal.classList.contains("hidden")) {
      closeLicenseManage();
    }
  }
});

// ========== 配置 ==========
function applyConfig(config: AppConfig) {
  const title = document.getElementById("app-title");
  if (title) title.textContent = config.app.title;
  renderButtons(config.buttons);
}

function renderButtons(buttons: ButtonConfig[]) {
  const grid = document.getElementById("button-grid")!;
  grid.innerHTML = "";

  // 最多取 4 个按钮
  const limited = buttons.slice(0, 4);

  limited.forEach((btn) => {
    const button = document.createElement("button");
    button.className = "action-btn";
    button.textContent = btn.label;
    button.dataset.id = btn.id;

    button.addEventListener("click", () => executeAction(button, btn));
    grid.appendChild(button);
  });
}

function renderDefaultButtons() {
  const defaultButtons: ButtonConfig[] = [
    {
      id: "sync_personnel",
      label: "同步人员",
      action: { type: "script", script: "return { success: true, message: '同步完成' };" },
    },
    {
      id: "today_attendance",
      label: "今日考勤",
      action: { type: "script", script: "return { success: true, message: '查询完成' };" },
    },
    {
      id: "yesterday_attendance",
      label: "昨日考勤",
      action: { type: "script", script: "return { success: true, message: '查询完成' };" },
    },
    {
      id: "placeholder",
      label: "未配置",
      action: { type: "script", script: "return { success: true, message: '未配置' };" },
    },
  ];
  renderButtons(defaultButtons);
}

// ========== 按钮执行 ==========
async function executeAction(button: HTMLButtonElement, config: ButtonConfig) {
  button.classList.add("loading");
  button.classList.remove("success", "error");

  try {
    let result: { success: boolean; message: string; data?: unknown };

    if (config.action.type === "script" && config.action.script) {
      result = await executeScriptInWebView(config.action.script);
    } else {
      result = await invoke<{ success: boolean; message: string; data?: unknown }>(
        "execute_action",
        { action: config.action }
      );
    }

    if (result.success) {
      button.classList.add("success");
      appendLog("info", config.label, result.message || "执行成功");
    } else {
      button.classList.add("error");
      appendLog("error", config.label, result.message || "执行失败");
    }

    setTimeout(() => {
      button.classList.remove("success", "error");
    }, 2000);
  } catch (e) {
    button.classList.add("error");
    appendLog("error", config.label, `执行异常: ${e}`);
    setTimeout(() => button.classList.remove("error"), 2000);
  } finally {
    button.classList.remove("loading");
  }
}

async function executeScriptInWebView(
  script: string
): Promise<{ success: boolean; message: string; data?: unknown }> {
  try {
    const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;
    const fn = new AsyncFunction(script);
    const result = await fn();

    if (result && typeof result === "object" && "success" in result) {
      return result;
    }
    return { success: true, message: "脚本执行完成", data: result };
  } catch (e) {
    return { success: false, message: `脚本执行失败: ${e}` };
  }
}

// ========== 定时任务开关 ==========
function updateSchedulerStatus(enabled: boolean) {
  const status = document.getElementById("scheduler-status")!;
  status.textContent = enabled ? "已开启" : "已关闭";
  status.className = enabled ? "status-on" : "status-off";
}

// ========== 日志 ==========
function appendLog(level: string, source: string, message: string) {
  const content = document.getElementById("log-content")!;
  const panel = document.getElementById("log-panel")!;
  const entry = document.createElement("div");
  entry.className = `log-entry ${level}`;

  const time = new Date().toLocaleTimeString("zh-CN", { hour12: false });
  entry.innerHTML = `<span class="time">[${time}]</span><span class="level">[${level.toUpperCase()}]</span><span class="source">[${source}]</span>${message}`;

  content.appendChild(entry);

  // 自动滚动到底部（延迟一帧确保 DOM 更新后再滚动）
  requestAnimationFrame(() => {
    panel.scrollTop = panel.scrollHeight;
    content.scrollTop = content.scrollHeight;
  });
}

(window as any).toggleLog = function () {
  const panel = document.getElementById("log-panel")!;
  const toggle = document.getElementById("log-toggle")!;
  logExpanded = !logExpanded;

  if (logExpanded) {
    panel.classList.remove("hidden");
    toggle.classList.add("expanded");
  } else {
    panel.classList.add("hidden");
    toggle.classList.remove("expanded");
  }
};

(window as any).clearLogs = function (e: Event) {
  e.stopPropagation();
  const content = document.getElementById("log-content")!;
  content.innerHTML = "";
  appendLog("info", "系统", "日志已清空");
};

(window as any).importConfig = async function (e: Event) {
  e.stopPropagation();
  try {
    const filePath = await open({
      multiple: false,
      filters: [{ name: "YAML", extensions: ["yaml", "yml"] }],
    });

    if (filePath) {
      const config = await invoke<AppConfig>("import_config", { path: filePath });
      applyConfig(config);
      appendLog("info", "系统", `配置已导入: ${filePath}`);
    }
  } catch (e) {
    appendLog("error", "系统", `导入失败: ${e}`);
  }
};

// ========== 同步功能 ==========
(window as any).syncPersonnel = async function (deviceId?: string) {
  try {
    appendLog("info", "同步", "开始同步人员数据...");
    const results = await invoke<SyncResult[]>("sync_personnel", { deviceId: deviceId || null });

    for (const result of results) {
      if (result.success) {
        appendLog("info", "同步", `[${result.device_name}] ${result.message}`);
      } else {
        appendLog("error", "同步", `[${result.device_name}] ${result.message}`);
      }
    }

    const totalSuccess = results.filter(r => r.success).length;
    const totalRecords = results.reduce((sum, r) => sum + r.total, 0);
    return { success: totalSuccess > 0, message: `同步完成: ${totalSuccess}/${results.length} 台设备, 共 ${totalRecords} 条记录` };
  } catch (e) {
    appendLog("error", "同步", `人员同步失败: ${e}`);
    return { success: false, message: `同步失败: ${e}` };
  }
};

(window as any).configurePush = async function (deviceId?: string) {
  try {
    appendLog("info", "推送", "开始配置设备 HTTP 推送...");
    const results = await invoke<SyncResult[]>("configure_push", { deviceId: deviceId || null });

    for (const result of results) {
      if (result.success) {
        appendLog("info", "推送", `[${result.device_name}] ${result.message}`);
      } else {
        appendLog("error", "推送", `[${result.device_name}] ${result.message}`);
      }
    }

    const totalSuccess = results.filter(r => r.success).length;
    return { success: totalSuccess > 0, message: `配置完成: ${totalSuccess}/${results.length} 台设备` };
  } catch (e) {
    appendLog("error", "推送", `推送配置失败: ${e}`);
    return { success: false, message: `配置失败: ${e}` };
  }
};

(window as any).testDevice = async function (deviceId: string) {
  try {
    const result = await invoke<boolean>("test_device_connection", { deviceId });
    return result;
  } catch (e) {
    return false;
  }
};

init();
