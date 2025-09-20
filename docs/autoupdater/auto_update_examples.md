# 自动更新使用示例

本文档提供了 BurnCloud 自动更新功能的详细使用示例。

## 命令行使用

### 1. 检查更新

检查是否有可用的新版本，但不执行更新：

```bash
# 仅检查更新
burncloud update --check-only
```

输出示例：
```
检查更新中...
✅ 发现新版本可用！
运行 'burncloud update' 来更新到最新版本
```

或者：
```
检查更新中...
✅ 已是最新版本
```

### 2. 执行更新

检查并执行更新到最新版本：

```bash
# 更新应用程序
burncloud update
```

输出示例：
```
正在更新 BurnCloud...
✅ 更新成功！
请重新启动应用程序以使用新版本
```

更新失败时的输出：
```
正在更新 BurnCloud...
❌ 更新失败: 网络连接错误
你可以手动从以下地址下载最新版本:
  GitHub: https://github.com/burncloud/burncloud/releases
  Gitee:  https://gitee.com/burncloud/burncloud/releases
```

## 编程接口使用

### 1. 基本使用

```rust
use burncloud_common::AutoUpdater;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    env_logger::init();

    // 创建自动更新器
    let updater = AutoUpdater::with_default_config();

    // 检查更新
    if updater.check_for_updates().await? {
        println!("发现新版本，开始更新...");

        // 执行更新
        updater.update_with_fallback().await?;
        println!("更新完成！请重启应用程序。");
    } else {
        println!("已是最新版本");
    }

    Ok(())
}
```

### 2. 自定义配置

```rust
use burncloud_common::{AutoUpdater, UpdateConfig};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 自定义配置
    let config = UpdateConfig {
        github_owner: "myorg".to_string(),
        github_repo: "myapp".to_string(),
        gitee_owner: "myorg".to_string(),
        gitee_repo: "myapp".to_string(),
        bin_name: "myapp".to_string(),
        current_version: "1.2.3".to_string(),
    };

    let updater = AutoUpdater::new(config);

    match updater.update_with_fallback().await {
        Ok(_) => {
            println!("✅ 更新成功");
        }
        Err(e) => {
            eprintln!("❌ 更新失败: {}", e);
        }
    }

    Ok(())
}
```

### 3. 仅检查更新

```rust
use burncloud_common::AutoUpdater;
use anyhow::Result;

async fn check_updates_only() -> Result<()> {
    let updater = AutoUpdater::with_default_config();

    println!("当前版本: {}", updater.current_version());

    match updater.check_for_updates().await? {
        true => {
            println!("🎉 有新版本可用！");
            println!("请手动运行更新命令或调用 update_with_fallback()");
        }
        false => {
            println!("✅ 当前已是最新版本");
        }
    }

    Ok(())
}
```

### 4. 错误处理示例

```rust
use burncloud_common::AutoUpdater;
use anyhow::Result;
use log::{info, warn, error};

async fn robust_update_check() -> Result<()> {
    env_logger::init();

    let updater = AutoUpdater::with_default_config();

    info!("开始检查更新...");

    match updater.check_for_updates().await {
        Ok(true) => {
            info!("发现新版本，开始更新");

            match updater.update_with_fallback().await {
                Ok(_) => {
                    info!("更新成功完成");
                    println!("✅ 更新成功！请重启应用程序以使用新版本。");
                }
                Err(e) => {
                    error!("更新失败: {}", e);
                    println!("❌ 更新失败: {}", e);
                    println!("解决方案:");
                    println!("1. 检查网络连接");
                    println!("2. 检查应用程序是否有写入权限");
                    println!("3. 手动下载新版本:");
                    println!("   - GitHub: https://github.com/burncloud/burncloud/releases");
                    println!("   - Gitee: https://gitee.com/burncloud/burncloud/releases");
                }
            }
        }
        Ok(false) => {
            info!("当前已是最新版本");
            println!("✅ 当前已是最新版本");
        }
        Err(e) => {
            error!("检查更新失败: {}", e);
            println!("❌ 检查更新失败: {}", e);
            println!("可能的原因:");
            println!("1. 网络连接问题");
            println!("2. 仓库访问受限");
            println!("3. 配置错误");
        }
    }

    Ok(())
}
```

### 5. 配置动态修改

```rust
use burncloud_common::{AutoUpdater, UpdateConfig};

async fn dynamic_config_example() -> anyhow::Result<()> {
    let mut updater = AutoUpdater::with_default_config();

    // 显示当前配置
    println!("当前版本: {}", updater.current_version());

    // 检查是否需要切换到企业内部源
    if is_corporate_network() {
        let corporate_config = UpdateConfig {
            github_owner: "internal-corp".to_string(),
            github_repo: "burncloud-enterprise".to_string(),
            // ... 其他企业配置
            ..Default::default()
        };

        updater.set_config(corporate_config);
        println!("已切换到企业内部更新源");
    }

    // 执行更新
    updater.update_with_fallback().await?;

    Ok(())
}

fn is_corporate_network() -> bool {
    // 实际实现中可以检查网络环境
    // 例如检查特定的域名是否可达
    false
}
```

