use super::*;
use axum::http::{HeaderMap, HeaderValue};
use std::future::Future;

#[tokio::test]
async fn budget_exhaustion_replays_prefix_remainder_and_trailers()
-> Result<(), Box<dyn std::error::Error>> {
    let mut trailers = HeaderMap::new();
    trailers.insert("x-finished", HeaderValue::from_static("yes"));
    let frames = VecDeque::from([
        Ok(Frame::data(Bytes::from_static(b"first"))),
        Ok(Frame::data(Bytes::from_static(b"second"))),
        Ok(Frame::data(Bytes::from_static(b"last"))),
        Ok(Frame::trailers(trailers.clone())),
    ]);
    let body = Body::new(Replay {
        frames,
        remaining: Body::empty(),
    });
    let mut body = buffer(body, 6, None)
        .await
        .err()
        .ok_or("must pass through")?;
    for expected in ["first", "second", "last"] {
        let frame = poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
            .await
            .ok_or("missing frame")??;
        assert_eq!(frame.data_ref().ok_or("expected data")?, expected);
    }
    let frame = poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
        .await
        .ok_or("missing trailers")??;
    assert_eq!(frame.trailers_ref(), Some(&trailers));
    Ok(())
}

struct Delayed {
    ready: Pin<Box<tokio::time::Sleep>>,
    body: Body,
}
impl HttpBody for Delayed {
    type Data = Bytes;
    type Error = axum::Error;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, axum::Error>>> {
        if self.ready.as_mut().poll(cx).is_pending() {
            return Poll::Pending;
        }
        Pin::new(&mut self.body).poll_frame(cx)
    }
}

#[tokio::test]
async fn deadline_replays_collected_prefix_and_resumes_slow_remainder()
-> Result<(), Box<dyn std::error::Error>> {
    let body = Body::new(Replay {
        frames: VecDeque::from([Ok(Frame::data(Bytes::from_static(b"prefix")))]),
        remaining: Body::new(Delayed {
            ready: Box::pin(tokio::time::sleep(Duration::from_millis(30))),
            body: Body::from("tail"),
        }),
    });
    let body = buffer(body, 100, Some(Duration::from_millis(1)))
        .await
        .err()
        .ok_or("deadline must stop decoration")?;
    assert_eq!(axum::body::to_bytes(body, 100).await?, "prefixtail");
    Ok(())
}

#[tokio::test]
async fn exact_budget_collects_and_body_errors_are_replayed()
-> Result<(), Box<dyn std::error::Error>> {
    let bytes = buffer(Body::from("exact"), 5, None)
        .await
        .map_err(|_| "exact limit must fit")?;
    assert_eq!(bytes, "exact");
    let body = Body::new(Replay {
        frames: VecDeque::from([
            Ok(Frame::data(Bytes::from_static(b"prefix"))),
            Err(axum::Error::new(std::io::Error::other(
                "source interrupted",
            ))),
        ]),
        remaining: Body::empty(),
    });
    let mut body = buffer(body, 100, None)
        .await
        .err()
        .ok_or("errors must pass through")?;
    let frame = poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
        .await
        .ok_or("missing prefix")??;
    assert_eq!(frame.data_ref().ok_or("data expected")?, "prefix");
    let frame = poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
        .await
        .ok_or("missing error")?;
    assert!(frame.is_err());
    Ok(())
}
