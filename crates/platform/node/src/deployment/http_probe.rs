// 导入标准库的 Duration，用于设置 HTTP 请求超时与重试间隔
use std::time::Duration;

// 导入 async_trait 宏：Rust 稳定版 trait 中不能直接写 async fn，
// 该宏会把 async fn 改写成返回 Pin<Box<dyn Future>> 的形式
use async_trait::async_trait;

// 导入 tokio 的异步 sleep，用于 readiness 重试循环之间的等待
use tokio::time::sleep;

// 从本 crate 导入探测相关的 trait、错误类型与目标类型：
// - HealthError：健康探测的错误
// - HealthProbe：健康探测 trait（本文件要实现它）
// - ReadinessError：就绪探测的错误
// - ReadinessProbe：就绪探测 trait（本文件要实现它）
// - ReadinessTarget：探测目标，包含 endpoint（一个 URL）
use crate::{HealthError, HealthProbe, ReadinessError, ReadinessProbe, ReadinessTarget};

// 定义 /v1/models 接口返回的 JSON 顶层结构
// 派生 Deserialize：允许从 JSON 反序列化得到此结构
#[derive(serde::Deserialize)]
struct ModelListResponse {
    // 模型列表（对应 OpenAI 风格 API 的 "data" 数组）
    data: Vec<ModelListEntry>,
}

// 定义模型列表中的单个条目
#[derive(serde::Deserialize)]
struct ModelListEntry {
    // 模型 ID，用于与请求中期望的 model_id 对比
    id: String,
    // 模型状态字段是可选的（有些实现不返回 status）
    // Option<ModelStatus>：None 表示 JSON 中缺失或为 null
    status: Option<ModelStatus>,
}

// 定义模型状态结构
#[derive(serde::Deserialize)]
struct ModelStatus {
    // 状态值的字符串形式（如 "loaded"），也是可选字段
    value: Option<String>,
}

// 判断一个 HTTP 响应是否表示"目标健康/就绪"的辅助异步函数
// 参数 response：已经收到的 HTTP 响应（会消费其 body）
// 参数 target：探测目标，用于读取原始 URL 以判断路径和查询参数
// 返回：
//   Ok(true)  -> 健康
//   Ok(false) -> 不健康（探测有效，只是结果是否定）
//   Err(reqwest::Error) -> 解析响应体时发生网络/解码错误
async fn response_is_healthy(
    response: reqwest::Response,
    target: &ReadinessTarget,
) -> Result<bool, reqwest::Error> {
    // 若状态码不是 2xx 成功类，直接判定为不健康
    // is_success() 判断状态码是否在 200..300 范围内
    if !response.status().is_success() {
        return Ok(false);
    }

    // 尝试把 target.endpoint 解析成 URL
    // let Ok(url) = ... else { ... }：let-else 语法，解析失败直接返回 Ok(false)
    // 即：URL 无法解析时，视为不健康（而不是报错）
    let Ok(url) = reqwest::Url::parse(&target.endpoint) else {
        return Ok(false);
    };

    // 若 URL 路径不是 /v1/models，说明不是模型列表接口
    // 这时只根据状态码判断（前面已确认 2xx），直接视为健康
    if url.path() != "/v1/models" {
        return Ok(true);
    }

    // 尝试从查询参数中提取期望的 model_id
    // 遍历所有 (key, value) 对，找到 key == "model_id" 的项，取出 value
    // then(|| value.into_owned())：若 key 匹配则把 Cow<str> 转成 String
    // find_map 找到第一个匹配项并返回 Some(String)，否则返回 None
    let expected_model_id = url
        .query_pairs()
        .find_map(|(key, value)| (key == "model_id").then(|| value.into_owned()));

    // 把响应体解析为 ModelListResponse（JSON）
    // .await? 表示解析失败会向上传播 reqwest::Error
    let body: ModelListResponse = response.json().await?;

    // 判断列表里是否至少有一个模型满足：ID 匹配（若有期望）且状态为 "loaded"
    Ok(body.data.iter().any(|model| {
        // 根据是否指定了 expected_model_id 判断 ID 是否匹配
        let requested_model_matches = match expected_model_id.as_deref() {
            // 指定了期望 ID：要求 model.id 与之完全相等
            Some(expected_model_id) => model.id == expected_model_id,
            // 未指定期望 ID：不限制 ID，恒为 true
            None => true,
        };
        // 两个条件同时满足才算健康：
        // 1) ID 匹配（或未指定）
        // 2) 模型状态 value 等于 "loaded"
        // status.as_ref() 取 Option<ModelStatus> 的引用
        // .and_then(|status| status.value.as_deref()) 取出 Option<&str>
        // == Some("loaded") 判断状态字符串
        requested_model_matches
            && model
                .status
                .as_ref()
                .and_then(|status| status.value.as_deref())
                == Some("loaded")
    }))
}

