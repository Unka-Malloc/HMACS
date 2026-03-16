use crate::types::SignatureShare;
use hmacs_core::ParticipantId;
use sha2::{Digest, Sha256};

/// Verifiable Secret Sharing (VSS) check.
/// In a real FROST implementation this verifies the share against the
/// polynomial commitment. Here we implement a simplified structural check
/// that can be replaced with full cryptographic verification.
pub fn verify_share(share: &SignatureShare, message: &[u8]) -> VssResult {
    if share.share_bytes.is_empty() {
        return VssResult::Invalid {
            agent_id: share.agent_id,
            reason: "Empty share bytes".into(),
        };
    }

    if share.share_bytes.len() != 64 {
        return VssResult::Invalid {
            agent_id: share.agent_id,
            reason: format!(
                "Share must be 64 bytes (Ed25519), got {}",
                share.share_bytes.len()
            ),
        };
    }

    if share.vss_commitment.is_empty() {
        return VssResult::Invalid {
            agent_id: share.agent_id,
            reason: "Missing VSS commitment".into(),
        };
    }

    // Structural commitment check: the commitment should be a hash
    // binding the share to the message. In production, replace this
    // with proper Feldman/Pedersen VSS verification.
    let mut hasher = Sha256::new();
    hasher.update(&share.share_bytes);
    hasher.update(message);
    let expected_prefix = &hasher.finalize()[..4];

    if share.vss_commitment.len() < 4 || share.vss_commitment[..4] != *expected_prefix {
        return VssResult::Invalid {
            agent_id: share.agent_id,
            reason: "VSS commitment does not match share-message binding".into(),
        };
    }

    VssResult::Valid
}

#[derive(Debug, Clone)]
pub enum VssResult {
    Valid,
    Invalid { agent_id: ParticipantId, reason: String },
}

impl VssResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, VssResult::Valid)
    }
}

/// Generate a valid VSS commitment for a share (helper for testing/agents).
pub fn compute_vss_commitment(share_bytes: &[u8], message: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(share_bytes);
    hasher.update(message);
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_valid_share(message: &[u8]) -> SignatureShare {
        let share_bytes = vec![0xABu8; 64];
        let commitment = compute_vss_commitment(&share_bytes, message);
        SignatureShare {
            agent_id: ParticipantId::new(),
            share_bytes,
            vss_commitment: commitment,
            submitted_at: Utc::now(),
        }
    }

    #[test]
    fn test_valid_share() {
        let msg = b"sign this message";
        let share = make_valid_share(msg);
        assert!(verify_share(&share, msg).is_valid());
    }

    #[test]
    fn test_empty_share() {
        let share = SignatureShare {
            agent_id: ParticipantId::new(),
            share_bytes: vec![],
            vss_commitment: vec![1, 2, 3, 4],
            submitted_at: Utc::now(),
        };
        assert!(!verify_share(&share, b"msg").is_valid());
    }

    #[test]
    fn test_wrong_commitment() {
        let share = SignatureShare {
            agent_id: ParticipantId::new(),
            share_bytes: vec![0xABu8; 64],
            vss_commitment: vec![0xFF, 0xFF, 0xFF, 0xFF],
            submitted_at: Utc::now(),
        };
        assert!(!verify_share(&share, b"msg").is_valid());
    }

    #[test]
    fn test_wrong_share_length() {
        let share = SignatureShare {
            agent_id: ParticipantId::new(),
            share_bytes: vec![0xABu8; 32], // should be 64
            vss_commitment: vec![1, 2, 3, 4],
            submitted_at: Utc::now(),
        };
        assert!(!verify_share(&share, b"msg").is_valid());
    }
}
