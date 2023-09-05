use crate::TssPeer;
use crate::{
    KeygenRequest, KeygenResponse, SignRequest, SignResponse, VerifyRequest, VerifyResponse,
};
use anemo::rpc::Status;
use anemo::Response;
#[derive(Default)]
pub struct TssPeerService {}

#[anemo::async_trait]
impl TssPeer for TssPeerService {
    async fn keygen(
        &self,
        request: anemo::Request<KeygenRequest>,
    ) -> Result<Response<KeygenResponse>, Status> {
        let reply = KeygenResponse {
            message: format!("Hello {}!", request.into_body().name),
        };
        Ok(Response::new(reply))
    }
    async fn sign(
        &self,
        request: anemo::Request<SignRequest>,
    ) -> Result<Response<SignResponse>, Status> {
        let reply = SignResponse {
            message: format!("Hello {}!", request.into_body().name),
        };
        Ok(Response::new(reply))
    }
    async fn verify(
        &self,
        request: anemo::Request<VerifyRequest>,
    ) -> Result<Response<VerifyResponse>, Status> {
        let reply = VerifyResponse {
            message: format!("Hello {}!", request.into_body().name),
        };
        Ok(Response::new(reply))
    }
    //     async fn say_hello(
    //         &self,
    //         request: Request<HelloRequest>,
    //     ) -> Result<Response<HelloResponse>, Status> {
    //         info!(
    //             "Got a request from {}",
    //             request.peer_id().unwrap().short_display(4)
    //         );

    //         let reply = HelloResponse {
    //             message: format!("Hello {}!", request.into_body().name),
    //         };

    //         Ok(Response::new(reply))

    //         // Example for a server handler using the `server_handler_return_raw_bytes` option.
    //         // let mut reply_bytes = bytes::BytesMut::new();
    //         // bincode::serialize_into(reply_bytes.as_mut().writer(), &reply)
    //         //     .map_err(|e| anemo::rpc::Status::from_error(e.into()))?;
    //         // Ok(Response::new(reply_bytes.freeze()))
    //     }
}
