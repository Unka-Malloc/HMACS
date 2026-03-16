use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use carbide_compliance::{
    ComplianceAlert, ConsentRecord, DataCategory, ErasureRequest, Jurisdiction,
    KycRecord, LegalBasis, RiskAssessment, ScreeningRecord,
    SubmitKycRequest, TravelRuleMessage, TransferParty,
};
use carbide_core::AssetSymbol;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthenticatedParticipant;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        // KYC
        .route("/kyc", post(submit_kyc).get(get_my_kyc))
        .route("/kyc/check", post(check_kyc_clearance))
        // AML / Sanctions
        .route("/screening", post(screen_participant))
        // Risk
        .route("/risk", get(get_my_risk))
        .route("/risk/assess", post(assess_risk))
        // Transaction monitoring
        .route("/alerts", get(get_my_alerts))
        .route("/alerts/open", get(get_open_alerts))
        // Travel rule
        .route("/travel-rule/check", post(check_travel_rule))
        // GDPR
        .route("/consent", post(record_consent))
        .route("/erasure", post(request_erasure))
        // Pre-transaction composite check
        .route("/pre-check", post(pre_transaction_check))
        // Jurisdiction info
        .route("/jurisdictions/{code}", get(get_jurisdiction_rules))
        .with_state(state)
}

// ──────────────────── KYC ────────────────────

async fn submit_kyc(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Json(input): Json<SubmitKycRequest>,
) -> ApiResult<Json<KycRecord>> {
    let record = state
        .compliance_service
        .submit_kyc(auth.participant_id, input)
        .map_err(ApiError)?;
    Ok(Json(record))
}

async fn get_my_kyc(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
) -> ApiResult<Json<Option<KycRecord>>> {
    let record = state.compliance_service.get_kyc(auth.participant_id);
    Ok(Json(record))
}

#[derive(Debug, Deserialize)]
struct KycCheckRequest {
    jurisdiction: Jurisdiction,
    amount_usdc: Option<Decimal>,
}

async fn check_kyc_clearance(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Json(req): Json<KycCheckRequest>,
) -> ApiResult<Json<ClearanceResult>> {
    match state.compliance_service.check_kyc_clearance(
        auth.participant_id,
        req.jurisdiction,
        req.amount_usdc,
    ) {
        Ok(()) => Ok(Json(ClearanceResult {
            cleared: true,
            reason: None,
        })),
        Err(e) => Ok(Json(ClearanceResult {
            cleared: false,
            reason: Some(e.to_string()),
        })),
    }
}

#[derive(Debug, Serialize)]
struct ClearanceResult {
    cleared: bool,
    reason: Option<String>,
}

// ──────────────────── AML ────────────────────

#[derive(Debug, Deserialize)]
struct ScreeningRequest {
    name: Option<String>,
    wallet_address: Option<String>,
    jurisdiction: Jurisdiction,
}

async fn screen_participant(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Json(req): Json<ScreeningRequest>,
) -> ApiResult<Json<ScreeningRecord>> {
    let record = state
        .compliance_service
        .screen_participant(
            auth.participant_id,
            req.name,
            req.wallet_address,
            req.jurisdiction,
        )
        .map_err(ApiError)?;
    Ok(Json(record))
}

// ──────────────────── Risk ────────────────────

#[derive(Debug, Deserialize)]
struct AssessRiskRequest {
    jurisdiction: Jurisdiction,
}

async fn get_my_risk(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
) -> ApiResult<Json<Option<RiskAssessment>>> {
    let assessment = state
        .compliance_service
        .get_risk_assessment(auth.participant_id);
    Ok(Json(assessment))
}

async fn assess_risk(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Json(req): Json<AssessRiskRequest>,
) -> ApiResult<Json<RiskAssessment>> {
    let assessment = state
        .compliance_service
        .assess_risk(auth.participant_id, req.jurisdiction);
    Ok(Json(assessment))
}

// ──────────────────── Monitoring ────────────────────

