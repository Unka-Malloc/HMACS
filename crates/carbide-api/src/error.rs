use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use carbide_core::CarbideError;
use serde_json::json;

pub struct ApiError(pub CarbideError);

impl From<CarbideError> for ApiError {
    fn from(err: CarbideError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.0.status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let body = json!({
            "error": {
                "code": status.as_u16(),
                "message": self.0.to_string(),
            }
        });

        (status, Json(body)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;
