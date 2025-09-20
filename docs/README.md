
  burncloud/
  ├── Cargo.toml (workspace)
  ├── packages/
  │   ├── core/           # 核心逻辑和数据结构
  │   ├── common/         # 共享工具和类型
  │   ├── server/         # 服务器后端
  │   ├── cli/           # 命令行工具
  │   ├── client/        # GUI客户端(Dioxus)
  │   └── code/          # 编程相关功能
  └── bin/
      └── burncloud.exe  # 主入口点

  各包的职责分工

  1. core - 核心业务逻辑
  - 模型管理(pull, run操作)
  - 配置管理
  - 数据存储抽象
  - 业务规则

  2. common - 共享基础设施
  - 错误处理
  - 日志
  - 序列化/反序列化
  - 网络工具
  - 文件操作

  3. server - 后端服务
  - HTTP API服务器
  - 模型推理服务
  - 资源管理

  4. cli - 命令行界面
  - 参数解析
  - 命令路由
  - 终端输出格式化

  5. client - GUI客户端
  - Dioxus界面
  - 状态管理
  - 与core的集成

  6. code - 编程工具
  - 代码生成
  - IDE集成
  - 开发者工具

  运行模式设计

  // 主入口逻辑
  fn main() {
      let args = env::args().collect::<Vec<String>>();

      match args.as_slice() {
          [_] => {
              // burncloud.exe (无参数)
              #[cfg(windows)]
              client::launch_gui();
              #[cfg(not(windows))]
              cli::show_help();
          },
          [_, "server"] => server::start(),
          [_, "code"] => code::start_cli(),
          [_, "pull", model] => core::pull_model(model),
          [_, "run", model] => core::run_model(model),
          _ => cli::handle_command(&args[1..]),
      }
  }

● 依赖关系图

  burncloud (main binary)
  ├── cli ──────┐
  ├── client ───┼── core ── common
  ├── server ───┤      └── server
  └── code ─────┘
