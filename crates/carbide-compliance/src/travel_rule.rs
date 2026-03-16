use chrono::{DateTime, Utc};
use carbide_core::{AssetSymbol, ParticipantId, TravelRuleMessageId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::jurisdiction::{Jurisdiction, JurisdictionRules};

/// VASP (Virtual Asset Service Provider) identifying information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaspInfo {
    pub name: String,
    pub lei: Option<String>,
    pub jurisdiction: Jurisdiction,
    pub registration_number: Option<String>,
    pub website: Option<String>,
    pub address: Option<String>,
}

/// Originator / Beneficiary information as required by the travel rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferParty {
    pub participant_id: ParticipantId,
    /// Full name (required above threshold)
    pub name: Option<String>,
    /// Wallet address on-chain
    pub wallet_address: String,
    /// Account reference on the VASP (internal ID)
    pub account_ref: Option<String>,
    /// Geographic address (required in some jurisdictions)
    pub geographic_address: Option<String>,
    /// National identifier (required in some jurisdictions)
    pub national_id: Option<String>,
    /// Date/place of birth (required by some jurisdictions for natural persons)
    pub date_of_birth: Option<String>,
    pub place_of_birth: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TravelRuleStatus {
    NotRequired,
    Pending,
    Sent,
    Received,
    Confirmed,
    Rejected,
    Failed,
}

/// A travel rule message associated with a crypto transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TravelRuleMessage {
    pub id: TravelRuleMessageId,
    pub status: TravelRuleStatus,
    pub jurisdiction: Jurisdiction,
    pub amount: Decimal,
    pub asset: AssetSymbol,
    pub originator: TransferParty,
    pub beneficiary: TransferParty,
    pub originating_vasp: VaspInfo,
    pub beneficiary_vasp: Option<VaspInfo>,
    pub tx_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

/// Check whether a transfer requires travel rule compliance.
pub fn check_travel_rule_required(
    amount_usdc_equivalent: Decimal,
    jurisdiction: Jurisdiction,
) -> bool {
    let rules = JurisdictionRules::for_jurisdiction(jurisdiction);
    rules.requires_travel_rule(amount_usdc_equivalent)
}

/// Validate that the originator info meets the jurisdiction's requirements.
pub fn validate_originator_info(
    party: &TransferParty,
    jurisdiction: Jurisdiction,
) -> Result<(), String> {
    // All jurisdictions require the name for travel rule transfers
    if party.name.is_none() {
        return Err("Originator name is required for travel rule compliance".into());
    }

    match jurisdiction {
        // EU (MiCA): Requires name + account ref for any amount >= €1,000
        Jurisdiction::Eu => {
            if party.account_ref.is_none() {
                return Err("EU requires originator account reference".into());
            }
        }
        // US: Name + address/account for transfers >= $3,000
        Jurisdiction::Us => {
            if party.geographic_address.is_none() && party.account_ref.is_none() {
                return Err(
                    "US requires originator address or account number".into(),
                );
            }
        }
        // HK: Name + account + ID for transfers
        Jurisdiction::Hk => {
            if party.account_ref.is_none() {
                return Err("HK requires originator account reference".into());
            }
        }
        // SG: Name + account + unique identifier
        Jurisdiction::Sg => {
            if party.account_ref.is_none() {
                return Err("SG requires originator account reference".into());
            }
        }
        // AE: Name + account
        Jurisdiction::Ae => {
            if party.account_ref.is_none() {
                return Err("AE requires originator account reference".into());
            }
        }
    }
    Ok(())
}

/// Validate beneficiary info requirements.
pub fn validate_beneficiary_info(
    party: &TransferParty,
    jurisdiction: Jurisdiction,
) -> Result<(), String> {
    if party.name.is_none() {
        return Err("Beneficiary name is required for travel rule compliance".into());
    }

    // EU requires beneficiary account reference for any travel-rule amount
    if jurisdiction == Jurisdiction::Eu && party.account_ref.is_none() {
        return Err("EU requires beneficiary account reference".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_party(name: Option<&str>, account_ref: Option<&str>) -> TransferParty {
        TransferParty {
            participant_id: ParticipantId::new(),
            name: name.map(String::from),
            wallet_address: "0xabc123".into(),
            account_ref: account_ref.map(String::from),
            geographic_address: None,
            national_id: None,
            date_of_birth: None,
            place_of_birth: None,
        }
    }

    #[test]
    fn test_us_travel_rule_check() {
        assert!(!check_travel_rule_required(dec!(2000), Jurisdiction::Us));
        assert!(check_travel_rule_required(dec!(3000), Jurisdiction::Us));
    }

    #[test]
    fn test_eu_lower_threshold() {
        assert!(check_travel_rule_required(dec!(1000), Jurisdiction::Eu));
        assert!(!check_travel_rule_required(dec!(999), Jurisdiction::Eu));
    }

    #[test]
    fn test_originator_name_required() {
        let party = make_party(None, Some("acc-1"));
        assert!(validate_originator_info(&party, Jurisdiction::Us).is_err());
    }

    #[test]
    fn test_eu_originator_needs_account() {
        let party = make_party(Some("Alice"), None);
        assert!(validate_originator_info(&party, Jurisdiction::Eu).is_err());

        let party = make_party(Some("Alice"), Some("acc-1"));
        assert!(validate_originator_info(&party, Jurisdiction::Eu).is_ok());
    }

    #[test]
    fn test_us_originator_needs_address_or_account() {
        let party = make_party(Some("Bob"), None);
        assert!(validate_originator_info(&party, Jurisdiction::Us).is_err());

        let mut party = make_party(Some("Bob"), Some("acc-1"));
        assert!(validate_originator_info(&party, Jurisdiction::Us).is_ok());

        party.account_ref = None;
        party.geographic_address = Some("123 Main St".into());
        assert!(validate_originator_info(&party, Jurisdiction::Us).is_ok());
    }
}
