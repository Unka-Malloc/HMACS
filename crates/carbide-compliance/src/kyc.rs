use chrono::{DateTime, Utc};
use carbide_core::{CarbideError, CarbideResult, KycRecordId, ParticipantId};
use serde::{Deserialize, Serialize};

use crate::jurisdiction::{Jurisdiction, KycTier};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KycStatus {
    NotStarted,
    Pending,
    InReview,
    Approved,
    Rejected,
    Expired,
    Suspended,
}

impl std::fmt::Display for KycStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::NotStarted => "not_started",
            Self::Pending => "pending",
            Self::InReview => "in_review",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
            Self::Suspended => "suspended",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentType {
    Passport,
    NationalId,
    DriverLicense,
    AddressProof,
    BankStatement,
    CorporateRegistration,
    SourceOfFunds,
    TaxId,
    SelfieWithId,
    AgentOperatorLicense,
}

impl std::fmt::Display for DocumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Passport => "passport",
            Self::NationalId => "national_id",
            Self::DriverLicense => "driver_license",
            Self::AddressProof => "address_proof",
            Self::BankStatement => "bank_statement",
            Self::CorporateRegistration => "corporate_registration",
            Self::SourceOfFunds => "source_of_funds",
            Self::TaxId => "tax_id",
            Self::SelfieWithId => "selfie_with_id",
            Self::AgentOperatorLicense => "agent_operator_license",
        };
        write!(f, "{}", s)
    }
}

/// A submitted KYC document reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycDocument {
    pub document_type: DocumentType,
    /// SHA-256 hash of the document content (never store raw PII in plaintext)
    pub content_hash: String,
    /// Encrypted storage reference (S3 key, vault path, etc.)
    pub storage_ref: String,
    pub submitted_at: DateTime<Utc>,
    pub verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
}

/// The KYC record for a participant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycRecord {
    pub id: KycRecordId,
    pub participant_id: ParticipantId,
    pub jurisdiction: Jurisdiction,
    pub tier: KycTier,
    pub status: KycStatus,
    /// Legal name (encrypted at rest — use only for compliance checks)
    pub legal_name: Option<String>,
    pub nationality: Option<String>,
    pub country_of_residence: Option<String>,
    pub date_of_birth: Option<String>,
    pub documents: Vec<KycDocument>,
    pub risk_score: Option<u32>,
    pub reviewer_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
    /// Expiration date after which re-verification is needed
    pub expires_at: Option<DateTime<Utc>>,
}

impl KycRecord {
    pub fn is_approved(&self) -> bool {
        self.status == KycStatus::Approved
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires_at {
            Utc::now() > expires
        } else {
            false
        }
    }

    pub fn is_valid(&self) -> bool {
        self.is_approved() && !self.is_expired()
    }

    pub fn meets_tier(&self, required: KycTier) -> bool {
        self.is_valid() && self.tier >= required
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitKycRequest {
    pub jurisdiction: Jurisdiction,
    pub tier: KycTier,
    pub legal_name: Option<String>,
    pub nationality: Option<String>,
    pub country_of_residence: Option<String>,
    pub date_of_birth: Option<String>,
    pub documents: Vec<SubmitDocumentRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitDocumentRequest {
    pub document_type: DocumentType,
    pub content_hash: String,
    pub storage_ref: String,
}

/// Required documents per KYC tier.
pub fn required_documents(tier: KycTier) -> Vec<DocumentType> {
    match tier {
        KycTier::None => vec![],
        KycTier::Basic => vec![DocumentType::SelfieWithId],
        KycTier::Standard => vec![
            DocumentType::Passport,
            DocumentType::AddressProof,
            DocumentType::SelfieWithId,
        ],
        KycTier::Enhanced => vec![
            DocumentType::Passport,
            DocumentType::AddressProof,
            DocumentType::SelfieWithId,
            DocumentType::SourceOfFunds,
            DocumentType::BankStatement,
        ],
    }
}

/// Validate that a KYC submission includes all required documents.
pub fn validate_kyc_submission(request: &SubmitKycRequest) -> CarbideResult<()> {
    let required = required_documents(request.tier);
    let submitted: Vec<DocumentType> = request
        .documents
        .iter()
        .map(|d| d.document_type)
        .collect();

    for req_doc in &required {
        if !submitted.contains(req_doc) {
            return Err(CarbideError::InvalidInput(format!(
                "Missing required document: {} for {} tier",
                req_doc, request.tier
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_approved_record(tier: KycTier) -> KycRecord {
        KycRecord {
            id: KycRecordId::new(),
            participant_id: ParticipantId::new(),
            jurisdiction: Jurisdiction::Us,
            tier,
            status: KycStatus::Approved,
            legal_name: Some("Test User".into()),
            nationality: None,
            country_of_residence: None,
            date_of_birth: None,
            documents: vec![],
            risk_score: None,
            reviewer_notes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            approved_at: Some(Utc::now()),
            expires_at: Some(Utc::now() + chrono::Duration::days(365)),
        }
    }

    #[test]
    fn test_kyc_tier_ordering() {
        assert!(KycTier::Enhanced > KycTier::Standard);
        assert!(KycTier::Standard > KycTier::Basic);
        assert!(KycTier::Basic > KycTier::None);
    }

    #[test]
    fn test_meets_tier() {
        let record = make_approved_record(KycTier::Standard);
        assert!(record.meets_tier(KycTier::Basic));
        assert!(record.meets_tier(KycTier::Standard));
        assert!(!record.meets_tier(KycTier::Enhanced));
    }

    #[test]
    fn test_expired_record_not_valid() {
        let mut record = make_approved_record(KycTier::Standard);
        record.expires_at = Some(Utc::now() - chrono::Duration::days(1));
        assert!(!record.is_valid());
        assert!(!record.meets_tier(KycTier::Basic));
    }

    #[test]
    fn test_validate_submission_missing_doc() {
        let req = SubmitKycRequest {
            jurisdiction: Jurisdiction::Us,
            tier: KycTier::Standard,
            legal_name: Some("Test".into()),
            nationality: None,
            country_of_residence: None,
            date_of_birth: None,
            documents: vec![SubmitDocumentRequest {
                document_type: DocumentType::Passport,
                content_hash: "abc".into(),
                storage_ref: "s3://docs/1".into(),
            }],
        };
        let result = validate_kyc_submission(&req);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_submission_complete() {
        let docs: Vec<SubmitDocumentRequest> =
            required_documents(KycTier::Standard)
                .into_iter()
                .map(|dt| SubmitDocumentRequest {
                    document_type: dt,
                    content_hash: "hash".into(),
                    storage_ref: "ref".into(),
                })
                .collect();

        let req = SubmitKycRequest {
            jurisdiction: Jurisdiction::Us,
            tier: KycTier::Standard,
            legal_name: Some("Test".into()),
            nationality: None,
            country_of_residence: None,
            date_of_birth: None,
            documents: docs,
        };
        assert!(validate_kyc_submission(&req).is_ok());
    }
}
