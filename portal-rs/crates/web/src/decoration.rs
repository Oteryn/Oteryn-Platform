//! Bounded, lossless buffering for optional HTML decoration.
use axum::body::{Body, Bytes, HttpBody};
use http_body::Frame;
use std::{
    collections::VecDeque,
    future::poll_fn,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

struct Replay {
    frames: VecDeque<Result<Frame<Bytes>, axum::Error>>,
    remaining: Body,
}
impl HttpBody for Replay {
    type Data = Bytes;
    type Error = axum::Error;
    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, axum::Error>>> {
        if let Some(frame) = self.frames.pop_front() {
            Poll::Ready(Some(frame))
        } else {
            Pin::new(&mut self.remaining).poll_frame(cx)
        }
    }
}

/// Collect within the decoration budget, or replay every consumed frame followed
/// by the untouched remainder. A deadline stops decoration, never delivery.
/// Trailers and body errors also require passthrough rather than text conversion.
pub async fn buffer(body: Body, limit: usize, timeout: Option<Duration>) -> Result<Bytes, Body> {
    let mut replay = Replay {
        frames: VecDeque::new(),
        remaining: body,
    };
    let deadline = timeout.map(|duration| tokio::time::Instant::now() + duration);
    let mut length = 0usize;
    loop {
        let frame = poll_fn(|cx| Pin::new(&mut replay.remaining).poll_frame(cx));
        let frame = if let Some(deadline) = deadline {
            match tokio::time::timeout_at(deadline, frame).await {
                Ok(frame) => frame,
                Err(_) => return Err(Body::new(replay)),
            }
        } else {
            frame.await
        };
        let Some(frame) = frame else { break };
        let passthrough = match &frame {
            Ok(frame) => match frame.data_ref() {
                Some(data) => {
                    length = length.saturating_add(data.len());
                    length > limit
                }
                None => true,
            },
            Err(_) => true,
        };
        replay.frames.push_back(frame);
        if passthrough {
            return Err(Body::new(replay));
        }
    }
    let mut bytes = Vec::with_capacity(length);
    for frame in replay.frames {
        if let Ok(frame) = frame
            && let Ok(data) = frame.into_data()
        {
            bytes.extend_from_slice(&data);
        }
    }
    Ok(Bytes::from(bytes))
}

#[cfg(test)]
mod tests;
