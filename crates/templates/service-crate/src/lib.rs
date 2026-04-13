//! # burncloud-service-{{crate_name}}
//!
//! Business logic layer for the `{{crate_name}}` domain.
//!
//! ## Architecture rules
//! - This crate MUST NOT depend on any `burncloud-database-*` crate directly.
//! - Database access goes through a `CrudRepository` trait from `burncloud-common`.
//! - Handlers in `burncloud-server` call this crate; this crate calls the repository trait.

use burncloud_common::CrudRepository;
use thiserror::Error;

/// Errors produced by the {{crate_name}} service
#[derive(Debug, Error)]
pub enum {{crate_name | pascal_case}}ServiceError {
    #[error("Not found")]
    NotFound,
    #[error("Database error: {0}")]
    Database(String),
}

/// Service for {{crate_name}} operations.
///
/// Inject a `CrudRepository` implementation at construction time.
pub struct {{crate_name | pascal_case}}Service<R> {
    repo: R,
}

impl<T, Id, R> {{crate_name | pascal_case}}Service<R>
where
    R: CrudRepository<T, Id, {{crate_name | pascal_case}}ServiceError> + Send + Sync,
    T: Send + Sync,
    Id: Send + Sync,
{
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder_test() {
        // Replace with real tests covering the service's core logic paths.
    }
}
