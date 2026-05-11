# 考勤事件推送接口文档

## 概述

考勤机（DS-K1T670M）通过 HTTP 回调方式，将考勤事件实时推送到后端服务器。

## 接口信息

| 项目 | 值 |
|------|-----|
| 请求方式 | `POST` |
| Content-Type | `application/json` |
| 认证方式 | 无（设备已配置 `httpAuthenticationMethod: none`） |
| 推送路径 | `/api/attendance/push` |

## 请求体

设备推送的 JSON 结构如下（海康 ISAPI 标准格式）：

```json
{
  "AccessControllerEvent": {
    "dateTime": "2026-05-08T08:30:15+08:00",
    "eventType": "accessControllerEvent",
    "subEventType": "faceRecognition",
    "employeeNo": "00001",
    "name": "张三",
    "doorNo": 1,
    "currentVerifyMode": "face",
    "verifyResult": "success",
    "mask": "no",
    "picturesNumber": 1,
    "pictures": [
      {
        "pictureURL": "http://192.168.1.100/ISAPI/Security/faceData/1?format=json"
      }
    ],
    "serialNo": "DSK1T670M20240101AAWR",
    "deviceName": "工地1号考勤机"
  }
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `dateTime` | string | 事件发生时间，ISO 8601 格式，含时区 |
| `eventType` | string | 事件类型，固定为 `accessControllerEvent` |
| `subEventType` | string | 子类型：`faceRecognition`(人脸)、`cardRecognition`(刷卡)、`fingerprintRecognition`(指纹)、`passwordRecognition`(密码) |
| `employeeNo` | string | 人员编号（工号） |
| `name` | string | 人员姓名 |
| `doorNo` | int | 门编号，通常为 `1` |
| `currentVerifyMode` | string | 验证方式：`face`、`card`、`fingerprint`、`password`、`faceOrCardOrPassword` 等 |
| `verifyResult` | string | 验证结果：`success`(成功)、`failed`(失败) |
| `mask` | string | 口罩状态：`yes`、`no`、`unrecognized` |
| `picturesNumber` | int | 附带图片数量 |
| `pictures` | array | 图片信息数组 |
| `pictures[].pictureURL` | string | 抓拍图片 URL（需设备网络可达才可下载） |
| `serialNo` | string | 设备序列号 |
| `deviceName` | string | 设备名称（部分固件不返回此字段） |

> 注：不同固件版本返回字段可能有差异，以上为核心字段。实际推送可能还包含 `activePost`、`attendanceStatus`、`checkTime` 等扩展字段，建议后端以宽松模式解析，忽略未知字段。

## 响应要求

设备要求服务器返回 HTTP 200 状态码，否则设备会尝试重推。

```json
{
  "statusCode": 1,
  "statusString": "OK"
}
```

| 字段 | 类型 | 说明 |
|------|------|------|
| `statusCode` | int | `1` 表示成功 |
| `statusString` | string | 状态描述 |

## 后端实现建议

### 1. 数据库存储（推荐字段）

| 表字段 | 来源 | 类型 | 说明 |
|--------|------|------|------|
| `id` | 自增 | bigint | 主键 |
| `event_time` | `dateTime` | datetime | 考勤时间 |
| `employee_no` | `employeeNo` | varchar | 工号 |
| `name` | `name` | varchar | 姓名 |
| `sub_event_type` | `subEventType` | varchar | 识别方式 |
| `verify_result` | `verifyResult` | varchar | 验证结果 |
| `door_no` | `doorNo` | int | 门编号 |
| `device_serial_no` | `serialNo` | varchar | 设备序列号（关联设备） |
| `raw_data` | 整个请求体 | json | 原始数据（便于排查问题） |
| `created_at` | 服务器时间 | datetime | 入库时间 |

### 2. 去重建议

设备重推时会发送相同数据，建议以 `serialNo` + `employeeNo` + `dateTime` 作为唯一键去重。

### 3. 设备 IP 获取

如需记录设备来源 IP，从请求头 `X-Forwarded-For` 或 `Remote-Address` 获取。

## 调试方法

工具箱配置推送后，可以用以下方式验证：

1. **工具箱日志**：点击"配置推送"按钮，日志会显示配置结果
2. **设备 Web 管理页**：访问 `http://设备IP` → 事件 → HTTP 推送，查看推送状态
3. **后端日志**：在接口入口打印完整请求体，确认收到数据
4. **手动测试**：在设备上刷一次脸，观察后端是否收到事件

## cURL 模拟测试

开发阶段可用 cURL 模拟设备推送：

```bash
curl -X POST http://localhost:8000/api/attendance/push \
  -H "Content-Type: application/json" \
  -d '{
    "AccessControllerEvent": {
      "dateTime": "2026-05-08T08:30:15+08:00",
      "eventType": "accessControllerEvent",
      "subEventType": "faceRecognition",
      "employeeNo": "00001",
      "name": "张三",
      "doorNo": 1,
      "currentVerifyMode": "face",
      "verifyResult": "success",
      "mask": "no",
      "picturesNumber": 0,
      "pictures": [],
      "serialNo": "DSK1T670M20240101AAWR"
    }
  }'
```

期望响应：

```json
{"statusCode": 1, "statusString": "OK"}
```
