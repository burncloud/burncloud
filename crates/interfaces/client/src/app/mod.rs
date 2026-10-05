#[expect(
    clippy::module_inception,
    reason = "the public `app::App` path is already used across desktop/liveview entrypoints; rename the nested module in a dedicated API-path migration rather than during warning cleanup"
)]
pub mod app;
pub mod bootstrap;
pub mod router;

pub use app::App;
