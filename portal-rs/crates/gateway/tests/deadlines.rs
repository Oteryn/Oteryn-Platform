use oteryn_gateway::{BODY_DEADLINE, Options, SHUTDOWN_DEADLINE, issuer::IssuerClient};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[tokio::test]
async fn incomplete_socket_bodies_expire_and_shutdown_drains_within_bound() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let issuer = IssuerClient::new(
        "http://127.0.0.1:1",
        "synthetic-test-service-token".into(),
        Duration::from_secs(20),
        Duration::from_secs(20),
    )?;
    let app = oteryn_gateway::router_with(
        issuer,
        Options {
            enabled: true,
            protocol3: true,
            version: "test".into(),
        },
    );
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(oteryn_gateway::serve_until(listener, app, async {
        let _ = stopped.await;
    }));
    let mut clients = vec![];
    for path in ["/v1/login", "/v1/recover", "/v1/recover/revoke"] {
        let mut client = TcpStream::connect(address).await?;
        client
            .write_all(
                format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: 200\r\n\r\n{{")
                    .as_bytes(),
            )
            .await?;
        clients.push(client);
    }
    let started = std::time::Instant::now();
    for mut client in clients {
        let mut response = vec![];
        tokio::time::timeout(
            BODY_DEADLINE + Duration::from_secs(2),
            client.read_to_end(&mut response),
        )
        .await??;
        let response = String::from_utf8(response)?;
        let (headers, body) = response
            .split_once("\r\n\r\n")
            .ok_or("missing HTTP response header terminator")?;
        let mut headers = headers.split("\r\n");
        assert_eq!(headers.next(), Some("HTTP/1.1 408 Request Timeout"));
        assert!(headers.any(|line| line == "connection: close"));
        assert_eq!(body, "");
    }
    assert!(started.elapsed() < BODY_DEADLINE + Duration::from_secs(2));
    // An incomplete header never reaches router middleware; it must not hold shutdown.
    let mut headers = TcpStream::connect(address).await?;
    headers
        .write_all(b"POST /v1/login HTTP/1.1\r\nHost:")
        .await?;
    let _ = stop.send(());
    tokio::time::timeout(SHUTDOWN_DEADLINE + Duration::from_secs(1), server).await???;
    Ok(())
}

#[tokio::test]
async fn shutdown_aborts_and_closes_stalled_connections() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let entered = std::sync::Arc::new(tokio::sync::Notify::new());
    let notify = entered.clone();
    let app = axum::Router::new().route(
        "/",
        axum::routing::get(move || {
            let notify = notify.clone();
            async move {
                notify.notify_one();
                std::future::pending::<axum::http::StatusCode>().await
            }
        }),
    );
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(oteryn_gateway::serve_until(listener, app, async {
        let _ = stopped.await;
    }));
    let mut client = TcpStream::connect(address).await?;
    client
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .await?;
    tokio::time::timeout(Duration::from_secs(1), entered.notified()).await?;
    let _ = stop.send(());
    tokio::time::timeout(SHUTDOWN_DEADLINE + Duration::from_secs(1), server).await???;
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(1), client.read_to_end(&mut response)).await??;
    assert!(response.is_empty());
    Ok(())
}

#[tokio::test]
async fn full_request_deadline_bounds_a_stalled_upstream() -> TestResult {
    use tower::ServiceExt;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let upstream = tokio::spawn(async move {
        let (socket, _) = listener.accept().await?;
        let _socket = socket;
        std::future::pending::<std::io::Result<()>>().await
    });
    let issuer = IssuerClient::new(
        &format!("http://{address}"),
        "synthetic-test-service-token".into(),
        Duration::from_secs(20),
        Duration::from_secs(20),
    )?;
    let app = oteryn_gateway::router(issuer, true, "test".into());
    let result = tokio::time::timeout(
        oteryn_gateway::REQUEST_DEADLINE + Duration::from_secs(1),
        app.oneshot(
            axum::http::Request::builder()
                .uri("/ready")
                .body(axum::body::Body::empty())?,
        ),
    )
    .await;
    upstream.abort();
    assert_eq!(result??.status(), axum::http::StatusCode::REQUEST_TIMEOUT);
    Ok(())
}
