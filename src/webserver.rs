use async_std::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use typescript_type_def::TypeDef;

use std::sync::Arc;
use tide::Request;
use tide_websockets::{Message, WebSocket};

use jsonrpc::{rpc, RpcHandler};

struct State {
    foo: usize,
}
impl State {
    pub fn new() -> Self {
        Self { foo: 0 }
    }
}

struct Session {
    state: Arc<State>,
}
impl Session {
    pub fn new(state: Arc<State>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl RpcHandler for Session {
    async fn on_notification(
        &self,
        method: String,
        params: serde_json::Value,
    ) -> Result<(), jsonrpc::Error> {
        RpcApi.on_notification(method, params).await
    }
    async fn on_request(
        &self,
        method: String,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, jsonrpc::Error> {
        RpcApi.on_request(method, params).await
        // Err(Error::new(Error::METHOD_NOT_FOUND, "Method not found"))
    }
}

#[derive(Deserialize, Debug, TypeDef)]
struct SumParams {
    a: usize,
    b: usize,
}

#[derive(Deserialize, Debug, TypeDef)]
struct Sum2Params(usize, usize);

struct RpcApi;

#[rpc]
impl RpcApi {
    pub async fn sum(&self, SumParams { a, b }: SumParams) -> jsonrpc::Result<usize> {
        Ok(a + b)
    }

    pub async fn sum2(&self, Sum2Params(a, b): Sum2Params) -> jsonrpc::Result<usize> {
        Ok(a + b)
    }

    pub async fn square(&self, num: (f32,)) -> jsonrpc::Result<f32> {
        Ok(num.0 * num.0)
    }

    #[rpc(positional)]
    pub async fn nothing(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    #[rpc(positional)]
    pub async fn many_args(&self, a: usize, b: Vec<String>) -> jsonrpc::Result<()> {
        eprintln!("called with {} and {:?}", a, b);
        Ok(())
    }

    #[rpc(notification)]
    pub async fn onevent(&self, ev: serde_json::Value) -> jsonrpc::Result<()> {
        eprintln!("notif: {:?}", ev);
        Ok(())
    }

    #[rpc(name = "yell", positional)]
    pub async fn shout(&self, message: String) -> jsonrpc::Result<String> {
        Ok(message.to_uppercase())
    }
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init();
    let state = Arc::new(State::new());
    let mut app = tide::with_state(state);

    app.at("/ws").get(WebSocket::new(
        |request: Request<Arc<State>>, mut stream| async move {
            let session = Session::new(request.state().clone());
            stream
                .send_json(&jsonrpc::Request {
                    jsonrpc: jsonrpc::Version::V2,
                    params: Some(jsonrpc::Params::Positional(vec![serde_json::to_value(
                        "hello",
                    )
                    .unwrap()])),
                    method: "sum2".to_string(),
                    id: Some(1),
                })
                .await?;
            while let Some(Ok(Message::Text(input))) = stream.next().await {
                if let Some(res) = jsonrpc::handle_message(&session, &input).await {
                    stream.send(Message::Text(res)).await?;
                }
            }
            Ok(())
        },
    ));

    app.listen("127.0.0.1:20808").await?;

    Ok(())
}
