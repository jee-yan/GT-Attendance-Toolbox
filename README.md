# GT-Attendance-Toolbox

考勤机管理桌面应用 —— 配置推送、同步人员、定时任务

## 功能

- **配置推送** — 一键配置考勤机 HTTP 推送，实时上报人脸/指纹打卡事件
- **同步人员** — 从考勤机批量拉取人员信息，同步到后端服务器
- **定时任务** — 支持 cron 表达式定时执行同步等操作
- **多设备支持** — 同时管理多台考勤机
- **可视化日志** — 实时查看操作日志和设备推送事件

## 技术栈

| 层 | 技术 |
|---|---|
| 前端 | TypeScript + Vite |
| 桌面框架 | Tauri 2.x |
| 后端逻辑 | Rust |
| 设备通信 | HTTP/Digest Auth |
| 配置格式 | YAML |

## 快速开始

### 环境要求

- [Node.js](https://nodejs.org/) >= 18
- [Rust](https://rustup.rs/) (nightly 不需要，stable 即可)
- Tauri 系统依赖：参考 [Tauri 官方文档](https://tauri.app/start/prerequisites/)

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
npm run tauri dev
```

### 构建

```bash
npm run tauri build
```

## 配置

在项目根目录创建 `config.yaml`：

```yaml
app:
  title: "考勤同步工具箱"

devices:
  - id: "device_1"
    name: "XX考勤机"
    ip: "192.168.1.100"
    port: 80
    username: "admin"
    password: "your_password"

server:
  base_url: "http://your-server:8000"
  upload_personnel_url: "/api/personnel/sync"
  push_path: "/api/attendance/push"

buttons:
  - id: "sync_personnel"
    label: "同步人员"
    action:
      type: script
      script: |
        return await window.syncPersonnel();

  - id: "configure_push"
    label: "配置推送"
    action:
      type: script
      script: |
        return await window.configurePush();

schedules:
  - id: "auto_sync"
    name: "自动同步人员"
    cron: "0 0 8 * * *"
    action:
      type: script
      script: |
        return await window.syncPersonnel();
```

完整配置说明见 [CONFIG.md](./CONFIG.md)。

## 项目结构

```
├── src/                    # 前端代码 (TypeScript)
│   ├── main.ts             # 主入口
│   └── styles/main.css     # 样式
├── src-tauri/              # 后端代码 (Rust)
│   ├── src/
│   │   ├── isapi_client.rs # 考勤机通信 (ISAPI/Digest Auth)
│   │   ├── sync_manager.rs # 同步与推送管理
│   │   ├── commands.rs     # Tauri 命令
│   │   ├── config.rs       # 配置解析
│   │   └── scheduler.rs    # 定时任务
│   └── Cargo.toml
├── docs/                   # 文档
│   └── api-attendance-push.md  # 推送接口文档
├── config.yaml             # 本地配置 (不提交到 git)
└── CONFIG.md               # 配置说明
```

## License

[MIT](./license)
