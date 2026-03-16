use chrono::{Duration, Utc};
use carbide_core::{CarbideError, ParticipantId, ParticipantKind};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub participant_id: String,
    pub kind: String,
    pub iat: i64,
    pub exp: i64,
}

pub struct JwtConfig {
    pub secret: String,
    pub token_ttl_hours: i64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "carbide-dev-secret-change-in-production".to_string(),
            token_ttl_hours: 24,
        }
    }
}

pub fn create_token(
    config: &JwtConfig,
    participant_id: ParticipantId,
    kind: ParticipantKind,
) -> Result<String, CarbideError> {
    let now = Utc::now();
    let exp = now + Duration::hours(config.token_ttl_hours);

    let claims = Claims {
        sub: participant_id.to_string(),
        participant_id: participant_id.to_string(),
        kind: kind.to_string(),
        iat: now.timestamp(),
        exp: exp.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.secret.as_bytes()),
    )
    .map_err(|e| CarbideError::Internal(format!("JWT encoding error: {e}")))
}

pub fn verify_token(config: &JwtConfig, token: &str) -> Result<Claims, CarbideError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.secret.as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| CarbideError::Unauthorized(format!("Invalid token: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_roundtrip() {
        let config = JwtConfig::default();
        let pid = ParticipantId::new();
        let token = create_token(&config, pid, ParticipantKind::Human).unwrap();
        let claims = verify_token(&config, &token).unwrap();
        assert_eq!(claims.participant_id, pid.to_string());
        assert_eq!(claims.kind, "human");
    }
}
