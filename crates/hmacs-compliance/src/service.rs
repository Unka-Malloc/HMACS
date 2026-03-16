use crate::aml::{self, ScreenRequest, ScreeningRecord, ScreeningResult};
use crate::gdpr::{self, ConsentRecord, DataCategory, ErasureRequest, LegalBasis};
use crate::jurisdiction::{Jurisdiction, JurisdictionRegistry, JurisdictionRules, KycTier};
use crate::kyc::{KycRecord, KycStatus, SubmitKycRequest};
use crate::monitoring::{self, ComplianceAlert, MonitoredTransaction, MonitoringConfig};
use crate::risk::{self, RiskAssessment};
use crate::travel_rule::{self, TransferParty, TravelRuleMessage, TravelRuleStatus, VaspInfo};
use chrono::{DateTime, Utc};
use hmacs_core::{
    AssetSymbol, ConsentRecordId, HmacsError, HmacsResult, KycRecordId, ParticipantId,
    TravelRuleMessageId,
};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use std::collections::HashMap;
use tracing::{info, warn};

/// Central compliance service that orchestrates KYC, AML, monitoring,
/// travel rule, risk scoring, and data protection across all jurisdictions.
pub struct ComplianceService {
    registry: JurisdictionRegistry,
    monitoring_config: MonitoringConfig,
    our_vasp: VaspInfo,

    kyc_records: RwLock<HashMap<ParticipantId, KycRecord>>,
    screening_records: RwLock<Vec<ScreeningRecord>>,
    alerts: RwLock<Vec<ComplianceAlert>>,
    travel_messages: RwLock<Vec<TravelRuleMessage>>,
    consent_records: RwLock<Vec<ConsentRecord>>,
    erasure_requests: RwLock<Vec<ErasureRequest>>,
    risk_assessments: RwLock<HashMap<ParticipantId, RiskAssessment>>,
    /// Tracks daily volume per participant (resets daily in production)
    daily_volumes: RwLock<HashMap<ParticipantId, Decimal>>,
    /// Tracks recent transaction counts per participant
    recent_tx_counts: RwLock<HashMap<ParticipantId, u32>>,
}

impl ComplianceService {
    pub fn new(our_vasp: VaspInfo) -> Self {
        Self {
            registry: JurisdictionRegistry::new(),
            monitoring_config: MonitoringConfig::default(),
            our_vasp,
            kyc_records: RwLock::new(HashMap::new()),
            screening_records: RwLock::new(Vec::new()),
            alerts: RwLock::new(Vec::new()),
            travel_messages: RwLock::new(Vec::new()),
            consent_records: RwLock::new(Vec::new()),
            erasure_requests: RwLock::new(Vec::new()),
            risk_assessments: RwLock::new(HashMap::new()),
            daily_volumes: RwLock::new(HashMap::new()),
            recent_tx_counts: RwLock::new(HashMap::new()),
        }
    }

    // ──────────────────── KYC ────────────────────

    /// Submit a KYC application.
    pub fn submit_kyc(
        &self,
        participant_id: ParticipantId,
        request: SubmitKycRequest,
    ) -> HmacsResult<KycRecord> {
        crate::kyc::validate_kyc_submission(&request)?;

        let now = Utc::now();
        let documents = request
            .documents
            .into_iter()
            .map(|d| crate::kyc::KycDocument {
                document_type: d.document_type,
                content_hash: d.content_hash,
                storage_ref: d.storage_ref,
                submitted_at: now,
                verified: false,
                verified_at: None,
                rejection_reason: None,
            })
            .collect();

        let record = KycRecord {
            id: KycRecordId::new(),
            participant_id,
            jurisdiction: request.jurisdiction,
            tier: request.tier,
            status: KycStatus::Pending,
            legal_name: request.legal_name,
            nationality: request.nationality,
            country_of_residence: request.country_of_residence,
            date_of_birth: request.date_of_birth,
            documents,
            risk_score: None,
            reviewer_notes: None,
            created_at: now,
            updated_at: now,
            approved_at: None,
            expires_at: None,
        };

        info!(
            participant = %participant_id,
            tier = %request.tier,
            jurisdiction = %request.jurisdiction,
            "KYC submission received"
        );

        self.kyc_records.write().insert(participant_id, record.clone());
        Ok(record)
    }

    /// Get a participant's KYC record.
    pub fn get_kyc(&self, participant_id: ParticipantId) -> Option<KycRecord> {
        self.kyc_records.read().get(&participant_id).cloned()
    }

