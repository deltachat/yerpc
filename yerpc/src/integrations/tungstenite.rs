use crate::{OutReceiver, RpcClient, RpcServer, RpcSession};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::oneshot,
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

pub fn tungstenite_client<R, S>(
    stream: WebSocketStream<S>,
    service: R,
) -> (RpcClient, oneshot::Receiver<anyhow::Result<()>>)
where
    R: RpcServer,
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let (client, out_rx) = RpcClient::new();
    let session = RpcSession::new(client.clone(), service);
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        let res = handle_tungstenite(stream, out_rx, session).await;
        let _ = tx.send(res);
    });
    (client, rx)
}

pub async fn handle_tungstenite<R, S>(
    mut stream: WebSocketStream<S>,
    mut out_rx: OutReceiver,
    session: RpcSession<R>,
) -> anyhow::Result<()>
where
    R: RpcServer,
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    loop {
        tokio::select! {
            message = out_rx.next() => {
                    let message = serde_json::to_string(&message)?;
                    stream.send(Message::Text(message)).await?;
            }
            message = stream.next() => {
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
