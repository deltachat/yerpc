use crate::{OutReceiver, RpcServer, RpcSession};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures_util::StreamExt;

pub async fn handle_ws_rpc<T: RpcServer>(
    ws: WebSocketUpgrade,
    out_rx: OutReceiver,
    session: RpcSession<T>,
) -> Response {
    ws.on_upgrade(move |socket| async move {
        handle_rpc(socket, out_rx, session).await.ok();
    })
}

pub async fn handle_rpc<T: RpcServer>(
    mut socket: WebSocket,
    mut out_rx: OutReceiver,
    session: RpcSession<T>,
) -> anyhow::Result<()> {
    loop {
        tokio::select! {
            message = out_rx.next() => {
                    let message = serde_json::to_string(&message)?;
                    socket.send(Message::Text(message)).await?;
            }
            message = socket.next() => {
                match message {
                    Some(Ok(Message::Text(message))) => {
                        session.handle_incoming(&message).await;
                    },
                    Some(Ok(Message::Binary(_))) => {
                        return Err(anyhow::anyhow!("Binary messages are not supported."))
                    }
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        return Err(err.into())
                    }
                    None => break,
                }
            }
        }
    }
    Ok(())
}
