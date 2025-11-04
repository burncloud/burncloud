# BurnCloud Service Models

模型服务层，提供简洁的增删改查接口

## 功能特性

- ✅ 调用 `burncloud-database-models` 数据库层
- ✅ 提供简洁的 CRUD 接口
- ✅ 支持高级查询（搜索、排序）
- ✅ 100% Rust 编写
- ✅ 异步设计

## API 接口

### ModelService

```rust
// 创建服务
let service = ModelService::new().await?;

// 增：添加模型
service.create(&model).await?;

// 删：删除模型
service.delete("model_id").await?;

// 改：更新模型
service.update(&model).await?;

// 查：获取单个模型
let model = service.get("model_id").await?;

// 查：列出所有模型
let models = service.list().await?;

// 查：根据类型搜索
let models = service.search_by_pipeline("text-generation").await?;

// 查：获取热门模型
let popular = service.get_popular(10).await?;

// 关闭服务
service.close().await?;
```

## 使用示例

运行示例程序：

```bash
cargo run --example usage
```

## 依赖关系

```
burncloud-service-models
  └─ burncloud-database-models
       └─ burncloud-database
```
