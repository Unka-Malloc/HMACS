use crate::types::{MpcError, Signature, SignatureShare};
use crate::vss::{self, VssResult};
use chrono::Utc;
use carbide_core::ParticipantId;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, warn};

/// Channel-based share submission for use with the racing executor.
pub type ShareSender = mpsc::Sender<SignatureShare>;
pub type ShareReceiver = mpsc::Receiver<SignatureShare>;

/// Redundant racing execution for a t-of-n FROST signature task.
///
/// Waits for the first `threshold` valid signature shares from `agents`.
/// Agents that are slow or offline are NOT penalized — their futures are
/// simply dropped and their micro-stakes are returned.
///
/// Returns `(valid_shares, malicious_agents)` on success.
pub async fn execute_frost_race(
    mut share_rx: ShareReceiver,
    message: &[u8],
    threshold: usize,
    total_agents: usize,
    timeout: Duration,
) -> Result<RaceResult, MpcError> {
    let mut valid_shares: Vec<SignatureShare> = Vec::with_capacity(threshold);
    let mut malicious_agents: Vec<ParticipantId> = Vec::new();
    let mut received_count = 0usize;

    let deadline = tokio::time::Instant::now() + timeout;

    info!(
        threshold = threshold,
        total = total_agents,
        timeout_ms = timeout.as_millis() as u64,
        "FROST race started"
    );

    loop {
        if valid_shares.len() >= threshold {
            info!(
                valid = valid_shares.len(),
                malicious = malicious_agents.len(),
                "Threshold reached — collecting results"
            );
            break;
        }

        if received_count >= total_agents && valid_shares.len() < threshold {
            return Err(MpcError::InsufficientShares {
                have: valid_shares.len(),
                need: threshold,
            });
        }

        tokio::select! {
            share = share_rx.recv() => {
                match share {
                    Some(s) => {
                        received_count += 1;
                        match vss::verify_share(&s, message) {
                            VssResult::Valid => {
                                info!(agent = %s.agent_id, "Valid share received");
                                valid_shares.push(s);
                            }
                            VssResult::Invalid { agent_id, reason } => {
                                warn!(
                                    agent = %agent_id,
                                    reason = %reason,
                                    "MALICIOUS: Invalid share detected (VSS failure)"
                                );
                                malicious_agents.push(agent_id);
                            }
                        }
                    }
                    None => {
                        // Channel closed — all agents done
                        if valid_shares.len() < threshold {
                            return Err(MpcError::InsufficientShares {
                                have: valid_shares.len(),
                                need: threshold,
                            });
                        }
                        break;
                    }
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                warn!(
                    valid = valid_shares.len(),
                    threshold = threshold,
                    "FROST race timeout reached"
                );
                return Err(MpcError::Timeout {
                    task_id: carbide_core::MpcTaskId::new(),
                });
            }
        }
    }

    // Drop remaining pending — late agents are simply not rewarded.
    drop(share_rx);

    Ok(RaceResult {
        valid_shares,
        malicious_agents,
    })
}

/// Assemble individual shares into a final FROST signature.
/// In a real implementation, this performs Lagrange interpolation over the
/// Ed25519 curve. Here we simulate the aggregation.
pub fn assemble_signature(shares: &[SignatureShare]) -> Result<Signature, MpcError> {
    if shares.is_empty() {
        return Err(MpcError::AssemblyFailed("No shares to assemble".into()));
    }

    // Simulated aggregation: XOR all share bytes together
    let mut combined = vec![0u8; 64];
    for share in shares {
        for (i, byte) in share.share_bytes.iter().enumerate() {
            if i < 64 {
                combined[i] ^= byte;
            }
        }
    }

    Ok(Signature {
        bytes: combined,
        assembled_at: Utc::now(),
    })
}

#[derive(Debug)]
pub struct RaceResult {
    pub valid_shares: Vec<SignatureShare>,
    pub malicious_agents: Vec<ParticipantId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vss::compute_vss_commitment;

    fn make_share(agent_id: ParticipantId, message: &[u8], valid: bool) -> SignatureShare {
        let share_bytes = vec![0xABu8; 64];
        let commitment = if valid {
            compute_vss_commitment(&share_bytes, message)
        } else {
            vec![0xFF; 32]
        };
        SignatureShare {
            agent_id,
            share_bytes,
            vss_commitment: commitment,
            submitted_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_race_reaches_threshold() {
        let msg = b"test message";
        let (tx, rx) = mpsc::channel(10);

        let agents: Vec<ParticipantId> = (0..5).map(|_| ParticipantId::new()).collect();

        let handle = tokio::spawn(async move {
            execute_frost_race(rx, msg, 3, 5, Duration::from_secs(5)).await
        });

        // Send 3 valid shares
        for agent in &agents[..3] {
            tx.send(make_share(*agent, msg, true)).await.unwrap();
        }

        let result = handle.await.unwrap().unwrap();
        assert_eq!(result.valid_shares.len(), 3);
        assert!(result.malicious_agents.is_empty());
    }

    #[tokio::test]
    async fn test_race_detects_malicious_agent() {
        let msg = b"test message";
        let (tx, rx) = mpsc::channel(10);

        let honest = ParticipantId::new();
        let malicious = ParticipantId::new();

        let handle = tokio::spawn(async move {
            execute_frost_race(rx, msg, 1, 2, Duration::from_secs(5)).await
        });

        // Send invalid share first
        tx.send(make_share(malicious, msg, false)).await.unwrap();
        // Then valid
        tx.send(make_share(honest, msg, true)).await.unwrap();

        let result = handle.await.unwrap().unwrap();
        assert_eq!(result.valid_shares.len(), 1);
        assert_eq!(result.malicious_agents.len(), 1);
        assert_eq!(result.malicious_agents[0], malicious);
    }

    #[tokio::test]
    async fn test_race_timeout() {
        let msg = b"test message";
        let (_tx, rx) = mpsc::channel::<SignatureShare>(10);

        let result =
            execute_frost_race(rx, msg, 3, 5, Duration::from_millis(50)).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_assemble_signature() {
        let msg = b"test";
        let shares: Vec<SignatureShare> = (0..3)
            .map(|_| {
                let bytes = vec![0xAB; 64];
                let commitment = compute_vss_commitment(&bytes, msg);
                SignatureShare {
                    agent_id: ParticipantId::new(),
                    share_bytes: bytes,
                    vss_commitment: commitment,
                    submitted_at: Utc::now(),
                }
            })
            .collect();
        let sig = assemble_signature(&shares).unwrap();
        assert_eq!(sig.bytes.len(), 64);
    }
}
