#[cfg(all(test, feature = "support-axum", feature = "support-tungstenite"))]
mod tests {
    use axum::{
        extract::ws::WebSocketUpgrade, http::StatusCode, response::Response, routing::get, Router,
    };
    use futures_util::{SinkExt, StreamExt};
    use std::net::SocketAddr;
    use tokio::net::TcpStream;
    use tokio_tungstenite::client_async;
    use tokio_tungstenite::tungstenite::Message;
    use yerpc::axum::handle_ws_rpc;
    use yerpc::tungstenite::tungstenite_client;
    use yerpc::{rpc, RpcClient, RpcSession};

    struct Api;

    impl Api {
        pub fn new() -> Self {
            Self
        }
    }

    #[rpc(all_positional, ts_outdir = "typescript/generated")]
    impl Api {
        async fn shout(&self, msg: String) -> String {
            msg.to_uppercase()
        }
        async fn add(&self, a: f32, b: f32) -> f32 {
            a + b
        }
    }

    async fn handler(ws: WebSocketUpgrade) -> Response {
        let (client, out_receiver) = RpcClient::new();
        let api = Api::new();
        let session = RpcSession::new(client, api);
        handle_ws_rpc(ws, out_receiver, session).await
    }

    #[tokio::test]
    async fn test_axum_websocket() -> anyhow::Result<()> {
        let app = Router::new().route("/rpc", get(handler));
        let addr = SocketAddr::from(([127, 0, 0, 1], 12345));
        let listener = std::net::TcpListener::bind(addr).unwrap();
        let server = axum::Server::from_tcp(listener).unwrap();
        tokio::spawn(async move {
            server.serve(app.into_make_service()).await.unwrap();
        });

        let tcp = TcpStream::connect("127.0.0.1:12345")
            .await
            .expect("Failed to connect");
        let url = url::Url::parse("ws://localhost:12345/rpc").unwrap();
        let (mut stream, response) = client_async(url, tcp)
            .await
            .expect("Client failed to connect");
        assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

        stream
            .send(Message::Text(
                r#"{"jsonrpc":"2.0","method":"shout","params":["foo"],"id":2}"#.into(),
            ))
            .await?;
        let res = stream.next().await.unwrap().unwrap();
        match res {
            Message::Text(text) => {
                assert_eq!(text, r#"{"jsonrpc":"2.0","id":2,"result":"FOO"}"#);
            }
            _ => panic!("Received unexepcted message {:?}", res),
        }

        let (client, _on_close) = tungstenite_client(stream, ());
        let res = client.send_request("add", Some([1.2, 2.3])).await?;
        let res: f32 = serde_json::from_value(res).unwrap();
        assert_eq!(res, 3.5);
        let res: String =
            serde_json::from_value(client.send_request("shout", Some(["foo"])).await?)?;
        assert_eq!(res.as_str(), "FOO");
        Ok(())
    }
}
