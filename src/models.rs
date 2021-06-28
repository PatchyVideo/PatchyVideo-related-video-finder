use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use bson::oid::ObjectId;
use serde::{Serialize, Deserialize};
use thiserror::Error;

use crate::pvgql_models;

#[derive(Serialize)]
pub struct ErrorResponse {
    code: u16,
    error: String,
    message: String,
}

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Requested video was not found")]
    NotFound,
    #[error("Unknown Internal Error")]
    Unknown
}
impl ServiceError {
    pub fn name(&self) -> String {
        match self {
            Self::NotFound => "NotFound".to_string(),
            Self::Unknown => "Unknown".to_string(),
        }
    }
}
impl ResponseError for ServiceError {
    fn status_code(&self) -> StatusCode {
        match *self {
            Self::NotFound  => StatusCode::NOT_FOUND,
            Self::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = ErrorResponse {
            code: status_code.as_u16(),
            message: self.to_string(),
            error: self.name(),
        };
        HttpResponse::build(status_code).json(error_response)
    }
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertRequest {
    pub vid: ObjectId
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertResponse {
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub vid: ObjectId,
    pub uid: Option<ObjectId>,
    pub top_k: Option<i32>,
    pub threshold: Option<f32>,
    pub sort_title: Option<bool>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub videos: Vec<pvgql_models::Video>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialVideoItem {
    pub title: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialVideo {
    pub _id: ObjectId,
    pub tags: Vec<u64>,
    pub item: PartialVideoItem
}

