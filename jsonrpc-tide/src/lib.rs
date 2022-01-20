use async_std::stream::StreamExt;
use async_std::task;
use jsonrpc::{MessageHandle, RpcHandle, RpcHandler};
use std::future::Future;
use std::sync::Arc;
use tide::{Endpoint, Request};
use tide_websockets::{Message as WsMessage, WebSocket};

pub fn jsonrpc_handler<State, Sess, Fun, Fut>(handler: Fun) -> impl Endpoint<State>
where
    State: Send + Sync + Clone + 'static,
    // Fun: Fn(State, RequestHandle) -> anyhow::Result<Sess> + Send + Sync + 'static,
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
                        // eprintln!("SEND OUT {:?}", message);
                        let message = serde_json::to_string(&message)?;
                        stream.send(WsMessage::Text(message)).await?;
                    }
                    let res: Result<(), anyhow::Error> = Ok(());
                    res
                }
            });
            while let Some(Ok(WsMessage::Text(input))) = stream.next().await {
                // eprintln!("RECV IN {:?}", input);
                handle.handle_message(&input).await;
            }
            Ok(())
        }
    })
}
