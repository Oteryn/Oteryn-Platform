use super::*;
use std::sync::{Arc, Mutex};
use tracing::{Event, Subscriber, field::Visit, span};

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<String>>);
impl Visit for Capture {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if let Ok(mut output) = self.0.lock() {
            output.push_str(&format!("{}={value:?}\n", field.name()));
        }
    }
}
impl Subscriber for Capture {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, attrs: &span::Attributes<'_>) -> span::Id {
        attrs.record(&mut self.clone());
        span::Id::from_u64(1)
    }
    fn record(&self, _: &span::Id, values: &span::Record<'_>) {
        values.record(&mut self.clone());
    }
    fn record_follows_from(&self, _: &span::Id, _: &span::Id) {}
    fn event(&self, event: &Event<'_>) {
        event.record(&mut self.clone());
    }
    fn enter(&self, _: &span::Id) {}
    fn exit(&self, _: &span::Id) {}
}

#[tokio::test(flavor = "current_thread")]
async fn credential_aliases_never_enter_debug_traces() -> Result<(), Box<dyn std::error::Error>> {
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    let capture = Capture::default();
    let _guard = tracing::subscriber::set_default(capture.clone());
    let db = oteryn_db::lazy_pool("postgres://portal@127.0.0.1:1/unreachable")?;
    let app = router(AppState { db }, "static");
    for prefix in [
        "/reset-password/",
        "/email-change/recover/",
        "/email-change/confirm/",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "{prefix}L5CredentialSentinel?token=L5QuerySentinel"
                ))
                .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    }
    // Exercise matched templates as well as the binary router's unmatched fallback.
    let aliases =
        redirects::router().layer(TraceLayer::new_for_http().make_span_with(request_span));
    for prefix in [
        "/reset-password/",
        "/email-change/recover/",
        "/email-change/confirm/",
    ] {
        let response = aliases
            .clone()
            .oneshot(
                Request::get(format!(
                    "{prefix}L5CredentialSentinel?token=L5QuerySentinel"
                ))
                .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::MOVED_PERMANENTLY);
    }
    let output = capture.0.lock().map_err(|_| "capture poisoned")?;
    assert!(output.contains("method="), "request tracing must execute");
    assert!(output.contains("/reset-password/{token}"));
    assert!(output.contains("/email-change/recover/{token}"));
    assert!(output.contains("/email-change/confirm/{token}"));
    assert!(!output.contains("L5CredentialSentinel"));
    assert!(!output.contains("L5QuerySentinel"));
    Ok(())
}
