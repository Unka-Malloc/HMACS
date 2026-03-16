use async_trait::async_trait;
use hmacs_core::{HmacsResult, PaginationParams};

/// Generic repository trait for CRUD operations.
#[async_trait]
pub trait Repository<T, Id, Create>: Send + Sync {
    async fn get_by_id(&self, id: Id) -> HmacsResult<T>;
    async fn create(&self, input: Create) -> HmacsResult<T>;
    async fn delete(&self, id: Id) -> HmacsResult<()>;
    async fn list(&self, params: &PaginationParams) -> HmacsResult<(Vec<T>, u64)>;
}