## 集成到应用程序

### 1. 启动时检查更新

```rust
use burncloud_common::AutoUpdater;
use std::time::Duration;
use tokio::time::timeout;

async fn startup_update_check() -> anyhow::Result<()> {
    let updater = AutoUpdater::with_default_config();

    // 设置超时，避免启动时间过长
    let check_result = timeout(
        Duration::from_secs(10),
        updater.check_for_updates()
    ).await;

    match check_result {
        Ok(Ok(true)) => {
            println!("🎉 发现新版本可用！");
            println!("运行 'burncloud update' 来更新");
            println!("或在设置中启用自动更新");
        }
        Ok(Ok(false)) => {
            // 静默处理，不打扰用户
        }
        Ok(Err(e)) => {
            log::debug!("检查更新失败: {}", e);
            // 不显示错误给用户，避免打扰
        }
        Err(_) => {
            log::debug!("检查更新超时");
        }
    }

    Ok(())
}
```

### 2. 后台定期检查

```rust
use burncloud_common::AutoUpdater;
use tokio::time::{interval, Duration};

async fn background_update_checker() {
    let updater = AutoUpdater::with_default_config();
    let mut timer = interval(Duration::from_secs(24 * 3600)); // 每天检查一次

    loop {
        timer.tick().await;

        match updater.check_for_updates().await {
            Ok(true) => {
                // 通知用户有更新可用
                notify_user_update_available();
            }
            Ok(false) => {
                // 无需操作
            }
            Err(e) => {
                log::debug!("后台检查更新失败: {}", e);
            }
        }
    }
}

fn notify_user_update_available() {
    // 实现用户通知逻辑
    // 例如系统通知、GUI 弹窗等
    println!("系统通知: BurnCloud 有新版本可用");
}
```

### 3. GUI 集成示例

```rust
// 假设在 GUI 应用中集成
use burncloud_common::AutoUpdater;

struct UpdateManager {
    updater: AutoUpdater,
    last_check: std::time::Instant,
}

impl UpdateManager {
    fn new() -> Self {
        Self {
            updater: AutoUpdater::with_default_config(),
            last_check: std::time::Instant::now(),
        }
    }

    async fn check_updates_if_needed(&mut self) -> anyhow::Result<bool> {
        // 每小时最多检查一次
        if self.last_check.elapsed().as_secs() < 3600 {
            return Ok(false);
        }

        self.last_check = std::time::Instant::now();
        self.updater.check_for_updates().await
    }

    async fn perform_update(&self) -> anyhow::Result<()> {
        // 在 GUI 中显示进度
        show_update_progress("正在下载更新...");

        match self.updater.update_with_fallback().await {
            Ok(_) => {
                show_update_success("更新成功！请重启应用程序。");
                Ok(())
            }
            Err(e) => {
                show_update_error(&format!("更新失败: {}", e));
                Err(e)
            }
        }
    }
}

fn show_update_progress(message: &str) {
    // GUI 进度显示实现
    println!("GUI: {}", message);
}

fn show_update_success(message: &str) {
    // GUI 成功提示实现
    println!("GUI Success: {}", message);
}

fn show_update_error(message: &str) {
    // GUI 错误提示实现
    println!("GUI Error: {}", message);
}
```

## 环境变量配置

可以通过环境变量配置日志级别：

```bash
# 启用详细日志
export RUST_LOG=info
burncloud update

# 启用调试日志
export RUST_LOG=debug
burncloud update

# 仅显示错误
export RUST_LOG=error
burncloud update
```

## 常见问题解决

### 1. 网络连接问题

```bash
# 检查网络连接
curl -I https://github.com/burncloud/burncloud/releases
curl -I https://gitee.com/burncloud/burncloud/releases

# 设置代理（如果需要）
export https_proxy=http://proxy.company.com:8080
burncloud update
```

### 2. 权限问题

```bash
# Linux/macOS: 给予执行权限
sudo chown $USER:$USER /usr/local/bin/burncloud
chmod +x /usr/local/bin/burncloud

# Windows: 以管理员身份运行
# 右键点击命令提示符 -> "以管理员身份运行"
```

### 3. 企业防火墙

如果企业网络阻止了外部访问，可以：

1. 配置企业内部镜像源
2. 手动下载并分发更新
3. 联系网络管理员开放相关域名

## 最佳实践

1. **定期检查**: 建议设置定期检查更新，但不要过于频繁
2. **用户通知**: 有更新时及时通知用户，但不要强制更新
3. **错误处理**: 妥善处理网络错误，提供备用方案
4. **日志记录**: 记录更新操作，便于问题排查
5. **测试验证**: 在生产环境部署前充分测试更新功能