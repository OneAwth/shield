use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use bcrypt::BcryptError;
use jsonwebtoken;
use sea_orm::{DbErr, TransactionError};
use serde_json::json;
use std::io;
use tokio::task::JoinError;
use tracing::error;

#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum Error {
    #[error("{0}")]
    Authenticate(#[from] AuthenticateError),

    #[error("{0}")]
    BadRequest(#[from] BadRequestError),

    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Transaction error: {0}")]
    DbTransaction(Box<dyn std::error::Error + Send + Sync>, StatusCode),

    #[error("{0}")]
    NotFound(#[from] NotFoundError),

    #[error("{0}")]
    RunSyncTask(#[from] JoinError),

    #[error("{0}")]
    HashPassword(#[from] BcryptError),

    #[error("{0}")]
    File(#[from] FileError),

    #[error("{0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

impl Error {
    fn get_codes(&self) -> (StatusCode, u16) {
        match self {
            // 4XX Errors
            Error::BadRequest(err) => err.get_codes(),
            Error::NotFound(_) => (StatusCode::NOT_FOUND, 40003),
            Error::Authenticate(err) => err.get_codes(),
            Error::Jwt(_) => (StatusCode::UNAUTHORIZED, 40012), // New JWT error code

            // 5XX Errors
            Error::RunSyncTask(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5005),
            Error::HashPassword(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5006),
            Error::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5007),
            Error::File(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5008),
            Error::SerdeJson(_) => (StatusCode::INTERNAL_SERVER_ERROR, 5009),
            Error::DbTransaction(_, status_code) => (*status_code, 5010),
        }
    }

    pub fn not_found() -> Self {
        Error::NotFound(NotFoundError::Generic)
    }

    pub fn cannot_perform_operation(message: &str) -> Self {
        Error::BadRequest(BadRequestError::CannotPerformOperation(message.to_string()))
    }

    pub fn log(&self) {
        let (status, code) = self.get_codes();
        error!("Error {}/{}: {}", status.as_u16(), code, self);
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        self.log();
        let (status_code, code) = self.get_codes();
        let message = self.to_string();
        let body = Json(json!({ "code": code, "message": message }));

        (status_code, body).into_response()
    }
}

#[derive(thiserror::Error, Debug)]
#[error("...")]
pub enum AuthenticateError {
    #[error("Wrong authentication credentials")]
    WrongCredentials,
    #[error("Max concurrent sessions reached")]
    MaxConcurrentSessions,
    #[error("Invalid authentication credentials")]
    InvalidToken,
    #[error("Invalid api credentials")]
    InvalidApiCredentials,
    #[error("Inappropriate resource access")]
    NoResource,
    #[error("Action forbidden")]
    ActionForbidden,
    #[error("User is locked")]
    Locked,
}

impl AuthenticateError {
    fn get_codes(&self) -> (StatusCode, u16) {
        match self {
            AuthenticateError::WrongCredentials => (StatusCode::UNAUTHORIZED, 40004),
            AuthenticateError::InvalidToken => (StatusCode::UNAUTHORIZED, 40005),
            AuthenticateError::Locked => (StatusCode::LOCKED, 40006),
            AuthenticateError::NoResource => (StatusCode::FORBIDDEN, 40008),
            AuthenticateError::ActionForbidden => (StatusCode::FORBIDDEN, 40009),
            AuthenticateError::MaxConcurrentSessions => (StatusCode::FORBIDDEN, 40010),
            AuthenticateError::InvalidApiCredentials => (StatusCode::UNAUTHORIZED, 40011),
        }
    }
}

impl From<TransactionError<DbErr>> for Error {
    fn from(err: TransactionError<DbErr>) -> Self {
        match err {
            TransactionError::Connection(db_err) => Error::DbTransaction(Box::new(db_err), StatusCode::INTERNAL_SERVER_ERROR),
            TransactionError::Transaction(db_err) => {
                let status_code = match db_err {
                    DbErr::RecordNotFound(_) => StatusCode::NOT_FOUND,
                    DbErr::Custom(ref e) if e.contains("duplicate key value") => StatusCode::CONFLICT,
                    _ => StatusCode::INTERNAL_SERVER_ERROR,
                };
                Error::DbTransaction(Box::new(db_err), status_code)
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BadRequestError {
    #[error("Cannot perform operation: {0}")]
    CannotPerformOperation(String),
}

impl BadRequestError {
    fn get_codes(&self) -> (StatusCode, u16) {
        (StatusCode::BAD_REQUEST, 40002)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum NotFoundError {
    #[error("Not found")]
    Generic,
    #[error("Session not found")]
    SessionNotFound,
    #[error("User not found")]
    UserNotFound,
    #[error("Client not found")]
    ClientNotFound,
    #[error("Resource Group not found")]
    ResourceGroupNotFound,
    #[error("Resource not found")]
    ResourceNotFound,
}

#[derive(thiserror::Error, Debug)]
#[error("Database error: {0}")]
pub struct DatabaseError(#[from] Box<dyn std::error::Error + Send + Sync>);

#[derive(thiserror::Error, Debug)]
#[error("File error: {0}")]
pub struct FileError(#[from] io::Error);

impl From<DbErr> for Error {
    fn from(err: DbErr) -> Self {
        Error::Database(DatabaseError(Box::new(err)))
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::File(FileError(err))
    }
}
