use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use carbide_core::{ParticipantId, ParticipantKind};
use carbide_identity::{create_token, verify_solana_sign_in, WalletSignInRequest};
use serde::{Deserialize, Serialize};

use crate::error::{ApiError, ApiResult};
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/wallet-sign-in", post(wallet_sign_in))
        .with_state(state)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignInResponse {
    pub token: String,
    pub participant_id: String,
}

async fn wallet_sign_in(
    State(state): State<AppState>,
    Json(req): Json<WalletSignInRequest>,
) -> ApiResult<Json<SignInResponse>> {
    if req.chain.to_lowercase() == "solana" {
        verify_solana_sign_in(&req).map_err(ApiError)?;
    }
    // TODO: EVM signature verification (EIP-191 / EIP-712)

    // For now, generate a participant ID from the wallet address
    // In production, look up or create the participant record
    let participant_id = ParticipantId::new();
    let token = create_token(&state.jwt_config, participant_id, ParticipantKind::Human)
        .map_err(ApiError)?;

    Ok(Json(SignInResponse {
        token,
        participant_id: participant_id.to_string(),
    }))
}
