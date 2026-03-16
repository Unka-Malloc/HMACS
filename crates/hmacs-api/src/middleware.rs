use axum::extract::Request;
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use hmacs_core::{HmacsError, ParticipantId};
use hmacs_identity::{verify_token, Claims, JwtConfig};
use std::sync::Arc;

use crate::error::ApiError;

/// Extract participant info from JWT claims stored in request extensions.
#[derive(Debug, Clone)]
pub struct AuthenticatedParticipant {
    pub participant_id: ParticipantId,
    pub kind: String,
    pub claims: Claims,
}

/// Auth middleware that validates JWT Bearer tokens.
pub async fn auth_middleware(
    jwt_config: Arc<JwtConfig>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| {
            ApiError(HmacsError::Unauthorized(
                "Missing Authorization header".into(),
            ))
        })?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| {
            ApiError(HmacsError::Unauthorized(
                "Authorization header must use Bearer scheme".into(),
            ))
        })?;

    let claims = verify_token(&jwt_config, token).map_err(ApiError)?;

    let participant_id: uuid::Uuid = claims
        .participant_id
        .parse()
        .map_err(|_| ApiError(HmacsError::Unauthorized("Invalid participant ID in token".into())))?;

    let auth = AuthenticatedParticipant {
        participant_id: ParticipantId::from(participant_id),
        kind: claims.kind.clone(),
        claims,
    };

    request.extensions_mut().insert(auth);
    Ok(next.run(request).await)
}
