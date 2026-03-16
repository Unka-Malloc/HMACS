use async_trait::async_trait;
use carbide_core::{CarbideResult, PaginationParams};

/// Generic repository trait for CRUD operations.
#[async_trait]
pub trait Repository<T, Id, Create>: Send + Sync {
    async fn get_by_id(&self, id: Id) -> CarbideResult<T>;
    async fn create(&self, input: Create) -> CarbideResult<T>;
    async fn delete(&self, id: Id) -> CarbideResult<()>;
    async fn list(&self, params: &PaginationParams) -> CarbideResult<(Vec<T>, u64)>;
}
