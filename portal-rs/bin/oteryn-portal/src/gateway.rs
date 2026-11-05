//! Gateway role starts without loading portal configuration or a database pool.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = oteryn_gateway::config::Config::from_env()?;
    let token = config.read_token()?;
    let issuer = oteryn_gateway::issuer::IssuerClient::new(
        &config.base_url,
        token,
        config.timeout,
        config.ready_timeout,
    )?;
    let app = oteryn_gateway::router(issuer, config.enabled, config.version);
    let listener = tokio::net::TcpListener::bind(&config.listen).await?;
    tracing::info!(
        bind = config.listen,
        native_login_enabled = config.enabled,
        "gateway serving"
    );
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
