pub mod tofnd;
use prost::Message;
use serde::Serializer;
pub use tofnd::*;
impl serde::Serialize for KeygenRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = Message::encode_to_vec(self);
        S::serialize_bytes(bytes.as_slice(), v)
    }
}
