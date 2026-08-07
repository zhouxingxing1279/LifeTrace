//! Uniform API error type mapped to the contract `ApiErrorV1`.

use std::fmt;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use lifetrace_contracts::{ApiErrorV1, ErrorCode};

/// API error with an HTTP status and the stable contract error body.
///
/// The contract body is boxed so `Result<T, ApiError>` remains compact across
/// hot handler and repository paths.
#[derive(Debug, Clone)]
pub struct ApiError {
    pub http_status: StatusCode,
    pub body: Box<ApiErrorV1>,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>, http_status: StatusCode) -> Self {
        Self {
            http_status,
            body: Box::new(ApiErrorV1::new(code, message)),
        }
    }

    pub fn with_request_id(mut self, request_id: Option<lifetrace_contracts::RequestId>) -> Self {
        self.body.request_id = request_id;
        self
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.body.code, self.body.message)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.http_status, Json(*self.body)).into_response()
    }
}

impl From<ApiError> for Response {
    fn from(value: ApiError) -> Self {
        value.into_response()
    }
}
