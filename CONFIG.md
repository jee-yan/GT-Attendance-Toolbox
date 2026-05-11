# DS-K1T670M 工具箱 - 配置文件说明

配置文件路径：项目根目录下 `config.yaml`

---

## 完整结构

```yaml
app:          # 应用设置
buttons:      # 按钮列表（固定 4 个）
schedules:    # 定时任务列表
```

---

## 1. app - 应用设置

```yaml
app:
  title: "你的应用标题"
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `title` | string | 是 | 窗口标题栏显示的文字 |

---

## 2. buttons - 按钮配置

按钮数量由配置文件决定，最少 1 个，最多 4 个。按数组顺序从左到右、从上到下排列（2列×N行）。

```yaml
buttons:
  - id: "button_1"          # 按钮唯一标识
    label: "按钮名称"        # 按钮显示文字
    action:                  # 按钮点击执行的动作
      type: script           # 动作类型：script 或 http
      script: |
        return { success: true, message: '执行完成' };
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 按钮唯一标识 |
| `label` | string | 是 | 按钮上显示的文字 |
| `action` | object | 是 | 按钮绑定的动作配置 |

### 按钮布局（按实际数量自动适配）

1 个按钮：
```
┌──────────┐
│ button_1 │
└──────────┘
```

2 个按钮：
```
┌──────────┐  ┌──────────┐
│ button_1 │  │ button_2 │
└──────────┘  └──────────┘
```

3 个按钮：
```
┌──────────┐  ┌──────────┐
│ button_1 │  │ button_2 │
└──────────┘  └──────────┘
┌──────────┐
│ button_3 │
└──────────┘
```

4 个按钮：
```
┌──────────┐  ┌──────────┐
│ button_1 │  │ button_2 │
└──────────┘  └──────────┘
┌──────────┐  ┌──────────┐
│ button_3 │  │ button_4 │
└──────────┘  └──────────┘
```

---

## 3. action - 动作配置

通过 `type` 字段区分，支持 `script` 和 `http` 两种。

### 3.1 JS 脚本 (type: script)

```yaml
action:
  type: script
  script: |
    // 在浏览器 WebView 中执行的 JavaScript 代码
    // 支持 async/await、fetch 等 Web API
    // 必须返回 { success: boolean, message: string }
    const resp = await fetch('http://localhost:3000/api/data');
    const data = await resp.json();
    return {
      success: true,
      message: '请求成功',
      data: data
    };
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | 是 | 固定值 `"script"` |
| `script` | string | 是 | JavaScript 代码，支持 async/await |

**返回值格式：**

```javascript
// 成功
return { success: true, message: "执行成功", data: { ... } };

// 失败
return { success: false, message: "失败原因" };
```

> **Loading 行为**：点击按钮后立即进入 loading 状态，loading 持续到脚本 `return` 为止。脚本中的耗时操作（如网络请求、`await` 等待）会自然地延长 loading 时间。示例：
>
> ```javascript
> // loading 持续 3 秒后显示成功
> await new Promise(r => setTimeout(r, 3000));
> return { success: true, message: '完成' };
> ```

### 3.2 HTTP 请求 (type: http)

```yaml
action:
  type: http
  method: POST
  url: "http://localhost:3000/api/data"
  headers:
    Content-Type: "application/json"
    Authorization: "Bearer your-token"
  body: '{"key": "value"}'
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `type` | string | 是 | 固定值 `"http"` |
| `method` | string | 是 | 支持：`GET`、`POST`、`PUT`、`DELETE`、`PATCH` |
| `url` | string | 是 | 请求的完整 URL |
| `headers` | map | 否 | 请求头键值对 |
| `body` | string | 否 | 请求体，仅 POST/PUT/PATCH 有效 |

---

## 4. schedules - 定时任务配置

通过界面底部的**定时任务开关**控制是否执行。开关默认为**关闭**状态。

