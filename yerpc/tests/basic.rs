use async_std::stream::StreamExt;
use yerpc::{rpc, RpcSession};

#[async_std::test]
async fn basic() -> anyhow::Result<()> {
    assert_eq!(1, 1, "it works");

    struct Api {}

    #[rpc(all_positional)]
    impl Api {
        pub async fn upper(&self, text: String) -> String {
            text.to_uppercase()
        }
    }

    let (session, mut out_rx) = RpcSession::create(Api {});

    let req = r#"{"jsonrpc":"2.0","method":"upper","params":["foo"],"id":1}"#;
    session.handle_incoming(req).await;
    let out = out_rx.next().await.unwrap();
    let out = serde_json::to_string(&out).unwrap();
    assert_eq!(out, r#"{"jsonrpc":"2.0","id":1,"result":"FOO"}"#);
    let client = session.client().clone();
    async_std::task::spawn(async move {
        let out = out_rx.next().await.unwrap();
        let out = serde_json::to_string(&out).unwrap();
        assert_eq!(
            out,
            r#"{"jsonrpc":"2.0","method":"bar","params":["woo"],"id":0}"#
        );
        session
            .handle_incoming(r#"{"jsonrpc":"2.0","id":0,"result":"boo"}"#)
            .await;
    });
    let res = client.send_request("bar", Some(&["woo"])).await.unwrap();
    assert_eq!(res, "boo");
    Ok(())
}
