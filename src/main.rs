mod app;
mod components;
mod pages;
mod styles;

use app::App;

fn main() {
    use dioxus::desktop::{Config, WindowBuilder};

    let window = WindowBuilder::new()
        .with_title("BurnCloud - 大模型本地部署平台")
        .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
        .with_resizable(true);

    let config = Config::new().with_window(window);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(App);
}