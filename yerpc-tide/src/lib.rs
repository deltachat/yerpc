use async_std::stream::StreamExt;
use async_std::task;
use std::future::Future;
use std::sync::Arc;
use tide::{Endpoint, Request};
use tide_websockets::{Message as WsMessage, WebSocket};
use yerpc::{RpcClient, RpcServer, RpcSession};

/// A Tide endpoint for a JSON-RPC 2.0 websocket.
///
/// The `handler` closure has to return a type that implements [yerpc::RpcHandler].
/// Either implement that manually or use `yerpc_derive::rpc`.
/// See the [webserver example](../../examples/webserver) for a usage example.
pub fn yerpc_handler<State, Server, Fun, Fut>(handler: Fun) -> impl Endpoint<State>
where
    State: Send + Sync + Clone + 'static,
    Fun: Fn(Request<State>, RpcClient) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = anyhow::Result<Server>> + Send + 'static,
    Server: RpcServer,
{
    let handler = Arc::new(handler);
    WebSocket::new(move |request: Request<State>, mut stream| {
        let handler = handler.clone();
        async move {
            let (client, mut outgoing) = RpcClient::new();
            let server = (handler)(request, client.clone()).await?;
            let session = RpcSession::new(client, server);
            task::spawn({
                let stream = stream.clone();
                async move {
                    while let Some(message) = outgoing.next().await {
                        let message = serde_json::to_string(&message)?;
                        // Abort on error.
                        stream.send(WsMessage::Text(message)).await?;
                    }
                    let res: Result<(), anyhow::Error> = Ok(());
                    res
                }
            });
            while let Some(Ok(WsMessage::Text(input))) = stream.next().await {
                session.handle_incoming(&input).await;
            }
            Ok(())
        }
    })
}
