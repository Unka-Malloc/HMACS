use carbide_core::CarbideError;
use serde::{Deserialize, Serialize};

/// MPC task state machine per the dev guide:
/// Open → Racing → Aggregating → Settled
///                               → Failed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MpcTaskState {
    /// Task posted to the order book, awaiting agent stake-locks.
    Open,
    /// n agents have locked micro-stakes; signature share collection in progress.
    Racing,
    /// t-of-n valid shares received; assembling the final signature.
    Aggregating,
    /// Final signature assembled and verified; bounty distributed.
    Settled,
    /// Timeout or irrecoverable failure; stakes unlocked, task may be retried.
    Failed,
}

impl std::fmt::Display for MpcTaskState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Open => "open",
            Self::Racing => "racing",
            Self::Aggregating => "aggregating",
            Self::Settled => "settled",
            Self::Failed => "failed",
        };
        write!(f, "{}", s)
    }
}

impl MpcTaskState {
    pub fn valid_transitions(&self) -> &[MpcTaskState] {
        match self {
            Self::Open => &[Self::Racing, Self::Failed],
            Self::Racing => &[Self::Aggregating, Self::Failed],
            Self::Aggregating => &[Self::Settled, Self::Failed],
            Self::Settled => &[],
            Self::Failed => &[Self::Open], // retry → new Open task
        }
    }

    pub fn can_transition_to(&self, next: MpcTaskState) -> bool {
        self.valid_transitions().contains(&next)
    }

    pub fn transition_to(&self, next: MpcTaskState) -> Result<MpcTaskState, CarbideError> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(CarbideError::InvalidStateTransition {
                from: self.to_string(),
                to: next.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_happy_path() {
        let mut s = MpcTaskState::Open;
        s = s.transition_to(MpcTaskState::Racing).unwrap();
        s = s.transition_to(MpcTaskState::Aggregating).unwrap();
        s = s.transition_to(MpcTaskState::Settled).unwrap();
        assert_eq!(s, MpcTaskState::Settled);
    }

    #[test]
    fn test_failure_from_racing() {
        let s = MpcTaskState::Racing;
        let s = s.transition_to(MpcTaskState::Failed).unwrap();
        assert_eq!(s, MpcTaskState::Failed);
    }

    #[test]
    fn test_retry_from_failed() {
        let s = MpcTaskState::Failed;
        let s = s.transition_to(MpcTaskState::Open).unwrap();
        assert_eq!(s, MpcTaskState::Open);
    }

    #[test]
    fn test_settled_is_terminal() {
        let s = MpcTaskState::Settled;
        assert!(s.transition_to(MpcTaskState::Open).is_err());
        assert!(s.valid_transitions().is_empty());
    }
}
