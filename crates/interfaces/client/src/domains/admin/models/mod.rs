#[path = "../model/actions.rs"]
pub mod actions;
#[path = "../model/components/mod.rs"]
pub mod components;
pub mod model;
pub mod page;
#[path = "../model/state.rs"]
pub mod state;
#[cfg(test)]
mod tests;

pub use page::AdminModels;