async fn get_my_alerts(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
) -> ApiResult<Json<Vec<ComplianceAlert>>> {
    let alerts = state.compliance_service.get_alerts(auth.participant_id);
    Ok(Json(alerts))
}

async fn get_open_alerts(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ComplianceAlert>>> {
    let alerts = state.compliance_service.get_all_open_alerts();
    Ok(Json(alerts))
}

// ──────────────────── Travel Rule ────────────────────

#[derive(Debug, Deserialize)]
struct TravelRuleCheckRequest {
    amount_usdc_equivalent: Decimal,
    asset: AssetSymbol,
    jurisdiction: Jurisdiction,
    originator: TransferParty,
    beneficiary: TransferParty,
}

async fn check_travel_rule(
    State(state): State<AppState>,
    Json(req): Json<TravelRuleCheckRequest>,
) -> ApiResult<Json<Option<TravelRuleMessage>>> {
    let message = state
        .compliance_service
        .check_travel_rule(
            req.amount_usdc_equivalent,
            req.asset,
            req.jurisdiction,
            &req.originator,
            &req.beneficiary,
        )
        .map_err(ApiError)?;
    Ok(Json(message))
}

// ──────────────────── GDPR ────────────────────

#[derive(Debug, Deserialize)]
struct ConsentRequest {
    category: DataCategory,
    legal_basis: LegalBasis,
    purpose: String,
    jurisdiction: Jurisdiction,
}

async fn record_consent(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Json(req): Json<ConsentRequest>,
) -> ApiResult<Json<ConsentRecord>> {
    let record = state.compliance_service.record_consent(
        auth.participant_id,
        req.category,
        req.legal_basis,
        req.purpose,
        req.jurisdiction,
    );
    Ok(Json(record))
}

#[derive(Debug, Deserialize)]
struct ErasureRequestBody {
    categories: Vec<DataCategory>,
    jurisdiction: Jurisdiction,
}

async fn request_erasure(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Json(req): Json<ErasureRequestBody>,
) -> ApiResult<Json<ErasureRequest>> {
    let result = state.compliance_service.request_erasure(
        auth.participant_id,
        req.categories,
        req.jurisdiction,
        Utc::now() - chrono::Duration::days(365), // placeholder; pull from DB in production
        false,
        false,
    );
    Ok(Json(result))
}

use chrono::Utc;

// ──────────────────── Composite Check ────────────────────

#[derive(Debug, Deserialize)]
struct PreCheckRequest {
    jurisdiction: Jurisdiction,
    amount_usdc: Decimal,
}

async fn pre_transaction_check(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Json(req): Json<PreCheckRequest>,
) -> ApiResult<Json<ClearanceResult>> {
    match state.compliance_service.pre_transaction_check(
        auth.participant_id,
        req.jurisdiction,
        req.amount_usdc,
    ) {
        Ok(()) => Ok(Json(ClearanceResult {
            cleared: true,
            reason: None,
        })),
        Err(e) => Ok(Json(ClearanceResult {
            cleared: false,
            reason: Some(e.to_string()),
        })),
    }
}

// ──────────────────── Jurisdiction Info ────────────────────

async fn get_jurisdiction_rules(
    Path(code): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let jurisdiction = match code.to_uppercase().as_str() {
        "US" => Jurisdiction::Us,
        "EU" => Jurisdiction::Eu,
        "HK" => Jurisdiction::Hk,
        "SG" => Jurisdiction::Sg,
        "AE" => Jurisdiction::Ae,
        _ => {
            return Err(ApiError(carbide_core::CarbideError::InvalidInput(format!(
                "Unknown jurisdiction: {}",
                code
            ))));
        }
    };

    let rules = carbide_compliance::JurisdictionRules::for_jurisdiction(jurisdiction);
    let json = serde_json::to_value(rules)
        .map_err(|e| ApiError(carbide_core::CarbideError::Internal(e.to_string())))?;
    Ok(Json(json))
}