```yaml
schedules:
  - id: "task_1"
    name: "任务名称"
    cron: "0 */5 * * * *"
    action:
      type: http
      method: GET
      url: "http://localhost:3000/health"
```

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `id` | string | 是 | 任务唯一标识 |
| `name` | string | 是 | 任务名称，日志中显示 |
| `cron` | string | 是 | Cron 表达式（6位，含秒） |
| `action` | object | 是 | 触发时执行的动作（同按钮） |

---

## 5. Cron 表达式

格式：`秒 分 时 日 月 周`

| 位置 | 含义 | 取值范围 |
|------|------|----------|
| 1 | 秒 | 0-59 |
| 2 | 分 | 0-59 |
| 3 | 时 | 0-23 |
| 4 | 日 | 1-31 |
| 5 | 月 | 1-12 |
| 6 | 周 | 0-7（0和7=周日） |

**常用示例：**

| 表达式 | 含义 |
|--------|------|
| `0 */5 * * * *` | 每 5 分钟 |
| `0 0 * * * *` | 每小时整点 |
| `0 0 8 * * *` | 每天 08:00 |
| `0 30 9 * * 1-5` | 工作日 09:30 |
| `0 0 0 1 * *` | 每月 1 号 00:00 |
| `30 8,12,18 * * * *` | 每天 08:30、12:30、18:30 |

---

## 6. License 机制

应用启动时检查根目录下的 `license.json` 文件。

**验证规则：**
- 文件不存在 → 弹窗阻断
- 文件内容被篡改 → 弹窗阻断
- 签名校验失败 → 弹窗阻断

**License 文件格式（由验证接口自动生成，无需手动创建）：**

```json
{
  "code": "XXXX-XXXX-XXXX",
  "issued_at": "2026-05-07T11:20:00+00:00",
  "signature": "sha256hash"
}
```

签名算法：`SHA256(code + issued_at + 内置salt)`

---

## 7. 完整示例

```yaml
app:
  title: "运维工具箱"

buttons:
  - id: "sync_personnel"
    label: "同步人员"
    action:
      type: script
      script: |
        const resp = await fetch('http://localhost:3000/api/personnel/sync');
        const data = await resp.json();
        return { success: true, message: `同步完成，共 ${data.count} 条`, data };

  - id: "today_attendance"
    label: "今日考勤"
    action:
      type: script
      script: |
        const resp = await fetch('http://localhost:3000/api/attendance/today');
        const data = await resp.json();
        return { success: true, message: '今日考勤查询完成', data };

  - id: "yesterday_attendance"
    label: "昨日考勤"
    action:
      type: http
      method: GET
      url: "http://localhost:3000/api/attendance/yesterday"
      headers:
        Accept: "application/json"

  - id: "export_report"
    label: "导出报表"
    action:
      type: http
      method: POST
      url: "http://localhost:3000/api/report/export"
      headers:
        Content-Type: "application/json"
      body: '{"type": "monthly", "format": "xlsx"}'

schedules:
  - id: "auto_sync"
    name: "自动同步人员"
    cron: "0 0 8 * * *"
    action:
      type: script
      script: |
        const resp = await fetch('http://localhost:3000/api/personnel/sync');
        return { success: true, message: '自动同步完成' };

  - id: "daily_check"
    name: "每日巡检"
    cron: "0 30 9 * * 1-5"
    action:
      type: http
      method: GET
      url: "http://localhost:3000/api/system/health"
```

---

## 8. 注意事项

1. **配置文件编码**：必须使用 UTF-8
2. **按钮数量**：1~4 个可配，多余 4 个的部分忽略
3. **脚本环境**：在浏览器 WebView 中执行，可使用 `fetch`、`console.log` 等 Web API
4. **脚本返回值**：必须包含 `success` (boolean) 字段
5. **Cron 表达式**：6 位格式（含秒），非标准 5 位
6. **定时开关**：每次启动应用默认为关闭状态，需手动开启
7. **配置重载**：修改后通过界面「导入配置」按钮重新加载