    /// Check whether a participant meets the required KYC tier for a jurisdiction.
    pub fn check_kyc_clearance(
        &self,
        participant_id: ParticipantId,
        jurisdiction: Jurisdiction,
        amount_usdc: Option<Decimal>,
    ) -> HmacsResult<()> {
        let rules = self.get_rules(jurisdiction)?;

        let mut required_tier = rules.min_kyc_tier;
        if let Some(amount) = amount_usdc {
            if rules.requires_elevated_kyc(amount) {
                required_tier = rules.elevated_kyc_tier;
            }
        }

        let record = self.kyc_records.read().get(&participant_id).cloned();

        match record {
            Some(rec) if rec.meets_tier(required_tier) => Ok(()),
            Some(rec) => Err(HmacsError::KycRequired(format!(
                "Current KYC tier '{}' insufficient; '{}' required for {} in {}",
                rec.tier, required_tier, amount_usdc.unwrap_or_default(), jurisdiction
            ))),
            None => Err(HmacsError::KycRequired(format!(
                "No KYC record found; '{}' tier required for {}",
                required_tier, jurisdiction
            ))),
        }
    }

    // ──────────────────── AML / Sanctions ────────────────────

    /// Screen a participant against sanctions lists.
    pub fn screen_participant(
        &self,
        participant_id: ParticipantId,
        name: Option<String>,
        wallet_address: Option<String>,
        jurisdiction: Jurisdiction,
    ) -> HmacsResult<ScreeningRecord> {
        let lists = aml::required_lists(&jurisdiction);

        let request = ScreenRequest {
            participant_id,
            name,
            wallet_address,
            lists,
        };

        let record = aml::perform_screening(&request);

        info!(
            participant = %participant_id,
            result = ?record.result,
            lists = record.lists_checked.len(),
            "AML screening completed"
        );

        if record.is_blocked() {
            warn!(
                participant = %participant_id,
                "SANCTIONS MATCH — participant blocked"
            );
        }

        self.screening_records.write().push(record.clone());
        Ok(record)
    }

    /// Check that a participant is not blocked by sanctions.
    pub fn check_sanctions_clearance(
        &self,
        participant_id: ParticipantId,
        jurisdiction: Jurisdiction,
    ) -> HmacsResult<()> {
        let rules = self.get_rules(jurisdiction)?;

        if !rules.mandatory_sanctions_screening {
            return Ok(());
        }

        let screenings = self.screening_records.read();
        let latest = screenings
            .iter()
            .rev()
            .find(|s| s.participant_id == participant_id);

        match latest {
            Some(s) if s.is_clear() => Ok(()),
            Some(s) if s.is_blocked() => Err(HmacsError::SanctionsMatch(format!(
                "Participant {} has a confirmed sanctions match",
                participant_id
            ))),
            Some(_) => Err(HmacsError::ComplianceBlocked {
                reason: "Sanctions screening pending review".into(),
                jurisdiction: jurisdiction.to_string(),
            }),
            None => Err(HmacsError::ComplianceBlocked {
                reason: "No sanctions screening on record".into(),
                jurisdiction: jurisdiction.to_string(),
            }),
        }
    }

    // ──────────────────── Transaction Monitoring ────────────────────

    /// Evaluate a transaction and generate alerts if rules are triggered.
    pub fn monitor_transaction(
        &self,
        tx: MonitoredTransaction,
    ) -> HmacsResult<Vec<ComplianceAlert>> {
        let rules = self.get_rules(tx.jurisdiction)?;

        let recent_count = self
            .recent_tx_counts
            .read()
            .get(&tx.participant_id)
            .copied()
            .unwrap_or(0);

        let daily_volume = self
            .daily_volumes
            .read()
            .get(&tx.participant_id)
            .copied()
            .unwrap_or_default();

        // Approximate account age (in production, pull from DB)
        let account_age = 365u32;

        let alerts = monitoring::evaluate_transaction(
            &tx,
            &rules,
            &self.monitoring_config,
            recent_count,
            daily_volume,
            account_age,
        );

        if !alerts.is_empty() {
            info!(
                participant = %tx.participant_id,
                alert_count = alerts.len(),
                "Transaction monitoring alerts generated"
            );
        }

        // Record alerts
        self.alerts.write().extend(alerts.clone());

        // Update counters
        *self
            .daily_volumes
            .write()
            .entry(tx.participant_id)
            .or_default() += tx.amount;
        *self
            .recent_tx_counts
            .write()
            .entry(tx.participant_id)
            .or_default() += 1;

        Ok(alerts)
    }

