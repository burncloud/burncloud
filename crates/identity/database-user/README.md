# burncloud-database-user

`user_` 业务域数据库 crate。管理用户账户、充值记录和应用层 API Key。
支持双币种钱包（USD/CNY），金额以纳美元（i64）精度存储。

## 关键类型

| 类型 | 说明 |
|------|------|
| `UserDatabase` / `UserAccountModel` | Crate 控制器，`init(&db)` + 用户账户操作（两者为同一类型的别名） |
| `UserAccount` / `UserAccountInput` | `user_accounts` 行类型 / 创建输入 |
| `UserRecharge` | `user_recharges` 行类型 |
| `UserApiKey` / `UserApiKeyModel` / `UserApiKeyInput` / `UserApiKeyUpdateInput` | `user_api_keys` 相关 |

## 目录结构

```
src/
├── lib.rs                — UserDatabase / UserAccountModel（聚合器+主操作集）
├── user_account.rs       — UserAccount, UserAccountInput
├── user_recharge.rs      — UserRecharge
├── user_api_key.rs       — UserApiKey, UserApiKeyModel, UserApiKeyInput, UserApiKeyUpdateInput
└── common.rs             — current_timestamp()
```

## 依赖

- `burncloud-database`, `burncloud-common`, `rand` — 核心抽象、共享类型、Key 生成