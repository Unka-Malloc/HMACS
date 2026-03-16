use chrono::{DateTime, Utc};
use hmacs_core::{ApiKeyId, HmacsError, ParticipantId};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const API_KEY_PREFIX: &str = "hmacs_";
const API_KEY_RANDOM_BYTES: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: ApiKeyId,
    pub participant_id: ParticipantId,
    /// Only the hash is stored; the raw key is returned once at creation time.
    pub key_hash: String,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub label: String,
    pub expires_in_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyResponse {
    pub id: ApiKeyId,
    pub raw_key: String,
    pub label: String,
    pub expires_at: Option<DateTime<Utc>>,
}

pub fn generate_api_key() -> String {
    let random_bytes: Vec<u8> = (0..API_KEY_RANDOM_BYTES)
        .map(|_| rand::thread_rng().gen())
        .collect();
    format!("{}{}", API_KEY_PREFIX, hex::encode(random_bytes))
}

pub fn hash_api_key(raw_key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn verify_api_key(raw_key: &str, stored_hash: &str) -> Result<(), HmacsError> {
    let computed_hash = hash_api_key(raw_key);
    if computed_hash == stored_hash {
        Ok(())
    } else {
        Err(HmacsError::Unauthorized("Invalid API key".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_generation() {
        let key = generate_api_key();
        assert!(key.starts_with("hmacs_"));
        assert_eq!(key.len(), API_KEY_PREFIX.len() + API_KEY_RANDOM_BYTES * 2);
    }

    #[test]
    fn test_key_hash_verify() {
        let key = generate_api_key();
        let hash = hash_api_key(&key);
        assert!(verify_api_key(&key, &hash).is_ok());
        assert!(verify_api_key("wrong_key", &hash).is_err());
    }
}