    pub fn get_alerts(&self, participant_id: ParticipantId) -> Vec<ComplianceAlert> {
        self.alerts
            .read()
            .iter()
            .filter(|a| a.participant_id == participant_id)
            .cloned()
            .collect()
    }

    pub fn get_all_open_alerts(&self) -> Vec<ComplianceAlert> {
        self.alerts
            .read()
            .iter()
            .filter(|a| {
                matches!(
                    a.status,
                    monitoring::AlertStatus::Open | monitoring::AlertStatus::UnderReview
                )
            })
            .cloned()
            .collect()
    }

    // ──────────────────── Travel Rule ────────────────────

    /// Check if a transfer requires travel rule compliance and validate data.
    pub fn check_travel_rule(
        &self,
        amount_usdc_equivalent: Decimal,
        asset: AssetSymbol,
        jurisdiction: Jurisdiction,
        originator: &TransferParty,
        beneficiary: &TransferParty,
    ) -> HmacsResult<Option<TravelRuleMessage>> {
        if !travel_rule::check_travel_rule_required(amount_usdc_equivalent, jurisdiction) {
            return Ok(None);
        }

        travel_rule::validate_originator_info(originator, jurisdiction)
            .map_err(HmacsError::TravelRuleViolation)?;
        travel_rule::validate_beneficiary_info(beneficiary, jurisdiction)
            .map_err(HmacsError::TravelRuleViolation)?;

        let message = TravelRuleMessage {
            id: TravelRuleMessageId::new(),
            status: TravelRuleStatus::Pending,
            jurisdiction,
            amount: amount_usdc_equivalent,
            asset,
            originator: originator.clone(),
            beneficiary: beneficiary.clone(),
            originating_vasp: self.our_vasp.clone(),
            beneficiary_vasp: None,
            tx_hash: None,
            created_at: Utc::now(),
            sent_at: None,
            confirmed_at: None,
        };

        info!(
            message_id = %message.id,
            amount = %amount_usdc_equivalent,
            jurisdiction = %jurisdiction,
            "Travel rule message created"
        );

        self.travel_messages.write().push(message.clone());
        Ok(Some(message))
    }

    // ──────────────────── Risk Scoring ────────────────────

    /// Assess or reassess a participant's risk level.
    pub fn assess_risk(
        &self,
        participant_id: ParticipantId,
        jurisdiction: Jurisdiction,
    ) -> RiskAssessment {
        let kyc_tier = self
            .kyc_records
            .read()
            .get(&participant_id)
            .filter(|r| r.is_valid())
            .map(|r| r.tier)
            .unwrap_or(KycTier::None);

        let country = self
            .kyc_records
            .read()
            .get(&participant_id)
            .and_then(|r| r.country_of_residence.clone());

        let has_sanctions = self
            .screening_records
            .read()
            .iter()
            .any(|s| {
                s.participant_id == participant_id
                    && s.result == ScreeningResult::ConfirmedMatch
            });

        let total_volume = self
            .daily_volumes
            .read()
            .get(&participant_id)
            .copied()
            .unwrap_or_default();

        let assessment = risk::assess_participant_risk(&risk::RiskInput {
            participant_id,
            kyc_tier,
            jurisdiction,
            country_of_residence: country.as_deref(),
            has_sanctions_match: has_sanctions,
            is_pep: false, // PEP check would come from screening records in production
            account_age_days: 365, // account age from DB in production
            total_volume_usdc: total_volume,
        });

        self.risk_assessments
            .write()
            .insert(participant_id, assessment.clone());

        assessment
    }

    pub fn get_risk_assessment(&self, participant_id: ParticipantId) -> Option<RiskAssessment> {
        self.risk_assessments.read().get(&participant_id).cloned()
    }

    // ──────────────────── GDPR / Data Protection ────────────────────

    /// Record consent for data processing.
    pub fn record_consent(
        &self,
        participant_id: ParticipantId,
        category: DataCategory,
        legal_basis: LegalBasis,
        purpose: String,
        jurisdiction: Jurisdiction,
    ) -> ConsentRecord {
        let record = ConsentRecord {
            id: ConsentRecordId::new(),
            participant_id,
            data_category: category,
            legal_basis,
            purpose,
            granted: true,
            granted_at: Some(Utc::now()),
            revoked_at: None,
            source: None,
            jurisdiction,
        };
        self.consent_records.write().push(record.clone());
        record
    }

