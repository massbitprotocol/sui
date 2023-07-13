use super::{codec as firehose, interceptors::MetricsInterceptor, stream_client::StreamClient};
use crate::blockchain::{
    block_stream::FirehoseCursor, Block as BlockchainBlock, BlockNumber, BlockPtr,
};
use crate::cheap_clone::CheapClone;
use crate::endpoint::{ConnectionType, EndpointMetrics, Provider, RequestLabels};
use crate::firehose::{
    decode_firehose_block, fetch_client::FetchClient, interceptors::AuthInterceptor,
};
use crate::substreams_rpc;
use futures03::StreamExt;
use slog::{debug, info, Logger};
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;
use tonic::codegen::http::uri::Scheme;
use tonic::codegen::{CompressionEncoding, InterceptedService};
use tonic::metadata::{Ascii, MetadataValue};
use tonic::transport::channel;
use tonic::transport::{Channel, ClientTlsConfig, Uri};
#[derive(Debug)]
pub struct FirehoseEndpoint {
    pub provider: Provider,
    pub auth: AuthInterceptor,
    pub filters_enabled: bool,
    pub compression_enabled: bool,
    endpoint_metrics: Arc<EndpointMetrics>,
    channel: Channel,
}

impl Display for FirehoseEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self.provider.as_str(), f)
    }
}

impl FirehoseEndpoint {
    pub fn new<S: AsRef<str>>(
        provider: S,
        url: S,
        token: Option<String>,
        filters_enabled: bool,
        compression_enabled: bool,
        endpoint_metrics: Arc<EndpointMetrics>,
    ) -> Self {
        let uri = url
            .as_ref()
            .parse::<Uri>()
            .expect("the url should have been validated by now, so it is a valid Uri");

        let endpoint_builder = match uri.scheme().unwrap_or(&Scheme::HTTP).as_str() {
            "http" => Channel::builder(uri),
            "https" => Channel::builder(uri)
                .tls_config(ClientTlsConfig::new())
                .expect("TLS config on this host is invalid"),
            _ => panic!("invalid uri scheme for firehose endpoint"),
        };

        // These tokens come from the config so they have to be ascii.
        let token: Option<MetadataValue<Ascii>> = token
            .map_or(Ok(None), |token| {
                let bearer_token = format!("bearer {}", token);
                bearer_token.parse::<MetadataValue<Ascii>>().map(Some)
            })
            .expect("Firehose token is invalid");

        // Note on the connection window size: We run multiple block streams on a same connection,
        // and a problematic subgraph with a stalled block stream might consume the entire window
        // capacity for its http2 stream and never release it. If there are enough stalled block
        // streams to consume all the capacity on the http2 connection, then _all_ subgraphs using
        // this same http2 connection will stall. At a default stream window size of 2^16, setting
        // the connection window size to the maximum of 2^31 allows for 2^15 streams without any
        // contention, which is effectively unlimited for normal graph node operation.
        //
        // Note: Do not set `http2_keep_alive_interval` or `http2_adaptive_window`, as these will
        // send ping frames, and many cloud load balancers will drop connections that frequently
        // send pings.
        let endpoint = endpoint_builder
            .initial_connection_window_size(Some((1 << 31) - 1))
            .connect_timeout(Duration::from_secs(10))
            .tcp_keepalive(Some(Duration::from_secs(15)))
            // Timeout on each request, so the timeout to estabilish each 'Blocks' stream.
            .timeout(Duration::from_secs(120));

        FirehoseEndpoint {
            provider: provider.as_ref().into(),
            channel: endpoint.connect_lazy(),
            auth: AuthInterceptor { token },
            filters_enabled,
            compression_enabled,
            endpoint_metrics,
        }
    }

    pub fn current_error_count(&self) -> u64 {
        self.endpoint_metrics.get_count(&self.provider)
    }

    // we need to -1 because there will always be a reference
    // inside FirehoseEndpoints that is not used (is always cloned).
    // pub fn get_capacity(self: &Arc<Self>) -> AvailableCapacity {
    //     self.subgraph_limit
    //         .get_capacity(Arc::strong_count(self).saturating_sub(1))
    // }

    fn new_client(
        &self,
    ) -> FetchClient<
        InterceptedService<MetricsInterceptor<Channel>, impl tonic::service::Interceptor>,
    > {
        let metrics = MetricsInterceptor {
            metrics: self.endpoint_metrics.cheap_clone(),
            service: self.channel.cheap_clone(),
            labels: RequestLabels {
                provider: self.provider.clone().into(),
                req_type: "unknown".into(),
                conn_type: ConnectionType::Firehose,
            },
        };

        let mut client: FetchClient<
            InterceptedService<MetricsInterceptor<Channel>, AuthInterceptor>,
        > = FetchClient::with_interceptor(metrics, self.auth.clone())
            .accept_compressed(CompressionEncoding::Gzip);

        if self.compression_enabled {
            client = client.send_compressed(CompressionEncoding::Gzip);
        }

        client
    }

    fn new_stream_client(
        &self,
    ) -> StreamClient<
        InterceptedService<MetricsInterceptor<Channel>, impl tonic::service::Interceptor>,
    > {
        let metrics = MetricsInterceptor {
            metrics: self.endpoint_metrics.cheap_clone(),
            service: self.channel.cheap_clone(),
            labels: RequestLabels {
                provider: self.provider.clone().into(),
                req_type: "unknown".into(),
                conn_type: ConnectionType::Firehose,
            },
        };

        let mut client = StreamClient::with_interceptor(metrics, self.auth.clone())
            .accept_compressed(CompressionEncoding::Gzip);

        if self.compression_enabled {
            client = client.send_compressed(CompressionEncoding::Gzip);
        }

        client
    }

