use carbide_core::CarbideResult;
use serde::{Deserialize, Serialize};

/// Authentication credentials for the SDK.
#[derive(Debug, Clone)]
pub enum SdkAuth {
    ApiKey(String),
    JwtToken(String),
}

impl SdkAuth {
    pub fn api_key(key: &str) -> Self {
        Self::ApiKey(key.to_string())
    }

    pub fn jwt(token: &str) -> Self {
        Self::JwtToken(token.to_string())
    }

    pub fn authorization_header(&self) -> String {
        match self {
            Self::ApiKey(key) => format!("Bearer {}", key),
            Self::JwtToken(token) => format!("Bearer {}", token),
        }
    }
}

/// Request to sign in via wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSignIn {
    pub wallet_address: String,
    pub chain: String,
    pub message: String,
    pub signature: String,
}

/// Response from wallet sign-in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignInResult {
    pub token: String,
    pub participant_id: String,
}

/// Convenience: sign a message with a given keypair (for Solana Ed25519).
pub fn sign_message_ed25519(keypair_bytes: &[u8; 64], message: &[u8]) -> CarbideResult<[u8; 64]> {
    use ed25519_dalek::{Signer, SigningKey};
    let signing_key = SigningKey::from_keypair_bytes(keypair_bytes)
        .map_err(|e| carbide_core::CarbideError::Internal(format!("Invalid keypair: {}", e)))?;
    let signature = signing_key.sign(message);
    Ok(signature.to_bytes())
}
