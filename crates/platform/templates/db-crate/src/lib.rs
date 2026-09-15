//! # burncloud-{{crate_name}}
//!
//! SQLx persistence for the `{{crate_name}}` domain type.
//!
//! ## Architecture rules (enforced by cargo-deny in Wave 3)
//! - MUST implement `burncloud_common::CrudRepository` for its primary domain type.
//! - MUST NOT be depended on directly by `burncloud-server` — only by service crates.
//! - All public types MUST be concrete structs, NOT `serde_json::Value`.

use async_trait::async_trait;
use burncloud_common::CrudRepository;
use burncloud_database::Database;
use thiserror::Error;

/// Errors produced by the {{crate_name}} database layer
#[derive(Debug, Error)]
pub enum {{crate_name | pascal_case}}DatabaseError {
    #[error("Database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Not found")]
    NotFound,
}

/// Primary domain model for {{crate_name}}
///
/// Replace fields with the actual domain model.
#[derive(Debug, Clone)]
pub struct Db{{crate_name | pascal_case}} {
    pub id: String,
    // TODO: add domain-specific fields
}

/// Repository implementation backed by SQLx
pub struct {{crate_name | pascal_case}}Repository {
    db: Database,
}

impl {{crate_name | pascal_case}}Repository {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl CrudRepository<Db{{crate_name | pascal_case}}, String, {{crate_name | pascal_case}}DatabaseError>
    for {{crate_name | pascal_case}}Repository
{
    async fn find_by_id(
        &self,
        id: &String,
    ) -> Result<Option<Db{{crate_name | pascal_case}}>, {{crate_name | pascal_case}}DatabaseError> {
        todo!("implement find_by_id for {{crate_name}}")
    }

    async fn list(&self) -> Result<Vec<Db{{crate_name | pascal_case}}>, {{crate_name | pascal_case}}DatabaseError> {
        todo!("implement list for {{crate_name}}")
    }

    async fn create(
        &self,
        input: &Db{{crate_name | pascal_case}},
    ) -> Result<Db{{crate_name | pascal_case}}, {{crate_name | pascal_case}}DatabaseError> {
        todo!("implement create for {{crate_name}}")
    }

    async fn update(
        &self,
        id: &String,
        input: &Db{{crate_name | pascal_case}},
    ) -> Result<bool, {{crate_name | pascal_case}}DatabaseError> {
        todo!("implement update for {{crate_name}}")
    }

    async fn delete(&self, id: &String) -> Result<bool, {{crate_name | pascal_case}}DatabaseError> {
        todo!("implement delete for {{crate_name}}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_test() {
        // Replace with integration tests against a real test database.
    }
}
