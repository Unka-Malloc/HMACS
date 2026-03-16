use chrono::{DateTime, Utc};
use hmacs_core::{ParticipantId, ScreeningId};
use serde::{Deserialize, Serialize};

/// Sanctions list providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SanctionsList {
    /// US OFAC SDN list
    OfacSdn,
    /// US OFAC Consolidated list
    OfacConsolidated,
    /// EU Consolidated Sanctions list
    EuConsolidated,
    /// UN Security Council Sanctions
    UnSanctions,
    /// UK HMT Sanctions
    UkHmt,
    /// Hong Kong MAS designated list
    HkDesignated,
    /// Singapore MAS designated list
    SgMas,
    /// UAE Local Terrorist List
    AeLocalList,
}

impl std::fmt::Display for SanctionsList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OfacSdn => write!(f, "OFAC_SDN"),
            Self::OfacConsolidated => write!(f, "OFAC_CONSOLIDATED"),
            Self::EuConsolidated => write!(f, "EU_CONSOLIDATED"),
            Self::UnSanctions => write!(f, "UN_SANCTIONS"),
            Self::UkHmt => write!(f, "UK_HMT"),
            Self::HkDesignated => write!(f, "HK_DESIGNATED"),
            Self::SgMas => write!(f, "SG_MAS"),
            Self::AeLocalList => write!(f, "AE_LOCAL_LIST"),
        }
    }
}

/// The result of a single screening against a sanctions list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreeningResult {
    Clear,
    PotentialMatch,
    ConfirmedMatch,
    FalsePositive,
    PendingReview,
}

/// Category of match (sanctions, PEP, adverse media, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchCategory {
    Sanctions,
    Pep,
    AdverseMedia,
    LawEnforcement,
    RegulatoryAction,
}

/// A single match found during screening.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningMatch {
    pub list: SanctionsList,
    pub category: MatchCategory,
    pub matched_name: String,
    pub match_score: f64,
    pub list_entry_id: Option<String>,
    pub details: Option<String>,
}

/// Full screening record for a participant or wallet address.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningRecord {
    pub id: ScreeningId,
    pub participant_id: ParticipantId,
    pub screened_name: Option<String>,
    pub screened_address: Option<String>,
    pub result: ScreeningResult,
    pub matches: Vec<ScreeningMatch>,
    pub lists_checked: Vec<SanctionsList>,
    pub screened_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub reviewer_id: Option<ParticipantId>,
    pub reviewer_notes: Option<String>,
}

impl ScreeningRecord {
    pub fn is_clear(&self) -> bool {
        self.result == ScreeningResult::Clear || self.result == ScreeningResult::FalsePositive
    }

    pub fn needs_review(&self) -> bool {
        self.result == ScreeningResult::PotentialMatch
            || self.result == ScreeningResult::PendingReview
    }

    pub fn is_blocked(&self) -> bool {
        self.result == ScreeningResult::ConfirmedMatch
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenRequest {
    pub participant_id: ParticipantId,
    pub name: Option<String>,
    pub wallet_address: Option<String>,
    pub lists: Vec<SanctionsList>,
}

/// Lists that should be checked per jurisdiction.
pub fn required_lists(jurisdiction: &crate::jurisdiction::Jurisdiction) -> Vec<SanctionsList> {
    use crate::jurisdiction::Jurisdiction;
    let mut lists = vec![SanctionsList::UnSanctions];
    match jurisdiction {
        Jurisdiction::Us => {
            lists.extend([SanctionsList::OfacSdn, SanctionsList::OfacConsolidated]);
        }
        Jurisdiction::Eu => {
            lists.extend([SanctionsList::EuConsolidated, SanctionsList::UkHmt]);
        }
        Jurisdiction::Hk => {
            lists.extend([
                SanctionsList::HkDesignated,
                SanctionsList::OfacSdn,
            ]);
        }
        Jurisdiction::Sg => {
            lists.extend([SanctionsList::SgMas, SanctionsList::OfacSdn]);
        }
        Jurisdiction::Ae => {
            lists.extend([
                SanctionsList::AeLocalList,
                SanctionsList::OfacSdn,
            ]);
        }
    }
    lists
}

/// Simulate a screening. In production, this calls an external screening API
/// (e.g., Chainalysis, Elliptic, ComplyAdvantage).
pub fn perform_screening(request: &ScreenRequest) -> ScreeningRecord {
    // Placeholder: always returns clear.
    // Real implementation would call external AML/sanctions APIs.
    ScreeningRecord {
        id: ScreeningId::new(),
        participant_id: request.participant_id,
        screened_name: request.name.clone(),
        screened_address: request.wallet_address.clone(),
        result: ScreeningResult::Clear,
        matches: vec![],
        lists_checked: request.lists.clone(),
        screened_at: Utc::now(),
        reviewed_at: None,
        reviewer_id: None,
        reviewer_notes: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jurisdiction::Jurisdiction;

    #[test]
    fn test_us_requires_ofac() {
        let lists = required_lists(&Jurisdiction::Us);
        assert!(lists.contains(&SanctionsList::OfacSdn));
        assert!(lists.contains(&SanctionsList::OfacConsolidated));
        assert!(lists.contains(&SanctionsList::UnSanctions));
    }

    #[test]
    fn test_screening_clear() {
        let record = perform_screening(&ScreenRequest {
            participant_id: ParticipantId::new(),
            name: Some("Alice".into()),
            wallet_address: None,
            lists: vec![SanctionsList::OfacSdn],
        });
        assert!(record.is_clear());
        assert!(!record.is_blocked());
    }

    #[test]
    fn test_eu_requires_eu_consolidated() {
        let lists = required_lists(&Jurisdiction::Eu);
        assert!(lists.contains(&SanctionsList::EuConsolidated));
    }
}
