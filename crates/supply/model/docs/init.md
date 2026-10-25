# Model Resolver 使用说明

Resolver 的输入是调用方已经准备好的 `ModelManifest`。Manifest 应包含模型名称、支持的
Runtime、Variant 的资源要求、`artifact_uri` 和 `checksum`。

Resolver 的调用流程如下：

```text
上游加载 Manifest
    -> 采集设备 RAM、VRAM、磁盘和 Runtime 信息
    -> RealModelResolver::resolve_with_capabilities
    -> 返回全部兼容 Variant
```

Resolver 不负责：

- 访问 HuggingFace 文件树；
- 递归扫描或筛选 GGUF 文件；
- 构建 artifact 下载地址；
- 下载、缓存或启动模型。

阶段三示例位于 `examples/phase_three.rs`，对应的 Manifest 示例位于
`examples/phase_three_manifest.json`。