    /// Process a data erasure request.
    pub fn request_erasure(
        &self,
        participant_id: ParticipantId,
        categories: Vec<DataCategory>,
        jurisdiction: Jurisdiction,
        account_created_at: DateTime<Utc>,
        has_open_transactions: bool,
        under_investigation: bool,
    ) -> ErasureRequest {
        let request = gdpr::evaluate_erasure_request(
            participant_id,
            &categories,
            jurisdiction,
            account_created_at,
            has_open_transactions,
            under_investigation,
        );

        info!(
            participant = %participant_id,
            status = ?request.status,
            completed = request.categories_completed.len(),
            retained = request.categories_retained.len(),
            "Erasure request processed"
        );

        self.erasure_requests.write().push(request.clone());
        request
    }

    /// Check if data protection laws apply for a jurisdiction.
    pub fn requires_data_protection(&self, jurisdiction: Jurisdiction) -> bool {
        gdpr::data_protection_applies(jurisdiction)
    }

    // ──────────────────── Composite Checks ────────────────────

    /// Run all pre-transaction compliance checks:
    /// 1. KYC clearance
    /// 2. Sanctions clearance
    /// 3. Risk assessment
    ///
    /// Returns `Ok(())` if the transaction may proceed.
    pub fn pre_transaction_check(
        &self,
        participant_id: ParticipantId,
        jurisdiction: Jurisdiction,
        amount_usdc: Decimal,
    ) -> HmacsResult<()> {
        // 1. KYC
        self.check_kyc_clearance(participant_id, jurisdiction, Some(amount_usdc))?;

        // 2. Sanctions
        self.check_sanctions_clearance(participant_id, jurisdiction)?;

        // 3. Risk
        let assessment = self.assess_risk(participant_id, jurisdiction);
        if !assessment.is_acceptable() {
            return Err(HmacsError::ComplianceBlocked {
                reason: format!(
                    "Risk level '{}' is not acceptable (score: {}/{})",
                    assessment.level, assessment.overall_score, assessment.max_possible_score
                ),
                jurisdiction: jurisdiction.to_string(),
            });
        }

        Ok(())
    }

    fn get_rules(&self, jurisdiction: Jurisdiction) -> HmacsResult<JurisdictionRules> {
        self.registry
            .get(&jurisdiction)
            .cloned()
            .ok_or_else(|| HmacsError::InvalidInput(format!("Unknown jurisdiction: {}", jurisdiction)))
    }
}

