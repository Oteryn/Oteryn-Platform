use super::*;
use oteryn_gameauth::devices;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Https {
    address: std::net::SocketAddr,
    client: tokio_rustls::TlsConnector,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Https {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl Https {
    async fn new(app: Router) -> Result<Self, Box<dyn std::error::Error>> {
        let rcgen::CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let server = rustls::ServerConfig::builder_with_provider(provider.clone())
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_no_client_auth()
            .with_single_cert(
                vec![cert.der().clone()],
                rustls::pki_types::PrivateKeyDer::Pkcs8(key_pair.serialize_der().into()),
            )?;
        let mut roots = rustls::RootCertStore::empty();
        roots.add(cert.der().clone())?;
        let client = rustls::ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])?
            .with_root_certificates(roots)
            .with_no_client_auth();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let task = tokio::spawn(devices::tls::serve(listener, server, app));
        Ok(Self {
            address,
            client: tokio_rustls::TlsConnector::from(Arc::new(client)),
            task,
        })
    }
    async fn post(
        &self,
        suffix: &str,
        scheme: &str,
        credential: &str,
        body: serde_json::Value,
    ) -> Result<(u16, serde_json::Value), Box<dyn std::error::Error>> {
        let socket = tokio::net::TcpStream::connect(self.address).await?;
        let mut stream = self
            .client
            .connect(
                rustls::pki_types::ServerName::try_from("localhost")?,
                socket,
            )
            .await?;
        let body = body.to_string();
        let request = format!(
            "POST /api/v1/game-auth/device-sessions{suffix} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nAuthorization: {scheme} {credential}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).await?;
        let mut response = Vec::new();
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            stream.read_to_end(&mut response),
        )
        .await??;
        let text = String::from_utf8(response)?;
        let (headers, body) = text.split_once("\r\n\r\n").ok_or("missing HTTP body")?;
        let status = headers
            .split_whitespace()
            .nth(1)
            .ok_or("missing HTTP status")?
            .parse()?;
        Ok((status, serde_json::from_str(body)?))
    }
}
async fn sanctioned_device_is_refused(ban: bool) -> TestResult {
    let db = Database::open()
        .await?
        .ok_or("PORTAL_TEST_DATABASE_URL required")?;
    let account = db.account().await?;
    db.publish(account).await?;
    let access = db.access_token(account).await?;
    let settings = devices::Settings {
        enabled: true,
        environment: "testing".into(),
        ..Default::default()
    };
    let sessions = devices::Sessions::new(
        db.pool.clone(),
        settings,
        Arc::new(devices::PlatformAuthority {
            oauth: db.config.clone(),
            projection_authority: AUTHORITY.into(),
        }),
    );
    // Exactly the application composition: the common GuardedTickets wrapper is
    // injected into the real device router and the production TLS transport.
    let app = devices::router(sessions).layer(axum::Extension(device_authority(
        db.config.clone(),
        AUTHORITY.into(),
    )));
    let https = Https::new(app).await?;
    let (status, enrolled) = https
        .post(
            "",
            "Bearer",
            &access,
            serde_json::json!({"protocol_version":1,"remember_device":true}),
        )
        .await?;
    assert_eq!(status, 200);
    let credential = enrolled["device_credential"]
        .as_str()
        .ok_or("missing enrolled credential")?
        .to_owned();
    let operation = serde_json::json!({"protocol_version":1,"client_id":db.client});
    // Establish both actor-to-result paths before applying an account-level sanction.
    let (status, ticket) = https
        .post("/tickets", "OterynDevice", &credential, operation.clone())
        .await?;
    assert_eq!(status, 200);
    let credential = ticket["device_credential"]
        .as_str()
        .ok_or("missing rotated credential")?
        .to_owned();
    let (status, characters) = https
        .post(
            "/native-characters",
            "OterynDevice",
            &credential,
            operation.clone(),
        )
        .await?;
    assert_eq!(status, 200);
    let credential = characters["device_credential"]
        .as_str()
        .ok_or("missing rotated credential")?
        .to_owned();
    let before: (String, i64, i64) = sqlx::query_as(
        "SELECT status,game_auth_generation,native_security_generation FROM accounts WHERE id=$1",
    )
    .bind(account)
    .fetch_one(&db.pool)
    .await?;
    if ban {
        let sanction = Uuid::now_v7();
        sqlx::query("INSERT INTO support_sanction_identities(sanction_id,account_id,kind) VALUES($1,$2,'ban')").bind(sanction).bind(account).execute(&db.pool).await?;
        sqlx::query("INSERT INTO support_sanction_revisions(account_id,revision,sanction_id,intent,active,decision_at) VALUES($1,1,$2,jsonb_build_object('operation','apply','kind','ban','public_reason','device regression'),true,clock_timestamp())")
            .bind(account).bind(sanction).execute(&db.pool).await?;
    } else {
        db.suspend(account).await?;
    }
    let after: (String, i64, i64) = sqlx::query_as(
        "SELECT status,game_auth_generation,native_security_generation FROM accounts WHERE id=$1",
    )
    .bind(account)
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(
        before, after,
        "sanction must leave status and generations unchanged"
    );
    let oauth_bearer = db.access_token(account).await?;
    let (oauth_status, _) = super::ticket(db.player(), &oauth_bearer).await?;
    assert_eq!(oauth_status, StatusCode::UNAUTHORIZED);
    let tickets_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_login_tickets WHERE account_id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await?;
    let credentials_before: i64 = sqlx::query_scalar("SELECT count(*) FROM device_session_credentials c JOIN device_sessions f ON f.id=c.family_id WHERE f.account_id=$1")
        .bind(account).fetch_one(&db.pool).await?;
    for suffix in ["/tickets", "/native-characters"] {
        let (status, body) = https
            .post(suffix, "OterynDevice", &credential, operation.clone())
            .await?;
        assert_eq!(status, 401);
        assert_eq!(
            body,
            serde_json::json!({"error":"device_authorization_unavailable"})
        );
    }
    let families_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM device_sessions WHERE account_id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await?;
    let enrollment_bearer = db.access_token(account).await?;
    let (status, body) = https
        .post(
            "",
            "Bearer",
            &enrollment_bearer,
            serde_json::json!({"protocol_version":1,"remember_device":true}),
        )
        .await?;
    assert_eq!(status, 401, "fresh enrollment must enforce sanctions");
    assert_eq!(
        body,
        serde_json::json!({"error":"device_authorization_unavailable"})
    );
    let families_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM device_sessions WHERE account_id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(families_before, families_after);
    let tickets_after: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_login_tickets WHERE account_id=$1")
            .bind(account)
            .fetch_one(&db.pool)
            .await?;
    assert_eq!(tickets_before, tickets_after);
    let credentials_after: i64 = sqlx::query_scalar("SELECT count(*) FROM device_session_credentials c JOIN device_sessions f ON f.id=c.family_id WHERE f.account_id=$1")
        .bind(account).fetch_one(&db.pool).await?;
    assert_eq!(
        credentials_before, credentials_after,
        "no successor credential"
    );
    let unused: i64 = sqlx::query_scalar("SELECT count(*) FROM device_session_credentials c JOIN device_sessions f ON f.id=c.family_id WHERE f.account_id=$1 AND c.consumed_at IS NULL AND f.revoked_at IS NULL")
        .bind(account).fetch_one(&db.pool).await?;
    assert_eq!(unused, 1, "sanctions refusal rolls back device rotation");
    let (status, body) = https
        .post("/revoke", "OterynDevice", &credential, operation)
        .await?;
    assert_eq!(status, 200, "sanctioned accounts can still revoke devices");
    assert_eq!(
        body,
        serde_json::json!({"protocol_version":1,"revoked":true})
    );
    drop(https);
    db.close().await
}
#[tokio::test]
async fn audit05_03_device_routes_refuse_effective_suspension() -> TestResult {
    let started = std::time::Instant::now();
    let result = sanctioned_device_is_refused(false).await;
    eprintln!(
        "audit05_03_device_routes_refuse_effective_suspension: {:?}",
        started.elapsed()
    );
    result
}
#[tokio::test]
async fn audit05_03_device_routes_refuse_effective_ban() -> TestResult {
    let started = std::time::Instant::now();
    let result = sanctioned_device_is_refused(true).await;
    eprintln!(
        "audit05_03_device_routes_refuse_effective_ban: {:?}",
        started.elapsed()
    );
    result
}
