use chrono::{DateTime, Utc};
use hmacs_core::{AssetSymbol, ParticipantId};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::jurisdiction::{Jurisdiction, KycTier};

/// Risk level classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Prohibited,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Prohibited => write!(f, "prohibited"),
        }
    }
}

/// Individual risk factors and their scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskFactor {
    pub name: String,
    pub category: RiskCategory,
    pub score: u32,
    pub max_score: u32,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskCategory {
    Geographic,
    Identity,
    Behavioral,
    Transactional,
    Counterparty,
    Product,
}

/// Composite risk assessment for a participant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub participant_id: ParticipantId,
    pub overall_score: u32,
    pub max_possible_score: u32,
    pub level: RiskLevel,
    pub factors: Vec<RiskFactor>,
    pub assessed_at: DateTime<Utc>,
    pub next_review_at: Option<DateTime<Utc>>,
}

impl RiskAssessment {
    pub fn is_acceptable(&self) -> bool {
        self.level != RiskLevel::Prohibited
    }
}

/// Risk assessment for a specific transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRiskAssessment {
    pub participant_id: ParticipantId,
    pub amount: Decimal,
    pub asset: AssetSymbol,
    pub jurisdiction: Jurisdiction,
    pub level: RiskLevel,
    pub score: u32,
    pub factors: Vec<RiskFactor>,
    pub requires_enhanced_review: bool,
    pub assessed_at: DateTime<Utc>,
}

/// High-risk jurisdictions (FATF grey/black list, conflict zones, etc.)
const HIGH_RISK_COUNTRIES: &[&str] = &[
    "KP", "IR", "SY", "MM", "YE", "AF", "LY", "SO", "SS", "VE", "CU",
];

/// Input data for participant risk assessment.
pub struct RiskInput<'a> {
    pub participant_id: ParticipantId,
    pub kyc_tier: KycTier,
    pub jurisdiction: Jurisdiction,
    pub country_of_residence: Option<&'a str>,
    pub has_sanctions_match: bool,
    pub is_pep: bool,
    pub account_age_days: u32,
    pub total_volume_usdc: Decimal,
}

