use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use oteryn_gateway::{Options, issuer::IssuerClient, router_with_proxies};
use std::{net::SocketAddr, time::Duration};
use tower::ServiceExt;

#[tokio::test]
async fn all_auth_routes_separate_forwarded_clients_only_for_trusted_peers()
-> Result<(), Box<dyn std::error::Error>> {
    for path in ["/v1/login", "/v1/recover", "/v1/recover/revoke"] {
        for trusted in [true, false] {
            let proxy: SocketAddr = "192.0.2.10:1234".parse()?;
            let app = router_with_proxies(
                IssuerClient::new(
                    "http://127.0.0.1:1",
                    "synthetic".into(),
                    Duration::from_secs(1),
                    Duration::from_secs(1),
                )?,
                Options {
                    enabled: true,
                    protocol3: true,
                    version: "test".into(),
                },
                if trusted {
                    vec![proxy.ip()]
                } else {
                    Vec::new()
                },
            );
            for i in 0..32 {
                let body = if path == "/v1/login" {
                    let mut body: serde_json::Value = serde_json::from_slice(include_bytes!(
                        "fixtures/game_client_native_login/request.json"
                    ))?;
                    body["attempt_ref"] = format!("0192b3c4-5d6e-7f80-9a1b-{i:012x}").into();
                    serde_json::to_vec(&body)?
                } else {
                    b"{}".to_vec()
                };
                let forwarded = if i < 31 {
                    "198.51.100.1"
                } else {
                    "198.51.100.2"
                };
                let response = app
                    .clone()
                    .oneshot(
                        Request::post(path)
                            .header("x-forwarded-for", forwarded)
                            .extension(ConnectInfo(proxy))
                            .body(Body::from(body))?,
                    )
                    .await?;
                assert_eq!(
                    response.status() == StatusCode::TOO_MANY_REQUESTS,
                    i == 30 || (i == 31 && !trusted),
                    "{path} trusted={trusted} request={i}"
                );
            }
        }
    }
    Ok(())
}