    fn new_substreams_client(
        &self,
    ) -> substreams_rpc::stream_client::StreamClient<
        InterceptedService<MetricsInterceptor<Channel>, impl tonic::service::Interceptor>,
    > {
        let metrics = MetricsInterceptor {
            metrics: self.endpoint_metrics.cheap_clone(),
            service: self.channel.cheap_clone(),
            labels: RequestLabels {
                provider: self.provider.clone().into(),
                req_type: "unknown".into(),
                conn_type: ConnectionType::Substreams,
            },
        };

        let mut client = substreams_rpc::stream_client::StreamClient::with_interceptor(
            metrics,
            self.auth.clone(),
        )
        .accept_compressed(CompressionEncoding::Gzip);

        if self.compression_enabled {
            client = client.send_compressed(CompressionEncoding::Gzip);
        }

        client
    }

    pub async fn get_block<M>(
        &self,
        cursor: FirehoseCursor,
        logger: &Logger,
    ) -> Result<M, anyhow::Error>
    where
        M: prost::Message + BlockchainBlock + Default + 'static,
    {
        debug!(
            logger,
            "Connecting to firehose to retrieve block for cursor {}", cursor
        );

        let req = firehose::SingleBlockRequest {
            transforms: [].to_vec(),
            reference: Some(firehose::single_block_request::Reference::Cursor(
                firehose::single_block_request::Cursor {
                    cursor: cursor.to_string(),
                },
            )),
        };

        let mut client = self.new_client();
        match client.block(req).await {
            Ok(v) => Ok(M::decode(
                v.get_ref().block.as_ref().unwrap().value.as_ref(),
            )?),
            Err(e) => return Err(anyhow::format_err!("firehose error {}", e)),
        }
    }

    pub async fn genesis_block_ptr<M>(&self, logger: &Logger) -> Result<BlockPtr, anyhow::Error>
    where
        M: prost::Message + BlockchainBlock + Default + 'static,
    {
        info!(logger, "Requesting genesis block from firehose");

        // We use 0 here to mean the genesis block of the chain. Firehose
        // when seeing start block number 0 will always return the genesis
        // block of the chain, even if the chain's start block number is
        // not starting at block #0.
        self.block_ptr_for_number::<M>(logger, 0).await
    }

    pub async fn block_ptr_for_number<M>(
        &self,
        logger: &Logger,
        number: BlockNumber,
    ) -> Result<BlockPtr, anyhow::Error>
    where
        M: prost::Message + BlockchainBlock + Default + 'static,
    {
        debug!(
            logger,
            "Connecting to firehose to retrieve block for number {}", number
        );

        let mut client = self.new_stream_client();

        // The trick is the following.
        //
        // Firehose `start_block_num` and `stop_block_num` are both inclusive, so we specify
        // the block we are looking for in both.
        //
        // Now, the remaining question is how the block from the canonical chain is picked. We
        // leverage the fact that Firehose will always send the block in the longuest chain as the
        // last message of this request.
        //
        // That way, we either get the final block if the block is now in a final segment of the
        // chain (or probabilisticly if not finality concept exists for the chain). Or we get the
        // block that is in the longuest chain according to Firehose.
        let response_stream = client
            .blocks(firehose::Request {
                start_block_num: number as i64,
                stop_block_num: number as u64,
                final_blocks_only: false,
                ..Default::default()
            })
            .await?;

        let mut block_stream = response_stream.into_inner();

        debug!(logger, "Retrieving block(s) from firehose");

        let mut latest_received_block: Option<BlockPtr> = None;
        while let Some(message) = block_stream.next().await {
            match message {
                Ok(v) => {
                    let block = decode_firehose_block::<M>(&v)?.ptr();

                    match latest_received_block {
                        None => {
                            latest_received_block = Some(block);
                        }
                        Some(ref actual_ptr) => {
                            // We want to receive all events related to a specific block number,
                            // however, in some circumstances, it seems Firehose would not stop sending
                            // blocks (`start_block_num: 0 and stop_block_num: 0` on NEAR seems to trigger
                            // this).
                            //
                            // To prevent looping infinitely, we stop as soon as a new received block's
                            // number is higher than the latest received block's number, in which case it
                            // means it's an event for a block we are not interested in.
                            if block.number > actual_ptr.number {
                                break;
                            }

                            latest_received_block = Some(block);
                        }
                    }
                }
                Err(e) => return Err(anyhow::format_err!("firehose error {}", e)),
            };
        }

        match latest_received_block {
            Some(block_ptr) => Ok(block_ptr),
            None => Err(anyhow::format_err!(
                "Firehose should have returned at least one block for request"
            )),
        }
    }

    pub async fn stream_blocks(
        self: Arc<Self>,
        request: firehose::Request,
    ) -> Result<tonic::Streaming<firehose::Response>, anyhow::Error> {
        let mut client = self.new_stream_client();
        let response_stream = client.blocks(request).await?;
        let block_stream = response_stream.into_inner();

        Ok(block_stream)
    }

    pub async fn substreams(
        self: Arc<Self>,
        request: substreams_rpc::Request,
    ) -> Result<tonic::Streaming<substreams_rpc::Response>, anyhow::Error> {
        let mut client = self.new_substreams_client();
        let response_stream = client.blocks(request).await?;
        let block_stream = response_stream.into_inner();

        Ok(block_stream)
    }
}
