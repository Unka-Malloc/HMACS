use chrono::{DateTime, Utc};
use carbide_core::{ConsentRecordId, ErasureRequestId, ParticipantId};
use serde::{Deserialize, Serialize};

use crate::jurisdiction::Jurisdiction;

/// Categories of personal data processed by the platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataCategory {
    /// Identity data (name, DOB, nationality)
    Identity,
    /// Contact data (email, phone)
    Contact,
    /// Financial data (balances, transactions)
    Financial,
    /// KYC documents (ID scans, proofs)
    KycDocuments,
    /// Behavioral data (login history, usage patterns)
    Behavioral,
    /// Technical data (IP addresses, device info)
    Technical,
    /// Wallet addresses
    WalletAddresses,
}

/// Legal basis for data processing (GDPR Article 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegalBasis {
    /// User explicitly consented
    Consent,
    /// Necessary for contract performance
    ContractPerformance,
    /// Compliance with legal obligation (AML/KYC)
    LegalObligation,
    /// Legitimate interests of the controller
    LegitimateInterest,
    /// Vital interests of the data subject
    VitalInterest,
}

/// A recorded consent from a participant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentRecord {
    pub id: ConsentRecordId,
    pub participant_id: ParticipantId,
    pub data_category: DataCategory,
    pub legal_basis: LegalBasis,
    pub purpose: String,
    pub granted: bool,
    pub granted_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    /// IP address or agent ID from which consent was given
    pub source: Option<String>,
    pub jurisdiction: Jurisdiction,
}

impl ConsentRecord {
    pub fn is_active(&self) -> bool {
        self.granted && self.revoked_at.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErasureStatus {
    Requested,
    InProgress,
    PartiallyCompleted,
    Completed,
    Rejected,
}

/// Reason an erasure request may be rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErasureRejectionReason {
    /// Legal retention period not yet elapsed (AML: 5-6 years)
    LegalRetention,
    /// Active open transactions
    ActiveTransactions,
    /// Ongoing investigation
    OngoingInvestigation,
    /// Data required for legal claims
    LegalClaims,
}

/// A data erasure (right-to-be-forgotten) request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErasureRequest {
    pub id: ErasureRequestId,
    pub participant_id: ParticipantId,
    pub status: ErasureStatus,
    pub categories_requested: Vec<DataCategory>,
    pub categories_completed: Vec<DataCategory>,
    pub categories_retained: Vec<(DataCategory, ErasureRejectionReason)>,
    pub requested_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub processor_notes: Option<String>,
}

/// Minimum data retention periods per jurisdiction (in days).
/// AML laws require keeping records for 5-6 years.
pub fn min_retention_days(jurisdiction: Jurisdiction) -> u32 {
    match jurisdiction {
        Jurisdiction::Us => 1825,  // 5 years (BSA)
        Jurisdiction::Eu => 1825,  // 5 years (AMLD)
        Jurisdiction::Hk => 2190,  // 6 years (AMLO)
        Jurisdiction::Sg => 1825,  // 5 years (MAS)
        Jurisdiction::Ae => 1825,  // 5 years (VARA)
    }
}

/// Determine which data categories can be erased and which must be retained.
pub fn evaluate_erasure_request(
    participant_id: ParticipantId,
    categories: &[DataCategory],
    jurisdiction: Jurisdiction,
    account_created_at: DateTime<Utc>,
    has_open_transactions: bool,
    under_investigation: bool,
) -> ErasureRequest {
    let now = Utc::now();
    let retention_days = min_retention_days(jurisdiction);
    let account_age_days = (now - account_created_at).num_days() as u32;
    let retention_elapsed = account_age_days >= retention_days;

    let mut completed = Vec::new();
    let mut retained = Vec::new();

    for &cat in categories {
        // Legal obligation data cannot be erased during retention period
        let is_legal_data = matches!(
            cat,
            DataCategory::Identity
                | DataCategory::Financial
                | DataCategory::KycDocuments
                | DataCategory::WalletAddresses
        );

        if under_investigation {
            retained.push((cat, ErasureRejectionReason::OngoingInvestigation));
        } else if is_legal_data && !retention_elapsed {
            retained.push((cat, ErasureRejectionReason::LegalRetention));
        } else if is_legal_data && has_open_transactions {
            retained.push((cat, ErasureRejectionReason::ActiveTransactions));
        } else {
            completed.push(cat);
        }
    }

    let status = if retained.is_empty() {
        ErasureStatus::Completed
    } else if !completed.is_empty() {
        ErasureStatus::PartiallyCompleted
    } else {
        ErasureStatus::Rejected
    };

    ErasureRequest {
        id: ErasureRequestId::new(),
        participant_id,
        status,
        categories_requested: categories.to_vec(),
        categories_completed: completed,
        categories_retained: retained,
        requested_at: now,
        completed_at: if status == ErasureStatus::Completed {
            Some(now)
        } else {
            None
        },
        processor_notes: None,
    }
}

/// Check whether data protection laws (GDPR/PDPA/PDPO) apply for a jurisdiction.
pub fn data_protection_applies(jurisdiction: Jurisdiction) -> bool {
    match jurisdiction {
        Jurisdiction::Us => false, // No federal data protection; state laws may apply
        Jurisdiction::Eu => true,  // GDPR
        Jurisdiction::Hk => true,  // PDPO
        Jurisdiction::Sg => true,  // PDPA
        Jurisdiction::Ae => true,  // UAE Data Protection Law
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eu_data_protection_applies() {
        assert!(data_protection_applies(Jurisdiction::Eu));
        assert!(!data_protection_applies(Jurisdiction::Us));
    }

    #[test]
    fn test_erasure_blocked_by_retention() {
        let pid = ParticipantId::new();
        let one_year_ago = Utc::now() - chrono::Duration::days(365);
        let request = evaluate_erasure_request(
            pid,
            &[DataCategory::Identity, DataCategory::Behavioral],
            Jurisdiction::Eu,
            one_year_ago,
            false,
            false,
        );
        // Identity retained (< 5 years), Behavioral can be erased
        assert!(request
            .categories_retained
            .iter()
            .any(|(c, _)| *c == DataCategory::Identity));
        assert!(request
            .categories_completed
            .contains(&DataCategory::Behavioral));
        assert_eq!(request.status, ErasureStatus::PartiallyCompleted);
    }

    #[test]
    fn test_erasure_blocked_by_investigation() {
        let pid = ParticipantId::new();
        let ten_years_ago = Utc::now() - chrono::Duration::days(3650);
        let request = evaluate_erasure_request(
            pid,
            &[DataCategory::Identity, DataCategory::Behavioral],
            Jurisdiction::Eu,
            ten_years_ago,
            false,
            true,
        );
        // Everything retained due to investigation
        assert_eq!(request.status, ErasureStatus::Rejected);
    }

    #[test]
    fn test_erasure_after_retention() {
        let pid = ParticipantId::new();
        let ten_years_ago = Utc::now() - chrono::Duration::days(3650);
        let request = evaluate_erasure_request(
            pid,
            &[DataCategory::Identity, DataCategory::Behavioral],
            Jurisdiction::Eu,
            ten_years_ago,
            false,
            false,
        );
        assert_eq!(request.status, ErasureStatus::Completed);
    }
}
