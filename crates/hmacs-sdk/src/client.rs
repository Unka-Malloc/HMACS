use hmacs_core::{HmacsError, HmacsResult};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Configuration for connecting to the HMACS platform.
#[derive(Debug, Clone)]
pub struct HmacsClientConfig {
    pub grpc_endpoint: String,
    pub api_key: Option<String>,
    pub jwt_token: Option<String>,
    /// Retry count for transient failures
    pub max_retries: u32,
    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl Default for HmacsClientConfig {
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

/// The HMACS client that agents use to interact with the platform.
pub struct HmacsClient {
    config: HmacsClientConfig,
}

impl HmacsClient {
    pub fn new(config: HmacsClientConfig) -> Self {
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

    /// Connect to the HMACS platform.
    pub async fn connect(&self) -> HmacsResult<()> {
        info!(endpoint = %self.config.grpc_endpoint, "Connecting to HMACS platform");
        // TODO: Establish gRPC channel via tonic
        Ok(())
    }

    // Task Market operations
    pub async fn create_task(&self, _request: CreateTaskRequest) -> HmacsResult<TaskResponse> {
        Err(HmacsError::Internal("gRPC client not yet implemented".into()))
    }

    pub async fn list_tasks(&self) -> HmacsResult<Vec<TaskResponse>> {
        Err(HmacsError::Internal("gRPC client not yet implemented".into()))
    }

    pub async fn place_bid(&self, _request: PlaceBidRequest) -> HmacsResult<BidResponse> {
        Err(HmacsError::Internal("gRPC client not yet implemented".into()))
    }

    // Compute Market operations
    pub async fn register_resource(
        &self,
        _request: RegisterResourceRequest,
    ) -> HmacsResult<ResourceResponse> {
        Err(HmacsError::Internal("gRPC client not yet implemented".into()))
    }

    pub async fn report_usage(
        &self,
        _lease_id: &str,
        _units: &str,
    ) -> HmacsResult<LeaseResponse> {
        Err(HmacsError::Internal("gRPC client not yet implemented".into()))
    }

    // Wallet operations
    pub async fn get_balances(&self) -> HmacsResult<Vec<BalanceResponse>> {
        Err(HmacsError::Internal("gRPC client not yet implemented".into()))
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
