use async_std::stream::StreamExt;
use async_std::task;
use std::future::Future;
use std::sync::Arc;
use tide::{Endpoint, Request};
use tide_websockets::{Message as WsMessage, WebSocket};
use yerpc::{MessageHandle, RpcHandle, RpcHandler};

/// A Tide endpoint for a JSON-RPC 2.0 websocket.
///
/// The `handler` closure has to return a type that implements [yerpc::RpcHandler].
/// Either implement that manually or use `yerpc_derive::rpc`.
/// See the [webserver example](../../examples/webserver) for a usage example.
pub fn yerpc_handler<State, Sess, Fun, Fut>(handler: Fun) -> impl Endpoint<State>
where
    State: Send + Sync + Clone + 'static,
    Fun: Fn(Request<State>, RpcHandle) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = anyhow::Result<Sess>> + Send + 'static,
    Sess: RpcHandler,
{
    let handler = Arc::new(handler);
    WebSocket::new(move |request: Request<State>, mut stream| {
        let handler = handler.clone();
        async move {
            let (request_handle, mut rx) = RpcHandle::new();
            let session = (handler)(request, request_handle.clone()).await?;
            let handle = MessageHandle::new(request_handle, session);
            task::spawn({
                let stream = stream.clone();
                async move {
                    while let Some(message) = rx.next().await {
                        let message = serde_json::to_string(&message)?;
                        // Abort on error.
                        stream.send(WsMessage::Text(message)).await?;
                    }
                    let res: Result<(), anyhow::Error> = Ok(());
                    res
                }
            });
            while let Some(Ok(WsMessage::Text(input))) = stream.next().await {
                handle.handle_message(&input).await;
            }
            Ok(())
        }
    })
}
