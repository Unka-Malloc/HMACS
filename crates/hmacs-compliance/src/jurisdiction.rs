use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported regulatory jurisdictions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Jurisdiction {
    /// United States — FinCEN / SEC / CFTC / OFAC
    Us,
    /// European Union — MiCA / AMLD / GDPR
    Eu,
    /// Hong Kong — SFC / AMLO
    Hk,
    /// Singapore — MAS / PSA
    Sg,
    /// Dubai — VARA / DFSA
    Ae,
}

impl std::fmt::Display for Jurisdiction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Us => write!(f, "US"),
            Self::Eu => write!(f, "EU"),
            Self::Hk => write!(f, "HK"),
            Self::Sg => write!(f, "SG"),
            Self::Ae => write!(f, "AE"),
        }
    }
}

/// KYC tier requirement per jurisdiction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KycTier {
    /// No verification — anonymous usage (very limited)
    None,
    /// Basic — email/phone + name
    Basic,
    /// Standard — government ID + address proof
    Standard,
    /// Enhanced — full due diligence (source of funds, business docs)
    Enhanced,
}

impl std::fmt::Display for KycTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Basic => write!(f, "basic"),
            Self::Standard => write!(f, "standard"),
            Self::Enhanced => write!(f, "enhanced"),
        }
    }
}

/// Jurisdiction-specific regulatory rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionRules {
    pub jurisdiction: Jurisdiction,
    /// Minimum KYC tier to onboard
    pub min_kyc_tier: KycTier,
    /// KYC tier required for transactions above the threshold
    pub elevated_kyc_tier: KycTier,
    /// Transaction amount (in USDC equivalent) triggering elevated KYC
    pub elevated_kyc_threshold: Decimal,
    /// Travel rule applies above this amount (in USDC equivalent)
    pub travel_rule_threshold: Decimal,
    /// Large transaction reporting threshold (in USDC equivalent)
    pub large_tx_report_threshold: Decimal,
    /// Whether sanctions screening is mandatory for every transaction
    pub mandatory_sanctions_screening: bool,
    /// Whether PEP (Politically Exposed Person) screening is required
    pub pep_screening_required: bool,
    /// Whether GDPR-style data protection applies
    pub data_protection_applies: bool,
    /// Maximum data retention period in days (0 = no limit specified)
    pub max_data_retention_days: u32,
    /// Whether agents must also complete KYC (via their operator)
    pub agent_kyc_required: bool,
    /// Regulatory body name for reference
    pub regulator: String,
    /// License type required
    pub license_type: String,
}

impl JurisdictionRules {
    /// US — FinCEN BSA / OFAC / SEC
    pub fn us() -> Self {
        Self {
            jurisdiction: Jurisdiction::Us,
            min_kyc_tier: KycTier::Basic,
            elevated_kyc_tier: KycTier::Enhanced,
            // CDD threshold: $3,000 for Travel Rule; $10,000 for CTR
            elevated_kyc_threshold: dec!(10000),
            travel_rule_threshold: dec!(3000),
            large_tx_report_threshold: dec!(10000),
            mandatory_sanctions_screening: true,
            pep_screening_required: true,
            data_protection_applies: false,
            max_data_retention_days: 1825, // 5 years BSA retention
            agent_kyc_required: true,
            regulator: "FinCEN / SEC / CFTC".into(),
            license_type: "Money Services Business (MSB)".into(),
        }
    }

    /// EU — MiCA / 5th/6th AMLD / GDPR
    pub fn eu() -> Self {
        Self {
            jurisdiction: Jurisdiction::Eu,
            min_kyc_tier: KycTier::Standard,
            elevated_kyc_tier: KycTier::Enhanced,
            // MiCA: €1,000 travel rule threshold
            elevated_kyc_threshold: dec!(15000),
            travel_rule_threshold: dec!(1000),
            large_tx_report_threshold: dec!(15000),
            mandatory_sanctions_screening: true,
            pep_screening_required: true,
            data_protection_applies: true,
            max_data_retention_days: 1825, // 5 years per AMLD
            agent_kyc_required: true,
            regulator: "National Competent Authority (NCA)".into(),
            license_type: "Crypto-Asset Service Provider (CASP)".into(),
        }
    }

    /// Hong Kong — SFC / AMLO
    pub fn hk() -> Self {
        Self {
            jurisdiction: Jurisdiction::Hk,
            min_kyc_tier: KycTier::Standard,
            elevated_kyc_tier: KycTier::Enhanced,
            // HKD 120,000 ≈ USD 15,300
            elevated_kyc_threshold: dec!(15300),
            travel_rule_threshold: dec!(1000),
            large_tx_report_threshold: dec!(15300),
            mandatory_sanctions_screening: true,
            pep_screening_required: true,
            data_protection_applies: true, // PDPO
            max_data_retention_days: 2190, // 6 years
            agent_kyc_required: true,
            regulator: "SFC (Securities and Futures Commission)".into(),
            license_type: "VASP License".into(),
        }
    }

