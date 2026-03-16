use chrono::Utc;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use hmacs_core::HmacsError;
use serde::{Deserialize, Serialize};

/// A challenge-response authentication request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthChallenge {
    pub message: String,
    pub issued_at: i64,
    pub expires_at: i64,
}

impl AuthChallenge {
    pub fn new(domain: &str, nonce: &str, ttl_secs: i64) -> Self {
        let now = Utc::now().timestamp();
        let message = format!(
            "{domain} wants you to sign in.\n\nNonce: {nonce}\nIssued At: {now}",
        );
        Self {
            message,
            issued_at: now,
            expires_at: now + ttl_secs,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.expires_at
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSignInRequest {
    pub wallet_address: String,
    pub chain: String,
    pub message: String,
    pub signature: String,
}

/// Verify an Ed25519 signature (used for Solana wallets).
pub fn verify_ed25519_signature(
    pubkey_bytes: &[u8; 32],
    message: &[u8],
    sig_bytes: &[u8; 64],
) -> Result<(), HmacsError> {
    let verifying_key = VerifyingKey::from_bytes(pubkey_bytes)
        .map_err(|e| HmacsError::Unauthorized(format!("Invalid public key: {e}")))?;

    let signature = Signature::from_bytes(sig_bytes);

    verifying_key
        .verify(message, &signature)
        .map_err(|e| HmacsError::Unauthorized(format!("Invalid signature: {e}")))
}

/// Verify a Solana wallet sign-in request.
pub fn verify_solana_sign_in(req: &WalletSignInRequest) -> Result<(), HmacsError> {
    let pubkey_bytes: [u8; 32] = bs58::decode(&req.wallet_address)
        .into_vec()
        .map_err(|e| HmacsError::Unauthorized(format!("Invalid wallet address: {e}")))?
        .try_into()
        .map_err(|_| HmacsError::Unauthorized("Wallet address must be 32 bytes".to_string()))?;

    let sig_bytes: [u8; 64] = bs58::decode(&req.signature)
        .into_vec()
        .map_err(|e| HmacsError::Unauthorized(format!("Invalid signature encoding: {e}")))?
        .try_into()
        .map_err(|_| HmacsError::Unauthorized("Signature must be 64 bytes".to_string()))?;

    verify_ed25519_signature(&pubkey_bytes, req.message.as_bytes(), &sig_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_creation() {
        let challenge = AuthChallenge::new("hmacs.io", "abc123", 300);
        assert!(!challenge.is_expired());
        assert!(challenge.message.contains("hmacs.io"));
        assert!(challenge.message.contains("abc123"));
    }

    #[test]
    fn test_expired_challenge() {
        let challenge = AuthChallenge {
            message: "test".to_string(),
            issued_at: 0,
            expires_at: 1,
        };
        assert!(challenge.is_expired());
    }
}
