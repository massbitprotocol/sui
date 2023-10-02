use anemo::{types::request::IntoRequest, Request};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Used by workers to send a new batch.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TssAnemoDeliveryMessage {
    pub from_party_uid: String,
    pub is_broadcast: bool,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TssAnemoKeygenRequest {
    pub message: TssAnemoDeliveryMessage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TssAnemoKeygenResponse {
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TssAnemoSignRequest {
    pub message: TssAnemoDeliveryMessage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TssAnemoSignResponse {
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TssAnemoVerifyRequest {
    pub message: TssAnemoDeliveryMessage,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TssAnemoVerifyResponse {
    pub message: String,
}

pub mod gg20 {
    use crate::KeygenInit;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
    pub struct DeliveryMessage {
        pub from_party_uid: String,
        pub is_broadcast: bool,
        pub payload: Vec<u8>,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct KeygenRequest {
        pub message: DeliveryMessage,
    }

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct KeygenResponse {
        pub message: String,
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RecoverRequest {
        //pub keygen_init: KeygenInit,
    }
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct RecoverResponse {}
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SignRequest {}
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SignResponse {}
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct KeyPresenceRequest {}
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct KeyPresenceResponse {}
}

pub mod multisig {
    use serde::{Deserialize, Serialize};
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct KeyPresenceRequest {}
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct KeyPresenceResponse {}
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct KeygenRequest {}
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct KeygenResponse {}
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SignRequest {}
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SignResponse {}
}
