//! # BurnCloud Service Upstream
//!
//! Upstream service layer providing business logic for upstream/channel management.

use burncloud_database::Database;
use burncloud_database_upstream::UpstreamModel;

pub use burncloud_database_upstream::DbUpstream;

type Result<T> = std::result::Result<T, burncloud_database::DatabaseError>;

/// Upstream service for managing API upstreams/channels
pub struct UpstreamService;

impl UpstreamService {
    /// Get all upstreams
    pub async fn get_all(db: &Database) -> Result<Vec<DbUpstream>> {
        UpstreamModel::get_all(db).await
    }

    /// Get a single upstream by ID
    pub async fn get(db: &Database, id: &str) -> Result<Option<DbUpstream>> {
        UpstreamModel::get(db, id).await
    }

    /// Create a new upstream
    pub async fn create(db: &Database, upstream: &DbUpstream) -> Result<()> {
        UpstreamModel::create(db, upstream).await
    }

    /// Update an existing upstream
    pub async fn update(db: &Database, upstream: &DbUpstream) -> Result<()> {
        UpstreamModel::update(db, upstream).await
    }

    /// Delete an upstream
    pub async fn delete(db: &Database, id: &str) -> Result<()> {
        UpstreamModel::delete(db, id).await
    }
}
