use crate::{OutReceiver, RpcServer, RpcSession};
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use tokio::task::JoinHandle;

pub async fn handle_ws_rpc<T: RpcServer>(
    ws: WebSocketUpgrade,
    out_rx: OutReceiver,
    session: RpcSession<T>,
) -> Response {
    ws.on_upgrade(move |socket| async move {
        match handle_rpc(socket, out_rx, session).await {
            Ok(()) => {}
            Err(err) => tracing::warn!("yerpc websocket closed with error {err:?}"),
        }
    })
}

pub async fn handle_rpc<T: RpcServer>(
    socket: WebSocket,
    mut out_rx: OutReceiver,
    session: RpcSession<T>,
) -> anyhow::Result<()> {
    let (mut sender, mut receiver) = socket.split();
    let send_task: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        while let Some(message) = out_rx.next().await {
            let message = serde_json::to_string(&message)?;
            tracing::trace!("RPC send {}", message);
            sender.send(Message::Text(message)).await?;
        }
        Ok(())
    });
    let recv_task: JoinHandle<anyhow::Result<()>> = tokio::spawn(async move {
        while let Some(message) = receiver.next().await {
            match message {
                Ok(Message::Text(message)) => {
                    tracing::trace!("RPC recv {}", message);
                    session.handle_incoming(&message).await;
                }
                Ok(Message::Binary(_)) => {
                    return Err(anyhow::anyhow!("Binary messages are not supported."))
                }
                Ok(_) => {}
                Err(err) => return Err(anyhow::anyhow!(err)),
            }
        }
        Ok(())
    });
    recv_task.await??;
    send_task.await??;
    Ok(())
}
