use anemo::{types::request::IntoRequest, Request};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Used by workers to send a new batch.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TssAnemoKeygenMessage {
    pub from_party_uid: String,
    pub is_broadcast: bool,
    pub payload: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TssAnemoKeygenRequest {
    pub message: TssAnemoKeygenMessage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TssAnemoKeygenResponse {
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TssAnemoSignRequest {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TssAnemoSignResponse {
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TssAnemoVerifyRequest {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TssAnemoVerifyResponse {
    pub message: String,
}
