//! # BurnCloud Service Upstream
//!
//! Upstream service layer providing business logic for upstream/channel management.

use burncloud_database::Database;
use burncloud_database_router::upstream::RouterUpstreamModel;

pub use burncloud_database_router::upstream::RouterUpstream;

type Result<T> = std::result::Result<T, burncloud_database::DatabaseError>;

/// Upstream service for managing API upstreams/channels
pub struct UpstreamService;

impl UpstreamService {
    /// Get all upstreams
    pub async fn get_all(db: &Database) -> Result<Vec<RouterUpstream>> {
        RouterUpstreamModel::get_all(db).await
    }

    /// Get a single upstream by ID
    pub async fn get(db: &Database, id: &str) -> Result<Option<RouterUpstream>> {
        RouterUpstreamModel::get(db, id).await
    }

    /// Create a new upstream
    pub async fn create(db: &Database, upstream: &RouterUpstream) -> Result<()> {
        RouterUpstreamModel::create(db, upstream).await
    }

    /// Update an existing upstream
    pub async fn update(db: &Database, upstream: &RouterUpstream) -> Result<()> {
        RouterUpstreamModel::update(db, upstream).await
    }

    /// Delete an upstream
    pub async fn delete(db: &Database, id: &str) -> Result<()> {
        RouterUpstreamModel::delete(db, id).await
    }
}
