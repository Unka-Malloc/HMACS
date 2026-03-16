use thiserror::Error;

#[derive(Debug, Error)]
pub enum HmacsError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Insufficient balance: need {needed}, have {available}")]
    InsufficientBalance {
        needed: String,
        available: String,
    },

    #[error("Invalid state transition: cannot move from {from} to {to}")]
    InvalidStateTransition {
        from: String,
        to: String,
    },

    #[error("Settlement error: {0}")]
    SettlementError(String),

    #[error("Chain error ({chain}): {message}")]
    ChainError {
        chain: String,
        message: String,
    },

    #[error("Compliance blocked: {reason} (jurisdiction: {jurisdiction})")]
    ComplianceBlocked {
        reason: String,
        jurisdiction: String,
    },

    #[error("KYC required: {0}")]
    KycRequired(String),

    #[error("Sanctions match: {0}")]
    SanctionsMatch(String),

    #[error("Travel rule violation: {0}")]
    TravelRuleViolation(String),

    #[error("Agent fund transfer forbidden: agents cannot initiate fund operations")]
    AgentFundTransferForbidden,

    #[error("Funds quarantined: {0}")]
    FundsQuarantined(String),

    #[error("Agent blacklisted: {0}")]
    AgentBlacklisted(String),

    #[error("MPC error: {0}")]
    MpcError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl HmacsError {
    pub fn not_found(entity: &str, id: impl std::fmt::Display) -> Self {
        Self::NotFound(format!("{} with id {} not found", entity, id))
    }

    pub fn status_code(&self) -> u16 {
        match self {
            Self::NotFound(_) => 404,
            Self::AlreadyExists(_) => 409,
            Self::InvalidInput(_) => 400,
            Self::Unauthorized(_) => 401,
            Self::Forbidden(_) => 403,
            Self::InsufficientBalance { .. } => 422,
            Self::InvalidStateTransition { .. } => 422,
            Self::ComplianceBlocked { .. } => 451,
            Self::KycRequired(_) => 403,
            Self::SanctionsMatch(_) => 451,
            Self::TravelRuleViolation(_) => 422,
            Self::AgentFundTransferForbidden => 403,
            Self::FundsQuarantined(_) => 423,
            Self::AgentBlacklisted(_) => 403,
            Self::MpcError(_) => 500,
            Self::SettlementError(_) => 502,
            Self::ChainError { .. } => 502,
            Self::DatabaseError(_) => 500,
            Self::Internal(_) => 500,
        }
    }
}

pub type HmacsResult<T> = Result<T, HmacsError>;

impl From<sqlx::Error> for HmacsError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => Self::NotFound("Record not found".to_string()),
            _ => Self::DatabaseError(err.to_string()),
        }
    }
}
