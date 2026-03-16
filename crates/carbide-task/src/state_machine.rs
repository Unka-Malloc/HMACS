use carbide_core::CarbideError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    Bidding,
    Assigned,
    InProgress,
    Delivered,
    Completed,
    Disputed,
    Resolved,
    Settled,
    Cancelled,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Open => "open",
            Self::Bidding => "bidding",
            Self::Assigned => "assigned",
            Self::InProgress => "in_progress",
            Self::Delivered => "delivered",
            Self::Completed => "completed",
            Self::Disputed => "disputed",
            Self::Resolved => "resolved",
            Self::Settled => "settled",
            Self::Cancelled => "cancelled",
        };
        write!(f, "{}", s)
    }
}

impl TaskStatus {
    /// Returns the set of valid next states from the current state.
    pub fn valid_transitions(&self) -> &[TaskStatus] {
        match self {
            Self::Open => &[Self::Bidding, Self::Cancelled],
            Self::Bidding => &[Self::Assigned, Self::Cancelled],
            Self::Assigned => &[Self::InProgress, Self::Cancelled],
            Self::InProgress => &[Self::Delivered],
            Self::Delivered => &[Self::Completed, Self::Disputed],
            Self::Completed => &[Self::Settled],
            Self::Disputed => &[Self::Resolved],
            Self::Resolved => &[Self::Settled],
            Self::Settled => &[],
            Self::Cancelled => &[],
        }
    }

    pub fn can_transition_to(&self, next: TaskStatus) -> bool {
        self.valid_transitions().contains(&next)
    }

    pub fn transition_to(&self, next: TaskStatus) -> Result<TaskStatus, CarbideError> {
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
    fn test_valid_transitions() {
        assert!(TaskStatus::Open.can_transition_to(TaskStatus::Bidding));
        assert!(TaskStatus::Open.can_transition_to(TaskStatus::Cancelled));
        assert!(!TaskStatus::Open.can_transition_to(TaskStatus::Completed));
    }

    #[test]
    fn test_full_lifecycle() {
        let mut status = TaskStatus::Open;
        status = status.transition_to(TaskStatus::Bidding).unwrap();
        status = status.transition_to(TaskStatus::Assigned).unwrap();
        status = status.transition_to(TaskStatus::InProgress).unwrap();
        status = status.transition_to(TaskStatus::Delivered).unwrap();
        status = status.transition_to(TaskStatus::Completed).unwrap();
        status = status.transition_to(TaskStatus::Settled).unwrap();
        assert_eq!(status, TaskStatus::Settled);
    }

    #[test]
    fn test_dispute_path() {
        let mut status = TaskStatus::Open;
        status = status.transition_to(TaskStatus::Bidding).unwrap();
        status = status.transition_to(TaskStatus::Assigned).unwrap();
        status = status.transition_to(TaskStatus::InProgress).unwrap();
        status = status.transition_to(TaskStatus::Delivered).unwrap();
        status = status.transition_to(TaskStatus::Disputed).unwrap();
        status = status.transition_to(TaskStatus::Resolved).unwrap();
        status = status.transition_to(TaskStatus::Settled).unwrap();
        assert_eq!(status, TaskStatus::Settled);
    }

    #[test]
    fn test_invalid_transition() {
        let result = TaskStatus::Open.transition_to(TaskStatus::Settled);
        assert!(result.is_err());
    }
}
