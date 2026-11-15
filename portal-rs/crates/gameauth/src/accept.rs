// Accept failures (including descriptor exhaustion) must not silently remove a
// private surface. Retry with capped backoff; sustained failure completes the
// task so the process supervisor can fail the process.
pub async fn accept_with_retry<T, F, Fut, Report>(
    mut accept: F,
    mut report: Report,
) -> std::io::Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::io::Result<T>>,
    Report: FnMut(&std::io::Error),
{
    let mut delay = std::time::Duration::from_millis(25);
    for attempt in 0..8 {
        match accept().await {
            Ok(connection) => return Ok(connection),
            Err(error) => {
                report(&error);
                if attempt == 7 {
                    return Err(error);
                }
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(std::time::Duration::from_secs(1));
            }
        }
    }
    unreachable!("accept loop returns on its final attempt")
}

#[cfg(test)]
mod accept_tests {
    use super::accept_with_retry;

    #[tokio::test]
    async fn recoverable_accept_error_is_recorded_and_next_connection_is_served()
    -> Result<(), Box<dyn std::error::Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let client = tokio::net::TcpStream::connect(listener.local_addr()?).await?;
        let mut first = true;
        let mut recorded = Vec::new();
        let (mut socket, _) = accept_with_retry(
            || {
                let fail = std::mem::take(&mut first);
                let listener = &listener;
                async move {
                    if fail {
                        Err(std::io::Error::from(std::io::ErrorKind::ConnectionAborted))
                    } else {
                        listener.accept().await
                    }
                }
            },
            |error| recorded.push(error.kind()),
        )
        .await?;
        assert_eq!(recorded, [std::io::ErrorKind::ConnectionAborted]);
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        socket.write_all(b"served").await?;
        let mut bytes = [0; 6];
        let mut client = client;
        client.read_exact(&mut bytes).await?;
        assert_eq!(&bytes, b"served");
        Ok(())
    }

    #[tokio::test]
    async fn sustained_accept_failure_returns_to_supervisor() {
        let started = std::time::Instant::now();
        let mut recorded = 0;
        let result = accept_with_retry(
            || async { Err::<(), _>(std::io::Error::from(std::io::ErrorKind::PermissionDenied)) },
            |_| recorded += 1,
        )
        .await;
        assert!(result.is_err());
        assert_eq!(recorded, 8);
        assert!(started.elapsed() >= std::time::Duration::from_millis(2575));
    }
}
