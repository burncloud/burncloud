# burncloud-service-ip

IP 地理位置服务。判断用户区域(CN/WORLD),带缓存。

## 关键类型

| 类型 | 说明 |
|------|------|
| `Region` | 区域枚举(CN, WORLD) |
| `get_location()` | IP → 地理位置查询 |
| `get_user_region()` | 获取用户所在区域 |

## 依赖

- `burncloud-service-setting` — 配置读取
- `reqwest` — HTTP 请求外部 IP 查询 API

## 注意

纯外部服务封装,无对应 database 子 crate。