    /// Singapore — MAS / PSA
    pub fn sg() -> Self {
        Self {
            jurisdiction: Jurisdiction::Sg,
            min_kyc_tier: KycTier::Standard,
            elevated_kyc_tier: KycTier::Enhanced,
            // SGD 20,000 ≈ USD 15,000
            elevated_kyc_threshold: dec!(15000),
            // SGD 1,500 ≈ USD 1,100
            travel_rule_threshold: dec!(1100),
            large_tx_report_threshold: dec!(15000),
            mandatory_sanctions_screening: true,
            pep_screening_required: true,
            data_protection_applies: true, // PDPA
            max_data_retention_days: 1825, // 5 years
            agent_kyc_required: true,
            regulator: "MAS (Monetary Authority of Singapore)".into(),
            license_type: "Major Payment Institution (MPI)".into(),
        }
    }

    /// Dubai — VARA
    pub fn ae() -> Self {
        Self {
            jurisdiction: Jurisdiction::Ae,
            min_kyc_tier: KycTier::Standard,
            elevated_kyc_tier: KycTier::Enhanced,
            // AED 55,000 ≈ USD 15,000
            elevated_kyc_threshold: dec!(15000),
            travel_rule_threshold: dec!(1000),
            large_tx_report_threshold: dec!(15000),
            mandatory_sanctions_screening: true,
            pep_screening_required: true,
            data_protection_applies: true, // UAE Data Protection Law
            max_data_retention_days: 1825, // 5 years
            agent_kyc_required: true,
            regulator: "VARA (Virtual Assets Regulatory Authority)".into(),
            license_type: "VASP License (VARA)".into(),
        }
    }

    pub fn for_jurisdiction(j: Jurisdiction) -> Self {
        match j {
            Jurisdiction::Us => Self::us(),
            Jurisdiction::Eu => Self::eu(),
            Jurisdiction::Hk => Self::hk(),
            Jurisdiction::Sg => Self::sg(),
            Jurisdiction::Ae => Self::ae(),
        }
    }

    /// Check whether a transaction amount triggers the travel rule.
    pub fn requires_travel_rule(&self, usdc_equivalent: Decimal) -> bool {
        usdc_equivalent >= self.travel_rule_threshold
    }

    /// Check whether a transaction amount requires elevated KYC.
    pub fn requires_elevated_kyc(&self, usdc_equivalent: Decimal) -> bool {
        usdc_equivalent >= self.elevated_kyc_threshold
    }

    /// Check whether a transaction must be reported as a large transaction.
    pub fn requires_large_tx_report(&self, usdc_equivalent: Decimal) -> bool {
        usdc_equivalent >= self.large_tx_report_threshold
    }
}

/// Registry holding rules for all operational jurisdictions.
pub struct JurisdictionRegistry {
    rules: HashMap<Jurisdiction, JurisdictionRules>,
}

impl JurisdictionRegistry {
    pub fn new() -> Self {
        let mut rules = HashMap::new();
        for j in [
            Jurisdiction::Us,
            Jurisdiction::Eu,
            Jurisdiction::Hk,
            Jurisdiction::Sg,
            Jurisdiction::Ae,
        ] {
            rules.insert(j, JurisdictionRules::for_jurisdiction(j));
        }
        Self { rules }
    }

    pub fn get(&self, jurisdiction: &Jurisdiction) -> Option<&JurisdictionRules> {
        self.rules.get(jurisdiction)
    }

    pub fn all(&self) -> impl Iterator<Item = &JurisdictionRules> {
        self.rules.values()
    }
}

impl Default for JurisdictionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_us_travel_rule_threshold() {
        let rules = JurisdictionRules::us();
        assert!(!rules.requires_travel_rule(dec!(2999)));
        assert!(rules.requires_travel_rule(dec!(3000)));
        assert!(rules.requires_travel_rule(dec!(5000)));
    }

    #[test]
    fn test_eu_travel_rule_lower_threshold() {
        let rules = JurisdictionRules::eu();
        assert!(rules.requires_travel_rule(dec!(1000)));
        assert!(!rules.requires_travel_rule(dec!(999)));
    }

    #[test]
    fn test_eu_gdpr_applies() {
        let rules = JurisdictionRules::eu();
        assert!(rules.data_protection_applies);
    }

    #[test]
    fn test_all_jurisdictions_have_sanctions_screening() {
        let registry = JurisdictionRegistry::new();
        for rules in registry.all() {
            assert!(
                rules.mandatory_sanctions_screening,
                "{} should require sanctions screening",
                rules.jurisdiction
            );
        }
    }

    #[test]
    fn test_registry_lookup() {
        let registry = JurisdictionRegistry::new();
        let sg = registry.get(&Jurisdiction::Sg).unwrap();
        assert_eq!(sg.jurisdiction, Jurisdiction::Sg);
        assert_eq!(sg.license_type, "Major Payment Institution (MPI)");
    }
}