/// 提供共享 HTTP client 的 readiness 与 health 探测器。
///
/// 每次请求最多等待 10 秒；readiness 在非 2xx 或请求失败后每秒重试，
/// health 则把非 2xx 作为不健康结果返回。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::deployment::HttpProbe;
///
/// let _probe = HttpProbe::default();
/// ```
// 派生 Clone：reqwest::Client 内部是 Arc 共享连接池，Clone 很便宜，
// 所以 HttpProbe 可以低成本克隆后分发给多个使用方
#[derive(Clone)]
pub struct HttpProbe {
    // 共享的 HTTP client：整个探测器复用同一个 client（连接池、超时配置统一）
    client: reqwest::Client,
}

// 为 HttpProbe 实现 Default，提供默认构造方式
impl Default for HttpProbe {
    // default() 返回一个配置好的 HttpProbe
    fn default() -> Self {
        // 用 builder 构造 reqwest::Client
        let client = reqwest::Client::builder()
            // 设置每个请求最多等待 10 秒（全局超时）
            .timeout(Duration::from_secs(10))
            // 构建 client
            .build()
            // 构建失败视为配置/环境级致命错误，直接 panic（不是运行时业务错误）
            .unwrap_or_else(|error| panic!("failed to create HTTP probe client: {error}"));

        // 用构造好的 client 组装 HttpProbe
        Self { client }
    }
}

// 用 async_trait 宏为 HttpProbe 实现 ReadinessProbe trait
// 该宏将 async fn 展开为返回 BoxFuture 的普通 fn，从而兼容 dyn 与稳定版 Rust
#[async_trait]
impl ReadinessProbe for HttpProbe {
    // 就绪探测：阻塞直到目标就绪（名字里的 wait 就是这个含义）
    // 最多等待 10 分钟，避免模型地址无效时部署请求永久挂起。
    async fn wait_ready(&self, target: ReadinessTarget) -> Result<(), ReadinessError> {
        // 外层用 tokio::time::timeout 包裹整个循环，设置总超时为 600 秒（10 分钟）
        // 一旦超时，整体返回 Err(Elapsed)，由下面的 map_err 转换为业务错误
        tokio::time::timeout(Duration::from_secs(600), async {
            // 无限重试循环：直到健康或超时
            loop {
                // 发起一次 GET 请求并等待响应
                match self.client.get(target.endpoint.as_str()).send().await {
                    // 收到响应
                    Ok(response) => {
                        // 再次判断响应内容是否健康；只有 Ok(true) 才认为就绪并返回成功
                        // 若返回 Ok(false)（不健康）或 Err（body 解析错误），则继续重试
                        if let Ok(true) = response_is_healthy(response, &target).await {
                            return Ok(());
                        }
                    }
                    // 请求构造失败（如 URL 非法、header 非法等 builder 错误）
                    // 这种错误重试也不会成功，直接返回 CheckFailed 终止等待
                    Err(error) if error.is_builder() => {
                        return Err(ReadinessError::CheckFailed(error.to_string()));
                    }
                    // 其他网络错误（连接失败、超时、DNS 失败等）：忽略并重试
                    Err(_) => {}
                }

                // 每次失败后等待 1 秒再重试，避免忙循环
                sleep(Duration::from_secs(1)).await;
            }
        })
        // 等待上面 timeout 包裹的 future 完成
        // 若外层超时（Elapsed），将其映射为 ReadinessError::CheckFailed
        // 注意超时字符串明确说明"600 秒后超时"，便于排查
        .await
        .map_err(|_| {
            ReadinessError::CheckFailed("runtime readiness timed out after 600 seconds".into())
        })?
        // 使用 `?` 后，Result 内部若为 Err 会向上传播（即内层返回的 CheckFailed）
        // 若为 Ok，则继续，函数整体返回 Ok(())
    }
}

// 用 async_trait 宏为 HttpProbe 实现 HealthProbe trait
#[async_trait]
impl HealthProbe for HttpProbe {
    // 健康探测：单次、非阻塞地判定目标此刻是否健康
    // 返回 Ok(true) 健康；Ok(false) 不健康（探测有效）；Err 探测本身失败
    async fn is_healthy(&self, target: ReadinessTarget) -> Result<bool, HealthError> {
        // 发起单次 GET 请求并等待响应
        let response = self
            .client
            .get(target.endpoint.as_str())
            .send()
            .await
            // 请求失败（任何 reqwest::Error）-> 包装成 HealthError::CheckFailed 返回
            // 注意：health 不重试，失败即 Err
            .map_err(|error| HealthError::CheckFailed(error.to_string()))?;

        // 复用 response_is_healthy 判断响应内容
        // 成功返回 Ok(true/false)；解析失败（reqwest::Error）映射为 HealthError::CheckFailed
        response_is_healthy(response, &target)
            .await
            .map_err(|error| HealthError::CheckFailed(error.to_string()))
    }
}
