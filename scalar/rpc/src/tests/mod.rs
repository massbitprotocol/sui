pub mod tss_client;

#[cfg(test)]
mod tests {
    use crate::tests::tss_client::connect_tss_server;
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
    #[tokio::test]
    async fn run_tss_client() {
        let result = connect_tss_server().await;
        assert!(result.is_ok())
    }

    #[tokio::test]
    async fn run_grpc_client() {
        let result = connect_grpc_server().await;
        assert!(result.is_ok())
    }
}
