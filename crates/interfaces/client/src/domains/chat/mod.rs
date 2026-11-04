//! Chat 首页的领域模块。

mod backend;
mod catalog;
mod model;
mod page;
mod selector;
mod state;

#[cfg(test)]
mod tests;

pub use page::ChatHome;
