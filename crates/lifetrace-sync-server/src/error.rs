//! Uniform API error type mapped to the contract `ApiErrorV1`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use lifetrace_contracts::{ApiErrorV1, ErrorCode};

/// API error with an HTTP status and the stable contract error body.
#[derive(Debug, Clone)]
pub struct ApiError {
    pub http_status: StatusCode,
    pub body: ApiErrorV1,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>, http_status: StatusCode) -> Self {
        Self {
            http_status,
            body: ApiErrorV1::new(code, message),
        }
    }

    pub fn with_request_id(mut self, request_id: Option<lifetrace_contracts::RequestId>) -> Self {
        self.body.request_id = request_id;
        self
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.http_status, Json(self.body)).into_response()
    }
}

impl From<ApiError> for Response {
    fn from(value: ApiError) -> Self {
        value.into_response()
    }
}
