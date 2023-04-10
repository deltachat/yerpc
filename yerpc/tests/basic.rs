use futures_util::StreamExt;
use yerpc::{rpc, RpcSession};

#[tokio::test]
async fn basic() -> anyhow::Result<()> {
    struct Api {}

    #[rpc(all_positional, ts_outdir = "typescript/generated")]
    impl Api {
        pub async fn constant(&self) -> String {
            "example".to_string()
        }

        pub async fn upper(&self, text: String) -> String {
            text.to_uppercase()
        }
    }

    let (session, mut out_rx) = RpcSession::create(Api {});

    let req = r#"{"jsonrpc":"2.0","method":"constant","id":3}"#;
    session.handle_incoming(req).await;
    let out = out_rx.next().await.unwrap();
    let out = serde_json::to_string(&out).unwrap();
    assert_eq!(out, r#"{"jsonrpc":"2.0","id":3,"result":"example"}"#);

    let req = r#"{"jsonrpc":"2.0","method":"upper","params":["foo"],"id":7}"#;
    session.handle_incoming(req).await;
    let out = out_rx.next().await.unwrap();
    let out = serde_json::to_string(&out).unwrap();
    assert_eq!(out, r#"{"jsonrpc":"2.0","id":7,"result":"FOO"}"#);

    let client = session.client().clone();
    tokio::spawn(async move {
        let out = out_rx.next().await.unwrap();
        let out = serde_json::to_string(&out).unwrap();
        assert_eq!(
            out,
            r#"{"jsonrpc":"2.0","method":"bar","params":["woo"],"id":1}"#
        );
        session
            .handle_incoming(r#"{"jsonrpc":"2.0","id":1,"result":"boo"}"#)
            .await;
    });
    let res = client.send_request("bar", Some(&["woo"])).await.unwrap();
    assert_eq!(res, "boo");
    Ok(())
}

#[tokio::test]
async fn basic_mixed_id_types() -> anyhow::Result<()> {
    struct Api {}

    #[rpc(all_positional, ts_outdir = "typescript/generated")]
    impl Api {
        pub async fn upper(&self, text: String) -> String {
            text.to_uppercase()
        }
    }

    let (session, mut out_rx) = RpcSession::create(Api {});

    let req = r#"{"jsonrpc":"2.0","method":"upper","params":["foo"],"id":"7"}"#;
    session.handle_incoming(req).await;
    let out = out_rx.next().await.unwrap();
    let out = serde_json::to_string(&out).unwrap();
    assert_eq!(out, r#"{"jsonrpc":"2.0","id":"7","result":"FOO"}"#);

    let req = r#"{"jsonrpc":"2.0","method":"upper","params":["foo"],"id":9}"#;
    session.handle_incoming(req).await;
    let out = out_rx.next().await.unwrap();
    let out = serde_json::to_string(&out).unwrap();
    assert_eq!(out, r#"{"jsonrpc":"2.0","id":9,"result":"FOO"}"#);

    let req = r#"{"jsonrpc":"2.0","method":"upper","params":["foo"],"id":"hi"}"#;
    session.handle_incoming(req).await;
    let out = out_rx.next().await.unwrap();
    let out = serde_json::to_string(&out).unwrap();
    assert_eq!(out, r#"{"jsonrpc":"2.0","id":"hi","result":"FOO"}"#);

    Ok(())
}
