use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FunPayError {
    #[error("unauthorized")]
    Unauthorized,
    #[error("request failed: {status}")]
    RequestFailed {
        status: StatusCode,
        body: String,
        url: String,
    },
    #[error("account not initiated")]
    AccountNotInitiated,
    #[error("parse error: {0}")]
    Parse(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("middleware: {0}")]
    Middleware(#[from] reqwest_middleware::Error),
}
