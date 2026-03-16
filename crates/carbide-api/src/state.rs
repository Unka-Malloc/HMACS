use carbide_compliance::ComplianceService;
use carbide_compute::ComputeService;
use carbide_engine::OrderBookManager;
use carbide_engine::MatchingEngine;
use carbide_identity::{DelegationService, JwtConfig};
use carbide_mpc::MpcService;
use carbide_task::TaskService;
use carbide_wallet::WalletService;
use std::sync::Arc;

/// Shared application state accessible from all route handlers.
#[derive(Clone)]
pub struct AppState {
    pub task_service: Arc<TaskService>,
    pub compute_service: Arc<ComputeService>,
    pub wallet_service: Arc<WalletService>,
    pub matching_engine: Arc<MatchingEngine>,
    pub compliance_service: Arc<ComplianceService>,
    pub delegation_service: Arc<DelegationService>,
    pub mpc_service: Arc<MpcService>,
    pub jwt_config: Arc<JwtConfig>,
}

impl AppState {
    pub fn new() -> Self {
        let order_books = Arc::new(OrderBookManager::new());
        let wallet = Arc::new(WalletService::new());
        Self {
            task_service: Arc::new(TaskService::new()),
            compute_service: Arc::new(ComputeService::new()),
            wallet_service: wallet.clone(),
            matching_engine: Arc::new(MatchingEngine::new(order_books)),
            compliance_service: Arc::new(ComplianceService::default()),
            delegation_service: Arc::new(DelegationService::new()),
            mpc_service: Arc::new(MpcService::new(wallet)),
            jwt_config: Arc::new(JwtConfig::default()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
