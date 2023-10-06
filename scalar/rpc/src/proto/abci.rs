#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CallContractRequest {
    #[prost(string, tag = "1")]
    pub sender: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub destination_chain: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub destination_contract_address: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "4")]
    pub payload_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub payload: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CallContractResponse {}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RequestArk {
    #[prost(string, tag = "1")]
    pub payload: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ScalarOutTransaction {
    #[prost(string, tag = "1")]
    pub message: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct KeygenOutput {
    #[prost(bytes = "vec", tag = "1")]
    pub pub_key: ::prost::alloc::vec::Vec<u8>,
}
/// ScalarAbciRequest is the request for ScalarAbci.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ScalarAbciRequest {
    #[prost(string, tag = "1")]
    pub payload: ::prost::alloc::string::String,
}
/// ScalarAbciResponse is the response for ScalarAbci.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ScalarAbciResponse {
    #[prost(oneof = "scalar_abci_response::Message", tags = "1, 2, 3")]
    pub message: ::core::option::Option<scalar_abci_response::Message>,
}
/// Nested message and enum types in `ScalarAbciResponse`.
pub mod scalar_abci_response {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Message {
        #[prost(message, tag = "1")]
        Ark(super::RequestArk),
        #[prost(message, tag = "2")]
        Tran(super::ScalarOutTransaction),
        #[prost(message, tag = "3")]
        Keygen(super::KeygenOutput),
    }
}
/// Generated client implementations.
pub mod scalar_abci_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// Echo is the echo service.
    #[derive(Debug, Clone)]
    pub struct ScalarAbciClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ScalarAbciClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ScalarAbciClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ScalarAbciClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            ScalarAbciClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// UnaryEcho is unary echo.
        pub async fn unary_scalar_abci(
            &mut self,
            request: impl tonic::IntoRequest<super::ScalarAbciRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ScalarAbciResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/abci.ScalarAbci/UnaryScalarAbci",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("abci.ScalarAbci", "UnaryScalarAbci"));
            self.inner.unary(req, path, codec).await
        }
        /// ServerStreamingEcho is server side streaming.
        pub async fn server_streaming_scalar_abci(
            &mut self,
            request: impl tonic::IntoRequest<super::ScalarAbciRequest>,
        ) -> std::result::Result<
            tonic::Response<tonic::codec::Streaming<super::ScalarAbciResponse>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/abci.ScalarAbci/ServerStreamingScalarAbci",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("abci.ScalarAbci", "ServerStreamingScalarAbci"));
            self.inner.server_streaming(req, path, codec).await
        }
        /// ClientStreamingEcho is client side streaming.
        pub async fn client_streaming_scalar_abci(
            &mut self,
            request: impl tonic::IntoStreamingRequest<Message = super::ScalarAbciRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ScalarAbciResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/abci.ScalarAbci/ClientStreamingScalarAbci",
            );
            let mut req = request.into_streaming_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("abci.ScalarAbci", "ClientStreamingScalarAbci"));
            self.inner.client_streaming(req, path, codec).await
        }
        /// BidirectionalStreamingScalarAbci is bidi streaming.
        pub async fn bidirectional_streaming_scalar_abci(
            &mut self,
            request: impl tonic::IntoStreamingRequest<Message = super::ScalarAbciRequest>,
        ) -> std::result::Result<
            tonic::Response<tonic::codec::Streaming<super::ScalarAbciResponse>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/abci.ScalarAbci/BidirectionalStreamingScalarAbci",
            );
            let mut req = request.into_streaming_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new(
                        "abci.ScalarAbci",
                        "BidirectionalStreamingScalarAbci",
                    ),
                );
            self.inner.streaming(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod scalar_abci_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with ScalarAbciServer.
    #[async_trait]
    pub trait ScalarAbci: Send + Sync + 'static {
        /// UnaryEcho is unary echo.
        async fn unary_scalar_abci(
            &self,
            request: tonic::Request<super::ScalarAbciRequest>,
        ) -> std::result::Result<
            tonic::Response<super::ScalarAbciResponse>,
            tonic::Status,
        >;
        /// Server streaming response type for the ServerStreamingScalarAbci method.
        type ServerStreamingScalarAbciStream: futures_core::Stream<
                Item = std::result::Result<super::ScalarAbciResponse, tonic::Status>,
            >
            + Send
            + 'static;
        /// ServerStreamingEcho is server side streaming.
        async fn server_streaming_scalar_abci(
            &self,
            request: tonic::Request<super::ScalarAbciRequest>,
        ) -> std::result::Result<
            tonic::Response<Self::ServerStreamingScalarAbciStream>,
            tonic::Status,
        >;
        /// ClientStreamingEcho is client side streaming.
        async fn client_streaming_scalar_abci(
            &self,
            request: tonic::Request<tonic::Streaming<super::ScalarAbciRequest>>,
        ) -> std::result::Result<
            tonic::Response<super::ScalarAbciResponse>,
            tonic::Status,
        >;
        /// Server streaming response type for the BidirectionalStreamingScalarAbci method.
        type BidirectionalStreamingScalarAbciStream: futures_core::Stream<
                Item = std::result::Result<super::ScalarAbciResponse, tonic::Status>,
            >
            + Send
            + 'static;
        /// BidirectionalStreamingScalarAbci is bidi streaming.
        async fn bidirectional_streaming_scalar_abci(
            &self,
            request: tonic::Request<tonic::Streaming<super::ScalarAbciRequest>>,
        ) -> std::result::Result<
            tonic::Response<Self::BidirectionalStreamingScalarAbciStream>,
            tonic::Status,
        >;
    }
    /// Echo is the echo service.
    #[derive(Debug)]
    pub struct ScalarAbciServer<T: ScalarAbci> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: ScalarAbci> ScalarAbciServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ScalarAbciServer<T>
    where
        T: ScalarAbci,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/abci.ScalarAbci/UnaryScalarAbci" => {
                    #[allow(non_camel_case_types)]
                    struct UnaryScalarAbciSvc<T: ScalarAbci>(pub Arc<T>);
                    impl<
                        T: ScalarAbci,
                    > tonic::server::UnaryService<super::ScalarAbciRequest>
                    for UnaryScalarAbciSvc<T> {
                        type Response = super::ScalarAbciResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ScalarAbciRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).unary_scalar_abci(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = UnaryScalarAbciSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/abci.ScalarAbci/ServerStreamingScalarAbci" => {
                    #[allow(non_camel_case_types)]
                    struct ServerStreamingScalarAbciSvc<T: ScalarAbci>(pub Arc<T>);
                    impl<
                        T: ScalarAbci,
                    > tonic::server::ServerStreamingService<super::ScalarAbciRequest>
                    for ServerStreamingScalarAbciSvc<T> {
                        type Response = super::ScalarAbciResponse;
                        type ResponseStream = T::ServerStreamingScalarAbciStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ScalarAbciRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).server_streaming_scalar_abci(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ServerStreamingScalarAbciSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/abci.ScalarAbci/ClientStreamingScalarAbci" => {
                    #[allow(non_camel_case_types)]
                    struct ClientStreamingScalarAbciSvc<T: ScalarAbci>(pub Arc<T>);
                    impl<
                        T: ScalarAbci,
                    > tonic::server::ClientStreamingService<super::ScalarAbciRequest>
                    for ClientStreamingScalarAbciSvc<T> {
                        type Response = super::ScalarAbciResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                tonic::Streaming<super::ScalarAbciRequest>,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).client_streaming_scalar_abci(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ClientStreamingScalarAbciSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.client_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/abci.ScalarAbci/BidirectionalStreamingScalarAbci" => {
                    #[allow(non_camel_case_types)]
                    struct BidirectionalStreamingScalarAbciSvc<T: ScalarAbci>(
                        pub Arc<T>,
                    );
                    impl<
                        T: ScalarAbci,
                    > tonic::server::StreamingService<super::ScalarAbciRequest>
                    for BidirectionalStreamingScalarAbciSvc<T> {
                        type Response = super::ScalarAbciResponse;
                        type ResponseStream = T::BidirectionalStreamingScalarAbciStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                tonic::Streaming<super::ScalarAbciRequest>,
                            >,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).bidirectional_streaming_scalar_abci(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = BidirectionalStreamingScalarAbciSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: ScalarAbci> Clone for ScalarAbciServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: ScalarAbci> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: ScalarAbci> tonic::server::NamedService for ScalarAbciServer<T> {
        const NAME: &'static str = "abci.ScalarAbci";
    }
}
