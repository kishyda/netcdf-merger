use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{Responder, Response};

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub struct ApiError {
    pub status: Status,
    pub message: String,
}

impl ApiError {
    pub fn new(status: Status, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(Status::BadRequest, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(Status::NotFound, message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(Status::Conflict, message)
    }

    pub fn internal<E: std::fmt::Display>(err: E) -> Self {
        Self {
            status: Status::InternalServerError,
            message: err.to_string(),
        }
    }
}

impl<E> From<E> for ApiError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        Self::internal(err)
    }
}

impl<'r> Responder<'r, 'static> for ApiError {
    fn respond_to(self, _req: &'r Request<'_>) -> rocket::response::Result<'static> {
        Response::build()
            .status(self.status)
            .sized_body(self.message.len(), std::io::Cursor::new(self.message))
            .ok()
    }
}
