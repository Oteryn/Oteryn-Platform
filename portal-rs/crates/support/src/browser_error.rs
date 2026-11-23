use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub(crate) enum BrowserError {
    Status(StatusCode),
    Prepared(Box<Response>),
}

impl From<StatusCode> for BrowserError {
    fn from(status: StatusCode) -> Self {
        Self::Status(status)
    }
}

impl From<Response> for BrowserError {
    fn from(response: Response) -> Self {
        Self::Prepared(Box::new(response))
    }
}

impl IntoResponse for BrowserError {
    fn into_response(self) -> Response {
        match self {
            Self::Status(status) => status.into_response(),
            Self::Prepared(response) => *response,
        }
    }
}
