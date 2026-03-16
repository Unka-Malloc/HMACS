use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use carbide_core::TaskId;
use carbide_task::{CreateBid, CreateTask, DeliverTask};
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::middleware::AuthenticatedParticipant;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", post(create_task).get(list_tasks))
        .route("/{id}", get(get_task))
        .route("/{id}/bids", post(place_bid).get(list_bids))
        .route("/{id}/accept-bid/{bid_id}", post(accept_bid))
        .route("/{id}/start", post(start_task))
        .route("/{id}/deliver", post(deliver_task))
        .route("/{id}/approve", post(approve_delivery))
        .route("/{id}/dispute", post(dispute_delivery))
        .route("/{id}/cancel", post(cancel_task))
        .with_state(state)
}

async fn create_task(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Json(input): Json<CreateTask>,
) -> ApiResult<Json<carbide_task::Task>> {
    let task = state
        .task_service
        .create_task(auth.participant_id, input)
        .map_err(ApiError)?;
    Ok(Json(task))
}

async fn get_task(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<carbide_task::Task>> {
    let task = state
        .task_service
        .get_task(TaskId::from(id))
        .map_err(ApiError)?;
    Ok(Json(task))
}

async fn list_tasks(
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<carbide_task::Task>>> {
    let tasks = state.task_service.list_tasks(None);
    Ok(Json(tasks))
}

async fn place_bid(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Path(id): Path<Uuid>,
    Json(mut input): Json<CreateBid>,
) -> ApiResult<Json<carbide_task::Bid>> {
    input.task_id = TaskId::from(id);
    let bid = state
        .task_service
        .place_bid(auth.participant_id, input)
        .map_err(ApiError)?;
    Ok(Json(bid))
}

async fn list_bids(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Vec<carbide_task::Bid>>> {
    let bids = state.task_service.get_bids_for_task(TaskId::from(id));
    Ok(Json(bids))
}

async fn accept_bid(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Path((id, bid_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<carbide_task::Task>> {
    let (task, _) = state
        .task_service
        .accept_bid(
            TaskId::from(id),
            carbide_core::BidId::from(bid_id),
            auth.participant_id,
        )
        .map_err(ApiError)?;
    Ok(Json(task))
}

async fn start_task(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<carbide_task::Task>> {
    let task = state
        .task_service
        .start_task(TaskId::from(id), auth.participant_id)
        .map_err(ApiError)?;
    Ok(Json(task))
}

async fn deliver_task(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Path(id): Path<Uuid>,
    Json(input): Json<DeliverTask>,
) -> ApiResult<Json<carbide_task::Task>> {
    let task = state
        .task_service
        .deliver_task(TaskId::from(id), auth.participant_id, input)
        .map_err(ApiError)?;
    Ok(Json(task))
}

async fn approve_delivery(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<carbide_task::Task>> {
    let task = state
        .task_service
        .approve_delivery(TaskId::from(id), auth.participant_id)
        .map_err(ApiError)?;
    Ok(Json(task))
}

async fn dispute_delivery(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<carbide_task::Task>> {
    let task = state
        .task_service
        .dispute_delivery(TaskId::from(id), auth.participant_id)
        .map_err(ApiError)?;
    Ok(Json(task))
}

async fn cancel_task(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthenticatedParticipant>,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<carbide_task::Task>> {
    let task = state
        .task_service
        .cancel_task(TaskId::from(id), auth.participant_id)
        .map_err(ApiError)?;
    Ok(Json(task))
}
