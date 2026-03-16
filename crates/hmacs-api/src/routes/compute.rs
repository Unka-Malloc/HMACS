use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use hmacs_compute::{
    ComputeLease, ComputeListing, ComputeResource, CreateListing, PriceGuidance,
    RegisterResource, ResourceFilter,
};
use hmacs_core::{AssetSymbol, ComputeLeaseId};
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthenticatedParticipant;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/resources", post(register_resource).get(list_resources))
        .route("/listings", post(create_listing).get(list_listings))
        .route("/listings/{id}/purchase", post(purchase_lease))
        .route("/leases/{id}", get(get_lease))
        .route("/leases/{id}/usage", post(report_usage))
        .route("/guidance", get(get_price_guidance))
        .with_state(state)
}

async fn register_resource(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Json(input): Json<RegisterResource>,
) -> ApiResult<Json<ComputeResource>> {
    let resource = state
        .compute_service
        .register_resource(auth.participant_id, input)
        .map_err(ApiError)?;
    Ok(Json(resource))
}

async fn list_resources(
    State(state): State<AppState>,
    Query(filter): Query<ResourceFilter>,
) -> ApiResult<Json<Vec<ComputeResource>>> {
    let resources = state.compute_service.list_resources(&filter);
    Ok(Json(resources))
}

async fn create_listing(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Json(input): Json<CreateListing>,
) -> ApiResult<Json<ComputeListing>> {
    let listing = state
        .compute_service
        .create_listing(auth.participant_id, input)
        .map_err(ApiError)?;
    Ok(Json(listing))
}

async fn list_listings(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<ComputeListing>>> {
    let listings = state.compute_service.list_active_listings();
    Ok(Json(listings))
}

async fn purchase_lease(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Path(id): Path<Uuid>,
    Json(body): Json<PurchaseRequest>,
) -> ApiResult<Json<ComputeLease>> {
    let lease = state
        .compute_service
        .purchase_lease(auth.participant_id, id, body.units)
        .map_err(ApiError)?;
    Ok(Json(lease))
}

#[derive(Debug, Deserialize)]
struct PurchaseRequest {
    units: Option<Decimal>,
}

async fn get_lease(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<ComputeLease>> {
    let lease = state
        .compute_service
        .get_lease(ComputeLeaseId::from(id))
        .map_err(ApiError)?;
    Ok(Json(lease))
}

#[derive(Debug, Deserialize)]
struct UsageReport {
    units: Decimal,
}

async fn report_usage(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Path(id): Path<Uuid>,
    Json(body): Json<UsageReport>,
) -> ApiResult<Json<ComputeLease>> {
    let lease = state
        .compute_service
        .report_usage(ComputeLeaseId::from(id), body.units, auth.participant_id)
        .map_err(ApiError)?;
    Ok(Json(lease))
}

#[derive(Debug, Deserialize)]
struct GuidanceQuery {
    unit_label: String,
    asset: AssetSymbol,
}

async fn get_price_guidance(
    State(state): State<AppState>,
    Query(q): Query<GuidanceQuery>,
) -> ApiResult<Json<PriceGuidance>> {
    let guidance = state.compute_service.get_price_guidance(&q.unit_label, q.asset);
    Ok(Json(guidance))
}
