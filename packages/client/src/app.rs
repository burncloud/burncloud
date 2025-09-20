#[cfg(feature = "gui")]
use dioxus::prelude::*;

#[cfg(feature = "gui")]
pub fn launch_gui() {
    // 确保在非async环境中启动GUI
    std::thread::spawn(|| {
        launch_gui_impl();
    }).join().unwrap();
}

#[cfg(feature = "gui")]
fn launch_gui_impl() {
    use dioxus::desktop::{Config, WindowBuilder};

    let window = WindowBuilder::new()
        .with_title("BurnCloud - 大模型本地部署平台")
        .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
        .with_resizable(true)
        .with_decorations(false);

    let config = Config::new().with_window(window);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(App);
}

#[cfg(not(feature = "gui"))]
pub fn launch_gui() {
    println!("GUI功能未启用，请使用命令行模式");
}

#[cfg(feature = "gui")]
pub fn App() -> Element {
    rsx! {
        div {
            style: "height: 100vh; background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);",
            div {
                style: "padding: 20px; color: white; text-align: center;",
                h1 { "🚀 BurnCloud" }
                h2 { "大模型本地部署平台" }
                p { "GUI界面开发中..." }

                div {
                    style: "margin-top: 40px;",
                    p { "功能模块:" }
                    ul {
                        style: "list-style: none; padding: 0;",
                        li { "📊 仪表板" }
                        li { "🤖 模型管理" }
                        li { "⚙️ 设置" }
                        li { "📈 监控" }
                        li { "🚀 部署" }
                    }
                }
            }
        }
    }
}