impl Default for ComplianceService {
    fn default() -> Self {
        Self::new(VaspInfo {
            name: "HMACS Platform".into(),
            lei: None,
            jurisdiction: Jurisdiction::Sg,
            registration_number: None,
            website: Some("https://hmacs.io".into()),
            address: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kyc::{DocumentType, SubmitDocumentRequest, SubmitKycRequest};
    use crate::monitoring::TransactionDirection;
    use hmacs_core::TransactionId;
    use rust_decimal_macros::dec;

    fn make_service() -> ComplianceService {
        ComplianceService::default()
    }

    fn submit_approved_kyc(svc: &ComplianceService, pid: ParticipantId, tier: KycTier) {
        let docs = crate::kyc::required_documents(tier)
            .into_iter()
            .map(|dt| SubmitDocumentRequest {
                document_type: dt,
                content_hash: "hash".into(),
                storage_ref: "ref".into(),
            })
            .collect();

        svc.submit_kyc(
            pid,
            SubmitKycRequest {
                jurisdiction: Jurisdiction::Us,
                tier,
                legal_name: Some("Test User".into()),
                nationality: None,
                country_of_residence: Some("US".into()),
                date_of_birth: None,
                documents: docs,
            },
        )
        .unwrap();

        // Manually approve for testing
        let mut records = svc.kyc_records.write();
        if let Some(rec) = records.get_mut(&pid) {
            rec.status = KycStatus::Approved;
            rec.approved_at = Some(Utc::now());
            rec.expires_at = Some(Utc::now() + chrono::Duration::days(365));
        }
    }

    fn add_clear_screening(svc: &ComplianceService, pid: ParticipantId) {
        svc.screen_participant(pid, Some("Test".into()), None, Jurisdiction::Us)
            .unwrap();
    }

    #[test]
    fn test_pre_transaction_pass() {
        let svc = make_service();
        let pid = ParticipantId::new();
        submit_approved_kyc(&svc, pid, KycTier::Standard);
        add_clear_screening(&svc, pid);

        let result = svc.pre_transaction_check(pid, Jurisdiction::Us, dec!(5000));
        assert!(result.is_ok());
    }

    #[test]
    fn test_pre_transaction_fail_no_kyc() {
        let svc = make_service();
        let pid = ParticipantId::new();
        let result = svc.pre_transaction_check(pid, Jurisdiction::Us, dec!(5000));
        assert!(result.is_err());
    }

    #[test]
    fn test_pre_transaction_fail_no_screening() {
        let svc = make_service();
        let pid = ParticipantId::new();
        submit_approved_kyc(&svc, pid, KycTier::Standard);
        let result = svc.pre_transaction_check(pid, Jurisdiction::Us, dec!(5000));
        assert!(result.is_err());
    }

    #[test]
    fn test_elevated_kyc_required() {
        let svc = make_service();
        let pid = ParticipantId::new();
        submit_approved_kyc(&svc, pid, KycTier::Basic);
        add_clear_screening(&svc, pid);

        // $5000 → Basic is enough for US (elevated at $10,000)
        let result = svc.check_kyc_clearance(pid, Jurisdiction::Us, Some(dec!(5000)));
        assert!(result.is_ok());

        // $15,000 → Enhanced required
        let result = svc.check_kyc_clearance(pid, Jurisdiction::Us, Some(dec!(15000)));
        assert!(result.is_err());
    }

    #[test]
    fn test_monitoring_generates_alert() {
        let svc = make_service();
        let pid = ParticipantId::new();
        let tx = MonitoredTransaction {
            transaction_id: TransactionId::new(),
            participant_id: pid,
            counterparty_id: None,
            direction: TransactionDirection::Outbound,
            amount: dec!(25000),
            asset: AssetSymbol::Usdc,
            jurisdiction: Jurisdiction::Us,
            timestamp: Utc::now(),
            wallet_address: None,
            counterparty_address: None,
        };

        let alerts = svc.monitor_transaction(tx).unwrap();
        assert!(!alerts.is_empty());

        let stored = svc.get_alerts(pid);
        assert!(!stored.is_empty());
    }

    #[test]
    fn test_travel_rule_not_required_small_amount() {
        let svc = make_service();
        let originator = TransferParty {
            participant_id: ParticipantId::new(),
            name: Some("Alice".into()),
            wallet_address: "abc".into(),
            account_ref: Some("acc-1".into()),
            geographic_address: None,
            national_id: None,
            date_of_birth: None,
            place_of_birth: None,
        };
        let beneficiary = originator.clone();

        let result = svc
            .check_travel_rule(
                dec!(500),
                AssetSymbol::Usdc,
                Jurisdiction::Us,
                &originator,
                &beneficiary,
            )
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_travel_rule_required_large_amount() {
        let svc = make_service();
        let originator = TransferParty {
            participant_id: ParticipantId::new(),
            name: Some("Alice".into()),
            wallet_address: "abc".into(),
            account_ref: Some("acc-1".into()),
            geographic_address: Some("123 Main St".into()),
            national_id: None,
            date_of_birth: None,
            place_of_birth: None,
        };
        let beneficiary = TransferParty {
            participant_id: ParticipantId::new(),
            name: Some("Bob".into()),
            wallet_address: "def".into(),
            account_ref: Some("acc-2".into()),
            geographic_address: None,
            national_id: None,
            date_of_birth: None,
            place_of_birth: None,
        };

        let result = svc
            .check_travel_rule(
                dec!(5000),
                AssetSymbol::Usdc,
                Jurisdiction::Us,
                &originator,
                &beneficiary,
            )
            .unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_gdpr_consent_and_erasure() {
        let svc = make_service();
        let pid = ParticipantId::new();

        let consent = svc.record_consent(
            pid,
            DataCategory::Identity,
            LegalBasis::Consent,
            "KYC processing".into(),
            Jurisdiction::Eu,
        );
        assert!(consent.is_active());

        let erasure = svc.request_erasure(
            pid,
            vec![DataCategory::Identity, DataCategory::Behavioral],
            Jurisdiction::Eu,
            Utc::now() - chrono::Duration::days(100),
            false,
            false,
        );
        // Identity retained (< 5 years), Behavioral erased
        assert!(erasure
            .categories_completed
            .contains(&DataCategory::Behavioral));
    }
}
