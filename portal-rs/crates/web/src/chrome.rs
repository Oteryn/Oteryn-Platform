//! Read the existing browser session for presentation without adding a CSRF gate.
use crate::session::SessionLayer;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use oteryn_ui::Registry;

/// Startup state: enabled navigation and the existing browser-session configuration.
#[derive(Clone, Debug)]
pub struct Shell {
    pub registry: Registry,
    pub sessions: SessionLayer,
}
/// Install as the outermost application middleware so every module can extract Chrome.
/// Session validation and mutation authorization remain in their existing route layers.
pub async fn middleware(State(shell): State<Shell>, mut request: Request, next: Next) -> Response {
    let lang = crate::surface::path_language(request.uri().path())
        .unwrap_or_else(|| crate::surface::negotiate(request.headers()))
        .code();
    let account = crate::session::chrome_account(&shell.sessions, &mut request).await;
    let chrome = shell.registry.chrome(lang, request.uri().path(), account);
    request.extensions_mut().insert(chrome);
    next.run(request).await
}
