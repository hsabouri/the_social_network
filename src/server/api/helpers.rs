use std::error::Error;

use tonic::Status;

pub trait ErrorStatus {
    fn error_internal(error: impl std::fmt::Display) -> Status {
        Status::internal(format!("{error}"))
    }

    fn error_invalid_argument(error: impl std::fmt::Display) -> Status {
        Status::invalid_argument(format!("{error}"))
    }
}

impl ErrorStatus for Status {}