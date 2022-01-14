

use async_std::{prelude::*};

use serde::{Deserialize};

use std::sync::Arc;
use tide::Request;
use tide_websockets::{Message, WebSocket};

// use deltachat_jsonrpc::rpc;
// use deltachat_jsonrpc_derive::rpc_derive;

use jsonrpc::{rpc};

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

#[derive(Deserialize, Debug)]
struct SumParams {
    a: usize,
    b: usize,
}

#[rpc]
impl Session {
    pub async fn sum(&self, SumParams { a, b }: SumParams) -> Result<usize, jsonrpc::Error> {
        Ok(a + b)
    }

    pub async fn sum2(&self, (a, b): (usize, usize)) -> Result<usize, jsonrpc::Error> {
        Ok(a + b)
    }

    #[rpc(notification)]
    pub async fn onevent(&self, ev: serde_json::Value) -> Result<(), jsonrpc::Error> {
        eprintln!("notif: {:?}", ev);
        Ok(())
    }
}

// #[async_trait]
// impl RpcHandler for Session {
//     async fn on_notification(
//         &self,
//         method: String,
//         params: serde_json::Value,
//     ) -> Result<(), jsonrpc::Error> {
//         eprintln!("on_notification`{}` {}", method, params);
//         Ok(())
//     }
//     async fn on_request(
//         &self,
//         method: String,
//         params: serde_json::Value,
//     ) -> Result<serde_json::Value, jsonrpc::Error> {
//         eprintln!("on_request `{}` {}", method, params);
//         Err(jsonrpc::Error::new(
//             jsonrpc::Error::METHOD_NOT_FOUND,
//             "Method not found",
//         ))
//     }
// }

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init();
    let state = State::new();
    let state = Arc::new(state);

    let mut app = tide::with_state(state);

    app.at("/ws").get(WebSocket::new(
        |request: Request<Arc<State>>, mut stream| async move {
            let session = Session::new(request.state().clone());
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