/// Compute the risk score for a participant based on available data.
pub fn assess_participant_risk(input: &RiskInput<'_>) -> RiskAssessment {
    let participant_id = input.participant_id;
    let kyc_tier = input.kyc_tier;
    let jurisdiction = input.jurisdiction;
    let country_of_residence = input.country_of_residence;
    let has_sanctions_match = input.has_sanctions_match;
    let is_pep = input.is_pep;
    let account_age_days = input.account_age_days;
    let total_volume_usdc = input.total_volume_usdc;

    let mut factors = Vec::new();
    let mut total_score: u32 = 0;
    let max_possible: u32 = 100;

    // Factor 1: KYC completeness (0-20)
    let kyc_score = match kyc_tier {
        KycTier::Enhanced => 0,
        KycTier::Standard => 5,
        KycTier::Basic => 12,
        KycTier::None => 20,
    };
    factors.push(RiskFactor {
        name: "KYC Completeness".into(),
        category: RiskCategory::Identity,
        score: kyc_score,
        max_score: 20,
        details: Some(format!("KYC tier: {}", kyc_tier)),
    });
    total_score += kyc_score;

    // Factor 2: Geographic risk (0-25)
    let geo_score = if has_sanctions_match {
        25
    } else if let Some(country) = country_of_residence {
        if HIGH_RISK_COUNTRIES.contains(&country) {
            25
        } else {
            match jurisdiction {
                Jurisdiction::Us | Jurisdiction::Eu | Jurisdiction::Sg => 0,
                Jurisdiction::Hk => 2,
                Jurisdiction::Ae => 5,
            }
        }
    } else {
        10 // unknown country
    };
    factors.push(RiskFactor {
        name: "Geographic Risk".into(),
        category: RiskCategory::Geographic,
        score: geo_score,
        max_score: 25,
        details: country_of_residence.map(|c| format!("Country: {}", c)),
    });
    total_score += geo_score;

    // Factor 3: PEP status (0-20)
    let pep_score = if is_pep { 20 } else { 0 };
    factors.push(RiskFactor {
        name: "PEP Status".into(),
        category: RiskCategory::Identity,
        score: pep_score,
        max_score: 20,
        details: if is_pep {
            Some("Politically Exposed Person".into())
        } else {
            None
        },
    });
    total_score += pep_score;

    // Factor 4: Account maturity (0-15)
    let maturity_score = if account_age_days < 7 {
        15
    } else if account_age_days < 30 {
        10
    } else if account_age_days < 90 {
        5
    } else {
        0
    };
    factors.push(RiskFactor {
        name: "Account Maturity".into(),
        category: RiskCategory::Behavioral,
        score: maturity_score,
        max_score: 15,
        details: Some(format!("{} days old", account_age_days)),
    });
    total_score += maturity_score;

    // Factor 5: Transaction volume (0-20)
    let volume_score = if total_volume_usdc > dec!(100000) {
        15
    } else if total_volume_usdc > dec!(50000) {
        10
    } else if total_volume_usdc > dec!(10000) {
        5
    } else {
        0
    };
    factors.push(RiskFactor {
        name: "Transaction Volume".into(),
        category: RiskCategory::Transactional,
        score: volume_score,
        max_score: 20,
        details: Some(format!("Total volume: {} USDC", total_volume_usdc)),
    });
    total_score += volume_score;

    let level = if has_sanctions_match {
        RiskLevel::Prohibited
    } else if total_score >= 60 {
        RiskLevel::High
    } else if total_score >= 30 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };

    let next_review = match level {
        RiskLevel::Prohibited => None,
        RiskLevel::High => Some(Utc::now() + chrono::Duration::days(30)),
        RiskLevel::Medium => Some(Utc::now() + chrono::Duration::days(90)),
        RiskLevel::Low => Some(Utc::now() + chrono::Duration::days(365)),
    };

    RiskAssessment {
        participant_id,
        overall_score: total_score,
        max_possible_score: max_possible,
        level,
        factors,
        assessed_at: Utc::now(),
        next_review_at: next_review,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_low_risk_participant() {
        let assessment = assess_participant_risk(&RiskInput {
            participant_id: ParticipantId::new(),
            kyc_tier: KycTier::Enhanced,
            jurisdiction: Jurisdiction::Us,
            country_of_residence: Some("US"),
            has_sanctions_match: false,
            is_pep: false,
            account_age_days: 365,
            total_volume_usdc: dec!(5000),
        });
        assert_eq!(assessment.level, RiskLevel::Low);
        assert!(assessment.is_acceptable());
    }

    #[test]
    fn test_sanctions_match_prohibited() {
        let assessment = assess_participant_risk(&RiskInput {
            participant_id: ParticipantId::new(),
            kyc_tier: KycTier::Enhanced,
            jurisdiction: Jurisdiction::Us,
            country_of_residence: Some("US"),
            has_sanctions_match: true,
            is_pep: false,
            account_age_days: 365,
            total_volume_usdc: dec!(5000),
        });
        assert_eq!(assessment.level, RiskLevel::Prohibited);
        assert!(!assessment.is_acceptable());
    }

    #[test]
    fn test_new_account_pep_high_risk() {
        let assessment = assess_participant_risk(&RiskInput {
            participant_id: ParticipantId::new(),
            kyc_tier: KycTier::Basic,
            jurisdiction: Jurisdiction::Eu,
            country_of_residence: None,
            has_sanctions_match: false,
            is_pep: true,
            account_age_days: 3,
            total_volume_usdc: dec!(60000),
        });
        // kyc=12 + geo=10 + pep=20 + maturity=15 + volume=15 = 72 → High
        assert_eq!(assessment.level, RiskLevel::High);
    }

    #[test]
    fn test_risk_level_ordering() {
        assert!(RiskLevel::Prohibited > RiskLevel::High);
        assert!(RiskLevel::High > RiskLevel::Medium);
        assert!(RiskLevel::Medium > RiskLevel::Low);
    }
}
