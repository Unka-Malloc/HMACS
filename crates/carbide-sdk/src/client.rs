use carbide_core::{CarbideError, CarbideResult};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Configuration for connecting to the Carbide platform.
#[derive(Debug, Clone)]
pub struct CarbideClientConfig {
    pub grpc_endpoint: String,
    pub api_key: Option<String>,
    pub jwt_token: Option<String>,
    /// Retry count for transient failures
    pub max_retries: u32,
    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl Default for CarbideClientConfig {
    fn default() -> Self {
        Self {
            grpc_endpoint: "http://localhost:50051".into(),
            api_key: None,
            jwt_token: None,
            max_retries: 3,
            timeout_secs: 30,
        }
    }
}

/// The Carbide client that agents use to interact with the platform.
pub struct CarbideClient {
    config: CarbideClientConfig,
}

impl CarbideClient {
    pub fn new(config: CarbideClientConfig) -> Self {
        Self { config }
    }

    /// Connect using API key authentication.
    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.config.api_key = Some(api_key.to_string());
        self
    }

    /// Connect using JWT token authentication.
    pub fn with_token(mut self, token: &str) -> Self {
        self.config.jwt_token = Some(token.to_string());
        self
    }

    pub fn endpoint(&self) -> &str {
        &self.config.grpc_endpoint
    }

    /// Connect to the Carbide platform.
    pub async fn connect(&self) -> CarbideResult<()> {
        info!(endpoint = %self.config.grpc_endpoint, "Connecting to Carbide platform");
        // TODO: Establish gRPC channel via tonic
        Ok(())
    }

    // Task Market operations
    pub async fn create_task(&self, _request: CreateTaskRequest) -> CarbideResult<TaskResponse> {
        Err(CarbideError::Internal("gRPC client not yet implemented".into()))
    }

    pub async fn list_tasks(&self) -> CarbideResult<Vec<TaskResponse>> {
        Err(CarbideError::Internal("gRPC client not yet implemented".into()))
    }

    pub async fn place_bid(&self, _request: PlaceBidRequest) -> CarbideResult<BidResponse> {
        Err(CarbideError::Internal("gRPC client not yet implemented".into()))
    }

    // Compute Market operations
    pub async fn register_resource(
        &self,
        _request: RegisterResourceRequest,
    ) -> CarbideResult<ResourceResponse> {
        Err(CarbideError::Internal("gRPC client not yet implemented".into()))
    }

    pub async fn report_usage(
        &self,
        _lease_id: &str,
        _units: &str,
    ) -> CarbideResult<LeaseResponse> {
        Err(CarbideError::Internal("gRPC client not yet implemented".into()))
    }

    // Wallet operations
    pub async fn get_balances(&self) -> CarbideResult<Vec<BalanceResponse>> {
        Err(CarbideError::Internal("gRPC client not yet implemented".into()))
    }
}

// SDK request/response types (mirrors proto but native Rust)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
    pub skill_tags: Vec<String>,
    pub budget_asset: String,
    pub budget_amount: String,
    pub restriction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub id: String,
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceBidRequest {
    pub task_id: String,
    pub amount: String,
    pub asset: String,
    pub proposal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidResponse {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterResourceRequest {
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub gpu_model: Option<String>,
    pub gpu_vram_gb: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceResponse {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseResponse {
    pub id: String,
    pub units_consumed: String,
    pub total_cost: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalanceResponse {
    pub asset: String,
    pub available: String,
    pub frozen: String,
}
