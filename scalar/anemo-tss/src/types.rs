use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Deserialize, Serialize)]
pub struct KeygenRequest {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KeygenResponse {
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SignRequest {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SignResponse {
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VerifyRequest {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VerifyResponse {
    pub message: String,
}
