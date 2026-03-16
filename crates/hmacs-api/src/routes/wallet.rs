use axum::extract::State;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use hmacs_core::AssetSymbol;
use hmacs_wallet::{Balance, LedgerEntry};
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthenticatedParticipant;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/balances", get(get_balances))
        .route("/deposit", post(deposit))
        .route("/withdraw", post(withdraw))
        .route("/ledger", get(get_ledger))
        .with_state(state)
}

async fn get_balances(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
) -> ApiResult<Json<Vec<Balance>>> {
    let balances = state.wallet_service.get_all_balances(auth.participant_id);
    Ok(Json(balances))
}

#[derive(Debug, Deserialize)]
struct DepositRequest {
    asset: AssetSymbol,
    amount: Decimal,
    tx_hash: Option<String>,
}

async fn deposit(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Json(req): Json<DepositRequest>,
) -> ApiResult<Json<Balance>> {
    let balance = state
        .wallet_service
        .deposit(auth.participant_id, req.asset, req.amount, req.tx_hash)
        .map_err(ApiError)?;
    Ok(Json(balance))
}

#[derive(Debug, Deserialize)]
struct WithdrawRequest {
    asset: AssetSymbol,
    amount: Decimal,
    to_address: String,
}

async fn withdraw(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Json(req): Json<WithdrawRequest>,
) -> ApiResult<Json<Balance>> {
    // In production, this would initiate an on-chain withdrawal via hmacs-settlement
    let balance = state
        .wallet_service
        .withdraw(
            auth.participant_id,
            req.asset,
            req.amount,
            Some(req.to_address),
        )
        .map_err(ApiError)?;
    Ok(Json(balance))
}

async fn get_ledger(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
) -> ApiResult<Json<Vec<LedgerEntry>>> {
    let entries = state.wallet_service.get_ledger_entries(auth.participant_id);
    Ok(Json(entries))
}
