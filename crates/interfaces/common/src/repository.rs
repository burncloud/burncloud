/// Generic CRUD repository trait that all database crates must implement.
///
/// # Architecture rule (enforced by cargo-deny in Wave 3)
/// Every `burncloud-database-*` crate must implement this trait for its primary domain type.
/// This prevents direct database access from the service or server layers — callers depend
/// on this trait, not on the concrete database crate.
///
/// # Example
/// ```rust,ignore
/// use burncloud_common::CrudRepository;
///
/// pub struct UserRepository { /* ... */ }
///
/// #[async_trait::async_trait]
/// impl CrudRepository<UserAccount, String, DatabaseError> for UserRepository {
///     async fn find_by_id(&self, id: &String) -> Result<Option<UserAccount>, DatabaseError> { todo!() }
///     async fn list(&self) -> Result<Vec<UserAccount>, DatabaseError> { todo!() }
///     async fn create(&self, input: &UserAccount) -> Result<UserAccount, DatabaseError> { todo!() }
///     async fn update(&self, id: &String, input: &UserAccount) -> Result<bool, DatabaseError> { todo!() }
///     async fn delete(&self, id: &String) -> Result<bool, DatabaseError> { todo!() }
/// }
/// ```
#[async_trait::async_trait]
pub trait CrudRepository<T, Id, Error>: Send + Sync
where
    T: Send + Sync,
    Id: Send + Sync,
    Error: Send,
{
    /// Fetch a single record by its ID. Returns `None` if not found.
    async fn find_by_id(&self, id: &Id) -> Result<Option<T>, Error>;

    /// List all records. Use sparingly in production — prefer paginated queries.
    async fn list(&self) -> Result<Vec<T>, Error>;

    /// Persist a new record and return the created entity.
    async fn create(&self, input: &T) -> Result<T, Error>;

    /// Update an existing record by ID. Returns `true` if the record was found and updated.
    async fn update(&self, id: &Id, input: &T) -> Result<bool, Error>;

    /// Delete a record by ID. Returns `true` if the record was found and removed.
    async fn delete(&self, id: &Id) -> Result<bool, Error>;
}
